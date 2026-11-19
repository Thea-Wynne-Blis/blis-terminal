<script setup lang="ts">
import { computed, onMounted, ref, useTemplateRef } from 'vue'
import type { NoteDraft } from './command-types'
import { executeNote } from './desktop-platform'
import { isCompositionKey } from './terminal-keyboard'

const props = defineProps<{ note: NoteDraft }>()
const emit = defineEmits<{ close: [] }>()
const dialog = useTemplateRef<HTMLDialogElement>('note-dialog')
const editor = useTemplateRef<HTMLTextAreaElement>('note-content')
const content = ref(props.note.content)
const savedContent = ref(props.note.content)
const busy = ref(false)
const error = ref('')
const savedMessage = ref('')
const confirmClose = ref(false)
const dirty = computed(() => content.value !== savedContent.value)
let create = props.note.create

async function save() {
  if (busy.value) return
  if (confirmClose.value) confirmClose.value = false
  busy.value = true
  error.value = ''
  const text = content.value
  try {
    const outcome = await executeNote({
      action: 'save',
      title: props.note.title,
      content: text,
      create,
    })
    if (outcome.type !== 'message') throw new Error('无法确认笔记已保存。')
    create = false
    savedContent.value = text
    savedMessage.value = outcome.text
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : String(cause)
  } finally {
    busy.value = false
  }
}

function requestClose() {
  if (busy.value) return
  if (dirty.value) confirmClose.value = true
  else emit('close')
}

function keepEditing() {
  confirmClose.value = false
  editor.value?.focus()
}

function handleKeydown(event: KeyboardEvent) {
  if (
    event.key.toLowerCase() !== 's' ||
    !event.ctrlKey ||
    event.altKey ||
    event.metaKey ||
    event.shiftKey ||
    isCompositionKey(event)
  )
    return
  event.preventDefault()
  if (!event.repeat) void save()
}

onMounted(() => {
  dialog.value?.showModal()
  editor.value?.focus()
})
</script>

<template>
  <dialog
    ref="note-dialog"
    class="note-editor"
    aria-labelledby="note-title"
    @cancel.prevent="requestClose"
    @keydown="handleKeydown"
  >
    <h2 id="note-title">{{ note.title }}</h2>
    <textarea
      ref="note-content"
      v-model="content"
      aria-label="笔记正文"
      :readonly="busy"
      spellcheck="false"
    />
    <p v-if="error" class="error" role="alert">{{ error }}</p>
    <p v-else-if="busy || (savedMessage && !dirty)" class="muted" role="status">
      {{ busy ? '正在保存…' : savedMessage }}
    </p>
    <div v-if="confirmClose" class="note-actions">
      <p role="status">有未保存的修改。</p>
      <button type="button" @click="keepEditing">继续编辑</button>
      <button type="button" @click="emit('close')">放弃修改</button>
    </div>
    <div v-else class="note-actions">
      <button type="button" :disabled="busy" @click="requestClose">关闭</button>
      <button type="button" :disabled="busy" @click="save">保存 Ctrl+S</button>
    </div>
  </dialog>
</template>
