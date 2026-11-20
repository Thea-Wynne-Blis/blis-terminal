<script setup lang="ts">
import { convertFileSrc, invoke, isTauri } from '@tauri-apps/api/core'
import {
  computed,
  nextTick,
  onMounted,
  onUnmounted,
  ref,
  useTemplateRef,
} from 'vue'
import BackgroundPicker from '../features/appearance/BackgroundPicker.vue'
import DodgeGame from '../features/games/DodgeGame.vue'
import NoteEditor from '../features/notes/NoteEditor.vue'
import RemovalDialog from '../components/RemovalDialog.vue'
import RunnerGame from '../features/games/RunnerGame.vue'
import SnakeGame from '../features/games/SnakeGame.vue'
import TextFileEditor from '../features/files/TextFileEditor.vue'
import type {
  AliasOperation,
  Appearance,
  DesktopCommand,
  GameId,
  NoteDraft,
  TextDraft,
} from './command-types'
import {
  executeAlias,
  executeAppearance,
  executeDirectory,
  executeNote,
  executeWelcome,
  runDesktopCommand,
} from '../platform/desktop-platform'
import { formatDirectoryListing } from '../features/files/directory-command'
import {
  cyclePathCompletion,
  pathContext,
  startPathCompletion,
  type PathCompletion,
} from '../features/files/path-completion'
import {
  completeCommand,
  executeTerminalInput,
  type CommandCompletion,
} from './terminal-commands'
import {
  getHistoryStep,
  getOutputShortcut,
  isCommandCompletionKey,
  isCompositionKey,
  isEscapeKey,
} from './terminal-keyboard'
import { defaultWelcome } from '../features/appearance/welcome-command'

type TranscriptEntry = {
  id: number
  command: string
  directory: string
  output: string
  error: boolean
  collapsed: boolean
  launching: boolean
}

const desktop = isTauri()
const input = useTemplateRef<HTMLInputElement>('terminal-input')
const screen = useTemplateRef<HTMLElement>('terminal-screen')
const line = ref('')
const workingDirectory = ref('')
const showWelcome = ref(true)
const welcomeMessage = ref(defaultWelcome)
const gameViews = { snake: SnakeGame, runner: RunnerGame, dodge: DodgeGame }
const activeGame = ref<{ name: GameId; entryId: number }>()
const history: string[] = []
const transcript = ref<TranscriptEntry[]>([])
const shortcutStatus = ref('')
const appearance = ref<Appearance>()
const pickerOpen = ref(false)
const pickerBusy = ref(false)
const pickerError = ref('')
const activeNote = ref<NoteDraft>()
const activeFile = ref<{ draft: TextDraft; entryId: number }>()
type PendingRemoval = { entryId: number } & (
  { kind: 'note'; title: string } | { kind: 'file'; id: number; path: string }
)
const pendingRemoval = ref<PendingRemoval>()
const modalOpen = computed(() =>
  Boolean(activeNote.value || pendingRemoval.value || activeFile.value),
)
const selectedTheme = computed(() =>
  appearance.value?.themes.find(({ id }) => id === appearance.value?.selected),
)
const wallpaper = computed(() =>
  selectedTheme.value?.imagePath
    ? convertFileSrc(selectedTheme.value.imagePath)
    : undefined,
)
const backdropStyle = computed(() => ({
  backgroundColor: selectedTheme.value?.color ?? '#101216',
  opacity: (appearance.value?.opacity ?? 100) / 100,
}))
const maxHistory = 100
const maxTranscript = 200
const maxOutputLength = 8192
const pendingOutput: Record<DesktopCommand['type'], string> = {
  alias: '正在处理命令别名…',
  directory: '正在处理目录命令…',
  note: '正在处理笔记…',
  ping: '正在等待 Ping 结果…',
  timer: '正在设置倒计时…',
  todo: '正在处理待办…',
  search: '正在处理搜索请求…',
  'open-target': '正在处理打开清单…',
  background: '正在读取背景设置…',
  welcome: '正在处理欢迎语…',
  exit: '正在退出 Blis…',
}
let nextEntry = 0
let historyPosition: number | undefined
let draft = ''
let completion: CommandCompletion | undefined
let pathCompletion: PathCompletion | undefined
let pathRequest = 0
let aliases = new Map<string, string>()
let commandQueue = Promise.resolve()

