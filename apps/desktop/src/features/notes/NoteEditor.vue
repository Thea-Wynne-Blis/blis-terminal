<script setup lang="ts">
import {
  computed,
  defineAsyncComponent,
  nextTick,
  onMounted,
  ref,
  useTemplateRef,
  watch,
} from 'vue'
import type { NoteDraft } from '../../terminal/command-types'
import { executeNote } from '../../platform/desktop-platform'
import { isCompositionKey } from '../../terminal/terminal-keyboard'
import { readVisualMarkdown } from './note-source'
import { useNotePreview } from './use-note-preview'
import './note-styles.css'

const RichNoteEditor = defineAsyncComponent(
  () => import('./RichNoteEditor.vue'),
)
const views = { visual: '可视化', source: '源码', split: '分栏' } as const
type NoteView = keyof typeof views

const props = defineProps<{ note: NoteDraft }>()
const emit = defineEmits<{ close: [] }>()
const dialog = useTemplateRef<HTMLDialogElement>('note-dialog')
const editor = useTemplateRef<HTMLTextAreaElement>('note-content')
const workspace = useTemplateRef<HTMLElement>('note-workspace')
const previewBody = useTemplateRef<HTMLElement>('preview-body')
const visualEditor = useTemplateRef<{ readMarkdown(): string }>('visual-editor')
const content = ref(props.note.content)
const savedContent = ref(props.note.content)
const busy = ref(false)
const error = ref('')
const savedMessage = ref('')
const confirmClose = ref(false)
const visualEdited = ref(false)
const dirty = computed(
  () => visualEdited.value || content.value !== savedContent.value,
)
const markdown = props.note.format === 'md'
const supportsVisual =
  markdown && readVisualMarkdown(content.value) !== undefined
const view = ref<NoteView>(supportsVisual ? 'visual' : 'source')
const viewMessage = ref(
  markdown && !supportsVisual ? '这篇笔记含有 HTML 或脚注，已用源码打开。' : '',
)
const themeKey = 'blis.note.markdown-theme'
const themes = { light: '浅色', dark: '深色', paper: '纸张' } as const
type PreviewTheme = keyof typeof themes
const theme = ref<PreviewTheme>(initialTheme())
const { preview, previewError } = useNotePreview(
  content,
  computed(() => view.value === 'split'),
  previewBody,
)
const editorShare = ref(50)
let create = props.note.create

function initialTheme(): PreviewTheme {
  try {
    const stored = localStorage.getItem(themeKey)
    if (stored === 'light' || stored === 'dark' || stored === 'paper')
      return stored
  } catch {
    // The preview remains usable when storage is unavailable.
  }
  return 'light'
}

if (markdown) {
  watch(theme, (value) => {
    try {
      localStorage.setItem(themeKey, value)
    } catch {
      // Keep the selected theme for the current session.
    }
  })
}

function changeView(next: NoteView) {
  if (busy.value || view.value === next) return
  readVisualDraft()
  if (next === 'visual' && readVisualMarkdown(content.value) === undefined) {
    viewMessage.value = '这篇笔记含有 HTML 或脚注，请使用源码或分栏编辑。'
    return
  }
  viewMessage.value = ''
  view.value = next
  if (next !== 'visual') void nextTick(() => editor.value?.focus())
}

function readVisualDraft() {
  if (!visualEdited.value || !visualEditor.value) return
  content.value = visualEditor.value.readMarkdown()
  visualEdited.value = false
}

async function save() {
  if (busy.value) return
  readVisualDraft()
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
      format: props.note.format,
    })
    if (outcome.type !== 'saved') throw new Error('无法确认笔记已保存。')
    create = false
    content.value = outcome.content
    savedContent.value = outcome.content
    savedMessage.value = outcome.text
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : String(cause)
  } finally {
    busy.value = false
  }
}

function requestClose() {
  if (busy.value) return
  readVisualDraft()
  if (dirty.value) confirmClose.value = true
  else emit('close')
}

function keepEditing() {
  confirmClose.value = false
  editor.value?.focus()
}

function setEditorShare(share: number) {
  editorShare.value = Math.min(74, Math.max(26, Math.round(share)))
}

