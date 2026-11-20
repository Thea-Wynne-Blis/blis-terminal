<script setup lang="ts">
import { computed, nextTick, onMounted, ref, useTemplateRef } from 'vue'
import { executeDirectory } from '../../platform/desktop-platform'
import type { TextDraft } from '../../terminal/command-types'
import { isCompositionKey } from '../../terminal/terminal-keyboard'

const props = defineProps<{ draft: TextDraft }>()
const emit = defineEmits<{ close: [message: string] }>()
const dialog = useTemplateRef<HTMLDialogElement>('dialog')
const editor = useTemplateRef<HTMLTextAreaElement>('editor')
const commandField = useTemplateRef<HTMLInputElement>('command')
const content = ref(props.draft.content)
const saved = ref(content.value)
const mode = ref<'normal' | 'insert' | 'command'>('normal')
const command = ref('')
const busy = ref(false)
const error = ref('')
const message = ref('')
const dirty = computed(() => content.value !== saved.value)

function normalMode() {
  mode.value = 'normal'
  editor.value?.focus()
}

function commandMode() {
  command.value = ''
  mode.value = 'command'
  void nextTick(() => commandField.value?.focus())
}

async function save(quit: boolean) {
  if (busy.value) return
  busy.value = true
  try {
    const text = content.value
    const result = await executeDirectory({
      action: 'save_text',
      id: props.draft.id,
      content: text,
    })
    saved.value = text
    message.value = result.text
    if (quit) emit('close', message.value)
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : String(cause)
  } finally {
    busy.value = false
  }
}

function executeEditorCommand(value: string) {
  if (busy.value) return
  error.value = ''
  normalMode()
  switch (value) {
    case 'w':
    case 'wq':
      void save(value === 'wq')
      break
    case 'q':
      if (dirty.value) {
        error.value = '有未保存的修改。用 :wq 保存并退出，或用 :q! 放弃修改。'
        break
      }
      emit('close', message.value)
      break
    case 'q!':
      emit('close', dirty.value ? '已放弃未保存的修改。' : message.value)
      break
    default:
      error.value = '可用编辑命令为 :w、:q、:wq、:q!。'
  }
}

function handleKeydown(event: KeyboardEvent) {
  if (isCompositionKey(event) || busy.value) return
  const { key, ctrlKey, altKey, metaKey } = event
  if (ctrlKey && !altKey && !metaKey && key.toLowerCase() === 's') {
    event.preventDefault()
    if (!event.repeat) executeEditorCommand('w')
    return
  }
  if (key === 'Escape') {
    event.preventDefault()
    normalMode()
    return
  }
  if (mode.value !== 'normal' || event.target !== editor.value) return
  if (ctrlKey || altKey || metaKey) return
  if (key === 'i') {
    event.preventDefault()
    insertMode()
  } else if (key === ':') {
    event.preventDefault()
    commandMode()
  }
}

function insertMode() {
  if (busy.value) return
  mode.value = 'insert'
  editor.value?.focus()
}

onMounted(() => {
  dialog.value?.showModal()
  editor.value?.focus()
})
</script>

<template>
  <dialog
    ref="dialog"
    class="note-editor vim-editor"
    aria-labelledby="file-editor-title"
    @cancel.prevent="normalMode"
    @keydown="handleKeydown"
  >
    <h2 id="file-editor-title">{{ draft.path }}</h2>
    <textarea
      ref="editor"
      v-model="content"
      aria-label="文件内容"
      :readonly="mode !== 'insert' || busy"
      spellcheck="false"
    />
    <form
      v-if="mode === 'command'"
      class="prompt"
      @submit.prevent="executeEditorCommand(command.trim())"
    >
      <label class="prompt-mark" for="file-editor-command">:</label>
      <input
        id="file-editor-command"
        ref="command"
        v-model="command"
        aria-label="编辑命令"
        autocomplete="off"
        spellcheck="false"
      />
    </form>
    <p v-if="error" class="error" role="alert">{{ error }}</p>
    <p v-else-if="busy || (message && !dirty)" role="status">
      {{ busy ? '正在保存…' : message }}
    </p>
    <footer class="vim-status">
      <span class="vim-mode">
        {{ { normal: '普通', insert: '插入', command: '命令' }[mode] }}
        {{ dirty ? '· 未保存' : '' }}
      </span>
      <span><kbd>i</kbd> 编辑</span>
      <span><kbd>Esc</kbd> 普通模式</span>
      <span><kbd>:w</kbd> 保存</span>
      <span><kbd>:wq</kbd> 保存退出</span>
      <span><kbd>:q</kbd> 退出</span>
      <span><kbd>:q!</kbd> 放弃退出</span>
    </footer>
  </dialog>
</template>

<style scoped>
.vim-editor {
  position: fixed;
  inset: 0;
  width: 100%;
  height: 100%;
  max-width: none;
  max-height: none;
  margin: 0;
  padding: 12px 16px 0;
  border: 0;
  border-radius: 0;
}

.vim-editor textarea {
  min-height: 0;
}

.vim-status {
  display: flex;
  flex-wrap: wrap;
  gap: 4px 14px;
  padding: 8px 0;
  border-top: 1px solid var(--menu-highlight);
  color: var(--muted);
}

.vim-mode,
.vim-status kbd {
  color: var(--accent);
  font: inherit;
}
</style>