function promptFor(directory: string) {
  return directory ? 'blis ' + directory + '>' : 'blis>'
}

function scrollToBottom() {
  if (screen.value) screen.value.scrollTop = screen.value.scrollHeight
}

function focusInput() {
  if (!activeGame.value) input.value?.focus({ preventScroll: true })
}

function focusOnBlankClick(event: MouseEvent) {
  if (!modalOpen.value && !pickerOpen.value && event.target === screen.value)
    focusInput()
}

function handleWindowFocus() {
  if (!modalOpen.value && !pickerOpen.value) focusInput()
}

function closeNote() {
  activeNote.value = undefined
  void nextTick(focusInput)
}

function closeFile(message: string) {
  if (!activeFile.value) return
  setCommandOutput(activeFile.value.entryId, message)
  activeFile.value = undefined
  void nextTick(focusInput)
}

function finishRemoval(confirmed: boolean) {
  const removal = pendingRemoval.value
  if (!removal) return
  pendingRemoval.value = undefined
  void nextTick(focusInput)
  if (!confirmed && removal.kind === 'note') {
    setCommandOutput(
      removal.entryId,
      '已取消删除笔记「' + removal.title + '」。',
    )
    return
  }

  if (confirmed)
    setCommandOutput(
      removal.entryId,
      removal.kind === 'file' ? '正在送入回收站…' : '正在删除笔记…',
    )
  commandQueue = commandQueue.then(async () => {
    try {
      if (removal.kind === 'file') {
        const outcome = await executeDirectory({
          action: 'finish_remove',
          id: removal.id,
          confirmed,
        })
        setCommandOutput(removal.entryId, outcome.text)
      } else {
        const outcome = await executeNote({
          action: 'remove',
          title: removal.title,
          confirmed: true,
        })
        if (outcome.type !== 'message') throw new Error('无法确认笔记已删除。')
        setCommandOutput(removal.entryId, outcome.text)
      }
    } catch (error) {
      setCommandOutput(removal.entryId, errorMessage(error), true)
    }
  })
}

function closePicker() {
  pickerOpen.value = false
  void nextTick(focusInput)
}

async function chooseBackground(name: string) {
  if (pickerBusy.value) return
  pickerBusy.value = true
  pickerError.value = ''
  try {
    appearance.value = await executeAppearance({ action: 'select', name })
    closePicker()
  } catch (error) {
    pickerError.value = errorMessage(error)
  } finally {
    pickerBusy.value = false
  }
}

function errorMessage(error: unknown): string {
  if (typeof error === 'string') return error
  return error instanceof Error ? error.message : '执行桌面操作失败。'
}

function clearTranscript() {
  showWelcome.value = false
  transcript.value = []
}

function toggleOutputs() {
  const entries = transcript.value.filter((entry) => entry.output)
  const collapse = entries.some((entry) => !entry.collapsed)
  for (const entry of entries) entry.collapsed = collapse
}

function limitOutput(output: string): string {
  return output.length > maxOutputLength
    ? `${output.slice(0, maxOutputLength)}\n…`
    : output
}

function submitCommand() {
  const command = line.value
  if (!command.trim()) return

  line.value = ''
  historyPosition = undefined
  const previousHistory = [...history]
  history.push(command)
  if (history.length > maxHistory) history.shift()

  // Read aliases and the current directory in the order commands were submitted.
  commandQueue = commandQueue.then(() =>
    dispatchCommand(command, previousHistory),
  )
}