function dragDivider(event: PointerEvent) {
  const divider = event.currentTarget as HTMLElement
  if (event.type === 'pointerdown' && event.button === 0)
    divider.setPointerCapture(event.pointerId)
  if (!divider.hasPointerCapture(event.pointerId) || !workspace.value) return
  const bounds = workspace.value.getBoundingClientRect()
  setEditorShare(((event.clientX - bounds.left) / bounds.width) * 100)
}

function stepDivider(event: KeyboardEvent, step: number) {
  event.preventDefault()
  setEditorShare(editorShare.value + step)
}

function handleKeydown(event: KeyboardEvent) {
  if (isCompositionKey(event)) return
  if (
    markdown &&
    event.ctrlKey &&
    !event.altKey &&
    !event.metaKey &&
    !event.shiftKey &&
    event.code === 'Slash'
  ) {
    event.preventDefault()
    if (!event.repeat) changeView(view.value === 'source' ? 'visual' : 'source')
    return
  }
  if (
    event.key.toLowerCase() !== 's' ||
    !event.ctrlKey ||
    event.altKey ||
    event.metaKey ||
    event.shiftKey
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
    class="note-editor note-window"
    :class="{ 'note-markdown-editor': markdown }"
    aria-labelledby="note-title"
    @cancel.prevent="requestClose"
    @keydown="handleKeydown"
  >
    <h2 id="note-title">
      {{ note.title }}
      <span class="muted">{{ note.format.toUpperCase() }}</span>
    </h2>
    <div v-if="markdown" class="note-view-bar">
      <div class="note-view-switch" role="group" aria-label="编辑视图">
        <button
          v-for="(label, mode) in views"
          :key="mode"
          type="button"
          :aria-pressed="view === mode"
          :disabled="busy"
          @click="changeView(mode)"
        >
          {{ label }}
        </button>
      </div>
      <label class="note-theme-control">
        页面主题
        <select v-model="theme" aria-label="页面主题">
          <option v-for="(label, key) in themes" :key="key" :value="key">
            {{ label }}
          </option>
        </select>
      </label>
    </div>
    <p v-if="viewMessage" role="status" class="muted">{{ viewMessage }}</p>
    <section
      v-if="view === 'visual'"
      class="note-preview note-visual"
      :data-theme="theme"
    >
      <RichNoteEditor
        ref="visual-editor"
        :model-value="content"
        :readonly="busy"
        @edited="visualEdited = true"
      />
    </section>
    <div
      v-else
      ref="note-workspace"
      class="note-workspace"
      :class="{ 'has-preview': view === 'split' }"
      :style="{
        '--note-editor-share': editorShare + 'fr',
        '--note-preview-share': 100 - editorShare + 'fr',
      }"
    >
      <textarea
        ref="note-content"
        v-model="content"
        aria-label="笔记正文"
        :readonly="busy"
        spellcheck="false"
      />
      <div
        v-if="view === 'split'"
        class="note-divider"
        role="separator"
        aria-label="调整编辑和预览宽度"
        aria-orientation="vertical"
        :aria-valuenow="editorShare"
        :aria-valuemin="26"
        :aria-valuemax="74"
        tabindex="0"
        @pointerdown="dragDivider"
        @pointermove="dragDivider"
        @keydown.left="stepDivider($event, -2)"
        @keydown.right="stepDivider($event, 2)"
      />
      <section v-if="view === 'split'" class="note-preview" :data-theme="theme">
        <div class="note-preview-tools">
          <span>Markdown 预览</span>
        </div>
        <p v-if="previewError" class="note-preview-placeholder" role="status">
          {{ previewError }}
        </p>
        <p v-else-if="!content.trim()" class="note-preview-placeholder">
          输入 Markdown 后在这里预览。
        </p>
        <!-- Only HTML generated from the filtered Markdown parser is rendered here. -->
        <article
          v-else
          ref="preview-body"
          class="note-rendered"
          aria-label="预览正文"
          v-html="preview"
        />
      </section>
    </div>
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
      <button type="button" class="note-save" :disabled="busy" @click="save">
        保存 <kbd>Ctrl+S</kbd>
      </button>
    </div>
  </dialog>
</template>
