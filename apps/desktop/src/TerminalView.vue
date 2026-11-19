<script setup lang="ts">
import { convertFileSrc, invoke, isTauri } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import {
  computed,
  nextTick,
  onMounted,
  onUnmounted,
  ref,
  useTemplateRef,
} from 'vue'
import BackgroundPicker from './BackgroundPicker.vue'
import NoteEditor from './NoteEditor.vue'
import NoteRemovalDialog from './NoteRemovalDialog.vue'
import type {
  AliasOperation,
  Appearance,
  DesktopCommand,
  NoteDraft,
} from './command-types'
import {
  executeAlias,
  executeAppearance,
  executeNote,
  runDesktopCommand,
} from './desktop-platform'
import {
  completeCommand,
  executeTerminalInput,
  terminalVersion,
  type CommandCompletion,
} from './terminal-commands'
import {
  getHistoryStep,
  getOutputShortcut,
  isCommandCompletionKey,
  isCompositionKey,
  shouldHideTerminal,
} from './terminal-keyboard'

type TranscriptEntry = {
  id: number
  command: string
  output: string
  error: boolean
  collapsed: boolean
  launching: boolean
}

const desktop = isTauri()
const input = useTemplateRef<HTMLInputElement>('terminal-input')
const screen = useTemplateRef<HTMLElement>('terminal-screen')
const line = ref('')
const showWelcome = ref(true)
const history: string[] = []
const transcript = ref<TranscriptEntry[]>([])
const shortcutStatus = ref('')
const hideFailed = ref(false)
const appearance = ref<Appearance>()
const pickerOpen = ref(false)
const pickerBusy = ref(false)
const pickerError = ref('')
const activeNote = ref<NoteDraft>()
const pendingNoteRemoval = ref<{ id: number; title: string }>()
const noteDialogOpen = computed(() =>
  Boolean(activeNote.value || pendingNoteRemoval.value),
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
  note: '正在处理笔记…',
  search: '正在处理搜索请求…',
  'open-target': '正在处理打开清单…',
  background: '正在读取背景设置…',
  exit: '正在退出 Blis…',
}
let nextEntry = 0
let historyPosition: number | undefined
let draft = ''
let completion: CommandCompletion | undefined
let aliases = new Map<string, string>()
let aliasQueue = Promise.resolve()

function focusInput() {
  input.value?.focus({ preventScroll: true })
}

function focusOnBlankClick(event: MouseEvent) {
  if (
    !noteDialogOpen.value &&
    !pickerOpen.value &&
    event.target === screen.value
  )
    focusInput()
}

function handleWindowFocus() {
  if (!noteDialogOpen.value && !pickerOpen.value) focusInput()
}

function closeNote() {
  activeNote.value = undefined
  void nextTick(focusInput)
}

function finishNoteRemoval(confirmed: boolean) {
  const removal = pendingNoteRemoval.value
  if (!removal) return
  pendingNoteRemoval.value = undefined
  void nextTick(focusInput)
  if (!confirmed) {
    setCommandOutput(removal.id, '已取消删除笔记「' + removal.title + '」。')
    return
  }

  setCommandOutput(removal.id, '正在删除笔记…')
  aliasQueue = aliasQueue.then(async () => {
    try {
      const outcome = await executeNote({
        action: 'remove',
        title: removal.title,
        confirmed: true,
      })
      if (outcome.type !== 'message') throw new Error('无法确认笔记已删除。')
      setCommandOutput(removal.id, outcome.text)
    } catch (error) {
      setCommandOutput(removal.id, errorMessage(error), true)
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

  // Expand commands after any preceding alias edits have been saved.
  aliasQueue = aliasQueue.then(() => dispatchCommand(command, previousHistory))
}

function dispatchCommand(command: string, previousHistory: readonly string[]) {
  const result = executeTerminalInput(command, previousHistory, aliases)
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
  let output = desktopCommand ? pendingOutput[result.type] : result.text
  if (launchAlias) output = '正在打开「' + launchAlias + '」…'
  transcript.value.push({
    id,
    command,
    output: limitOutput(output),
    error: result.type === 'error',
    collapsed: false,
    launching: launchAlias !== undefined,
  })
  if (transcript.value.length > maxTranscript) transcript.value.shift()
  void nextTick(() => {
    if (screen.value) screen.value.scrollTop = screen.value.scrollHeight
  })
  if (desktopCommand) {
    const request = completeDesktopCommand(id, result)
    if (result.type === 'alias' || result.type === 'note') return request
  }
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
    } else if (command.type === 'note') {
      const operation = command.operation
      if (
        noteDialogOpen.value &&
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
        pendingNoteRemoval.value = { id, title: outcome.title }
        output = '等待确认删除笔记「' + outcome.title + '」。'
      } else {
        output = outcome.text
      }
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
    entry.output = limitOutput(output)
    entry.error = failed
    entry.launching = false
  }
}

function resetCompletion() {
  completion = undefined
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
  completion = completeCommand(
    line.value,
    field.selectionStart,
    completion,
    aliases,
  )
  if (!completion) return

  event.preventDefault()
  const { value, caret } = completion
  line.value = value
  void nextTick(() => field.setSelectionRange(caret, caret))
}

function handleInputKeydown(event: KeyboardEvent) {
  if (noteDialogOpen.value || pickerOpen.value || isCompositionKey(event))
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

async function hideTerminal() {
  hideFailed.value = false
  try {
    await getCurrentWindow().hide()
  } catch (error) {
    hideFailed.value = true
    console.error('Failed to hide terminal window', error)
  }
}

function handleKeydown(event: KeyboardEvent) {
  if (noteDialogOpen.value) return
  if (pickerOpen.value) {
    if (shouldHideTerminal(event)) {
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
    return
  }

  if (desktop && shouldHideTerminal(event)) {
    event.preventDefault()
    void hideTerminal()
  }
}

onMounted(async () => {
  focusInput()
  window.addEventListener('focus', handleWindowFocus)
  window.addEventListener('keydown', handleKeydown)
  if (!desktop) return

  aliasQueue = refreshAliases({ action: 'list' }).catch((error) =>
    console.error('Failed to load aliases', error),
  )

  void executeAppearance({ action: 'get' })
    .then((settings) => {
      appearance.value = settings
    })
    .catch((error) => console.error('Failed to load background', error))

  try {
    const shortcut = await invoke<string | null>('active_shortcut')
    const keys = shortcut?.replaceAll('+', ' + ')
    shortcutStatus.value = keys
      ? `唤醒 ${keys}    隐藏 Esc`
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
      <p>Blis Terminal {{ terminalVersion }}</p>
      <p class="muted">输入 <code>help</code> 查看命令。</p>
      <p v-if="shortcutStatus" class="muted" role="status">
        {{ shortcutStatus }}
      </p>
    </header>

    <section class="transcript" role="log" aria-label="命令输出">
      <div v-for="entry in transcript" :key="entry.id" class="entry">
        <p class="command">
          <span class="prompt-mark">blis&gt;</span> {{ entry.command }}
        </p>
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

    <div class="prompt">
      <label class="prompt-mark" for="terminal-input" aria-hidden="true"
        >blis&gt;</label
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
    <p v-if="hideFailed" class="error" role="alert">隐藏窗口失败，请重试。</p>
    <NoteEditor v-if="activeNote" :note="activeNote" @close="closeNote" />
    <NoteRemovalDialog
      v-if="pendingNoteRemoval"
      :title="pendingNoteRemoval.title"
      @cancel="finishNoteRemoval(false)"
      @confirm="finishNoteRemoval(true)"
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