function dispatchCommand(command: string, previousHistory: readonly string[]) {
  const parsed = executeTerminalInput(command, previousHistory, aliases)
  if (activeGame.value) {
    if (parsed.type === 'game' && parsed.game === activeGame.value.name)
      void nextTick(scrollToBottom)
    return
  }
  const result =
    parsed.type === 'game' && (modalOpen.value || pickerOpen.value)
      ? { type: 'error' as const, text: '请先完成当前操作，再开始游戏。' }
      : parsed
  if (result.type === 'clear') {
    clearTranscript()
    return
  }
  const id = nextEntry++
  const desktopCommand = 'operation' in result || result.type === 'exit'
  const launchAlias =
    result.type === 'open-target' && result.operation.action === 'launch'
      ? result.operation.alias
      : undefined
  let output = desktopCommand
    ? pendingOutput[result.type]
    : result.type === 'game'
      ? ''
      : result.text
  if (launchAlias) output = '正在打开「' + launchAlias + '」…'
  transcript.value.push({
    id,
    command,
    directory: workingDirectory.value,
    output: limitOutput(output),
    error: result.type === 'error',
    collapsed: false,
    launching: launchAlias !== undefined,
  })
  if (transcript.value.length > maxTranscript) transcript.value.shift()
  if (result.type === 'game') {
    activeGame.value = { name: result.game, entryId: id }
    resetCompletion()
  }
  void nextTick(scrollToBottom)
  if (desktopCommand) {
    const request = completeDesktopCommand(id, result)
    if (
      result.type === 'alias' ||
      result.type === 'background' ||
      result.type === 'directory' ||
      result.type === 'note' ||
      result.type === 'todo' ||
      result.type === 'welcome'
    )
      return request
  }
}

function finishGame(summary: string) {
  const game = activeGame.value
  if (!game) return
  activeGame.value = undefined
  setCommandOutput(game.entryId, summary)
  void nextTick(() => {
    focusInput()
    scrollToBottom()
  })
}

async function refreshAliases(operation: AliasOperation) {
  aliases = new Map(Object.entries(await executeAlias(operation)))
  resetCompletion()
}

async function completeAliasCommand(operation: AliasOperation) {
  await refreshAliases(operation)
  switch (operation.action) {
    case 'add':
      return '已保存别名「' + operation.name + '」。'
    case 'remove':
      return '已移除别名「' + operation.name + '」。'
    case 'list':
      return (
        [...aliases]
          .map(([name, target]) => name + '  →  ' + target)
          .join('\n') || '暂无命令别名。输入 help alias 查看添加方法。'
      )
  }
}

async function completeDesktopCommand(id: number, command: DesktopCommand) {
  let output: string
  let failed = false
  try {
    if (command.type === 'alias') {
      output = await completeAliasCommand(command.operation)
    } else if (command.type === 'directory') {
      if (
        (command.operation.action === 'remove' ||
          command.operation.action === 'edit_text') &&
        (modalOpen.value || pickerOpen.value)
      )
        throw new Error('请先完成当前操作。')
      const outcome = await executeDirectory(command.operation)
      workingDirectory.value = outcome.directory
      if (outcome.removal)
        pendingRemoval.value = { entryId: id, kind: 'file', ...outcome.removal }
      if (outcome.editor)
        activeFile.value = { entryId: id, draft: outcome.editor }
      output = outcome.listing
        ? formatDirectoryListing(outcome.listing)
        : outcome.text
    } else if (command.type === 'note') {
      const operation = command.operation
      if (
        modalOpen.value &&
        (operation.action === 'new' ||
          operation.action === 'edit' ||
          operation.action === 'remove')
      )
        throw new Error('请先关闭当前笔记。')
      const outcome = await executeNote(operation)
      if (outcome.type === 'editor') {
        activeNote.value = outcome
        output = ''
      } else if (outcome.type === 'confirm_remove') {
        pendingRemoval.value = {
          entryId: id,
          kind: 'note',
          title: outcome.title,
        }
        output = '等待确认删除笔记「' + outcome.title + '」。'
      } else {
        output = outcome.text
      }
    } else if (command.type === 'welcome') {
      welcomeMessage.value =
        (await executeWelcome(command.operation)) ?? defaultWelcome
      showWelcome.value = true
      output =
        command.operation.action === 'get'
          ? welcomeMessage.value
          : '欢迎语已更新为「' + welcomeMessage.value + '」。'
    } else if (command.type === 'background') {
      const operation = command.operation
      appearance.value = await executeAppearance(operation)
      switch (operation.action) {
        case 'get':
          pickerError.value = ''
          pickerOpen.value = true
          output = ''
          break
        case 'opacity':
          output = `背景不透明度已设置为 ${appearance.value.opacity}%。`
          break
        case 'reset':
          output = '已恢复深色背景和 100% 不透明度。'
          break
        case 'remove':
          output = `已移除背景「${operation.name}」。`
          break
        case 'select':
          output = `已切换背景为「${operation.name}」。`
          break
        default:
          output = `已添加背景「${operation.name}」。输入 background 可选择。`
      }
    } else {
      output = await runDesktopCommand(command)
    }
  } catch (error) {
    output = errorMessage(error)
    failed = true
  }

  setCommandOutput(id, output, failed)
}

function setCommandOutput(id: number, output: string, failed = false) {
  const entry = transcript.value.find((entry) => entry.id === id)
  if (entry) {
    const panel = screen.value
    const followingOutput =
      panel && panel.scrollHeight - panel.clientHeight - panel.scrollTop <= 2
    entry.output = limitOutput(output)
    entry.error = failed
    entry.launching = false
    if (followingOutput) void nextTick(scrollToBottom)
  }
}

function resetCompletion() {
  completion = undefined
  pathCompletion = undefined
  pathRequest++
}

function applyCompletion(
  field: HTMLInputElement,
  completed: { value: string; caret: number },
) {
  line.value = completed.value
  void nextTick(() => field.setSelectionRange(completed.caret, completed.caret))
}

function completeInput(event: KeyboardEvent) {
  const field = input.value
  if (
    field?.selectionStart == null ||
    field.selectionStart !== field.selectionEnd
  ) {
    resetCompletion()
    return
  }
  const value = line.value
  const caret = field.selectionStart
  const cycled = cyclePathCompletion(pathCompletion, value, caret)
  if (cycled) {
    event.preventDefault()
    pathCompletion = cycled
    applyCompletion(field, cycled)
    return
  }
  completion = completeCommand(value, caret, completion, aliases)
  if (completion) {
    event.preventDefault()
    applyCompletion(field, completion)
    return
  }
  const context = desktop ? pathContext(value, caret) : undefined
  if (!context) return
  event.preventDefault()
  const request = ++pathRequest
  void commandQueue
    .then(() =>
      executeDirectory({
        action: 'complete_path',
        prefix: context.prefix,
        directories_only: context.directoriesOnly,
        all: context.all,
      }),
    )
    .then((outcome) => {
      if (
        request !== pathRequest ||
        line.value !== value ||
        field.selectionStart !== caret ||
        field.selectionEnd !== caret
      )
        return
      pathCompletion = startPathCompletion(context, outcome.candidates ?? [])
      if (pathCompletion) applyCompletion(field, pathCompletion)
    })
    .catch((error) => console.error('Failed to complete path', error))
}

function handleInputKeydown(event: KeyboardEvent) {
  if (
    activeGame.value ||
    modalOpen.value ||
    pickerOpen.value ||
    isCompositionKey(event)
  )
    return
  if (isCommandCompletionKey(event)) {
    completeInput(event)
    return
  }
  resetCompletion()
  if (event.key === 'Enter' && !event.repeat) {
    event.preventDefault()
    submitCommand()
    return
  }

  const step = getHistoryStep(event)
  if (!step || !history.length) return

  if (historyPosition === undefined) {
    if (step === 1) return
    draft = line.value
    historyPosition = history.length
  }
  event.preventDefault()
  historyPosition = Math.max(0, historyPosition + step)
  if (historyPosition === history.length) {
    line.value = draft
    historyPosition = undefined
  } else {
    line.value = history[historyPosition] ?? ''
  }
  void nextTick(() =>
    input.value?.setSelectionRange(line.value.length, line.value.length),
  )
}

function handleKeydown(event: KeyboardEvent) {
  if (activeGame.value || modalOpen.value) return
  if (pickerOpen.value) {
    if (isEscapeKey(event)) {
      event.preventDefault()
      closePicker()
    }
    return
  }
  const shortcut = getOutputShortcut(event)
  if (shortcut) {
    event.preventDefault()
    if (event.repeat) return
    if (shortcut === 'clear') {
      clearTranscript()
    } else {
      toggleOutputs()
    }
    focusInput()
  }
}

onMounted(async () => {
  focusInput()
  window.addEventListener('focus', handleWindowFocus)
  window.addEventListener('keydown', handleKeydown)
  if (!desktop) return

  commandQueue = Promise.all([
    executeWelcome({ action: 'get' })
      .then((message) => {
        welcomeMessage.value = message ?? defaultWelcome
      })
      .catch((error) => console.error('Failed to load welcome message', error)),
    refreshAliases({ action: 'list' }).catch((error) =>
      console.error('Failed to load aliases', error),
    ),
    executeDirectory({ action: 'pwd' })
      .then((outcome) => {
        workingDirectory.value = outcome.directory
      })
      .catch((error) =>
        console.error('Failed to read current directory', error),
      ),
  ]).then(() => undefined)

  void executeAppearance({ action: 'get' })
    .then((settings) => {
      appearance.value = settings
    })
    .catch((error) => console.error('Failed to load background', error))

  try {
    const shortcut = await invoke<string | null>('active_shortcut')
    const keys = shortcut?.replaceAll('+', ' + ')
    shortcutStatus.value = keys
      ? `显示或隐藏 ${keys}`
      : '快捷键不可用，请从托盘唤醒。'
  } catch (error) {
    shortcutStatus.value = '读取快捷键失败，请从托盘唤醒。'
    console.error('Failed to read global shortcut', error)
  }
})

onUnmounted(() => {
  window.removeEventListener('focus', handleWindowFocus)
  window.removeEventListener('keydown', handleKeydown)
})
</script>

<template>
  <main
    ref="terminal-screen"
    class="terminal"
    :class="{ 'light-theme': selectedTheme?.light }"
    aria-label="Blis Terminal"
    @click="focusOnBlankClick"
  >
    <div
      class="terminal-backdrop"
      :class="{ 'has-wallpaper': wallpaper }"
      :style="backdropStyle"
      aria-hidden="true"
    >
      <img v-if="wallpaper" :src="wallpaper" alt="" />
    </div>
    <header v-if="showWelcome" class="welcome">
      <p>{{ welcomeMessage }}</p>
      <p class="muted">输入 <code>help</code> 查看命令。</p>
      <p v-if="shortcutStatus" class="muted" role="status">
        {{ shortcutStatus }}
      </p>
    </header>

    <section class="transcript" role="log" aria-label="命令输出">
      <div v-for="entry in transcript" :key="entry.id" class="entry">
        <p class="command">
          <span class="prompt-mark">{{ promptFor(entry.directory) }}</span>
          {{ entry.command }}
        </p>
        <component
          :is="gameViews[activeGame.name]"
          v-if="activeGame?.entryId === entry.id"
          :light="selectedTheme?.light ?? false"
          @finish="finishGame"
        />
        <pre
          v-if="entry.output && !entry.collapsed"
          :class="{ error: entry.error }"
          >{{ entry.output }}</pre>
        <progress
          v-if="entry.launching && !entry.collapsed"
          class="launch-progress"
          aria-label="正在打开"
        />
      </div>
    </section>

    <div v-if="!activeGame" class="prompt">
      <label
        class="prompt-mark"
        for="terminal-input"
        aria-hidden="true"
        :title="workingDirectory"
        >{{ promptFor(workingDirectory) }}</label
      >
      <input
        id="terminal-input"
        ref="terminal-input"
        v-model="line"
        type="text"
        aria-label="命令输入"
        autocomplete="off"
        autocapitalize="off"
        spellcheck="false"
        maxlength="4096"
        @input="resetCompletion"
        @blur="resetCompletion"
        @keydown="handleInputKeydown"
      />
    </div>
    <NoteEditor v-if="activeNote" :note="activeNote" @close="closeNote" />
    <TextFileEditor
      v-if="activeFile"
      :draft="activeFile.draft"
      @close="closeFile"
    />
    <RemovalDialog
      v-if="pendingRemoval"
      :title="pendingRemoval.kind === 'note' ? '删除笔记' : '放入回收站'"
      :description="
        pendingRemoval.kind === 'note'
          ? '确定永久删除笔记「' + pendingRemoval.title + '」吗？'
          : '将「' + pendingRemoval.path + '」送入回收站？'
      "
      :confirm-label="pendingRemoval.kind === 'note' ? '删除' : '回收'"
      @cancel="finishRemoval(false)"
      @confirm="finishRemoval(true)"
    />
    <BackgroundPicker
      v-if="pickerOpen && appearance"
      :themes="appearance.themes"
      :selected="appearance.selected"
      :busy="pickerBusy"
      :error="pickerError"
      @choose="chooseBackground"
      @close="closePicker"
    />
  </main>
</template>
