<script setup lang="ts">
import type { ChainedCommands } from '@tiptap/core'
import type { Node as ProseMirrorNode } from '@tiptap/pm/model'
import { EditorContent, useEditor } from '@tiptap/vue-3'
import { computed, nextTick, ref, shallowRef, useTemplateRef, watch } from 'vue'
import { isCompositionKey } from '../../terminal/terminal-keyboard'
import { noteExtensions } from './rich-note-extensions'
import { readVisualMarkdown } from './note-source'

const props = defineProps<{ modelValue: string; readonly: boolean }>()
const emit = defineEmits<{ edited: [] }>()
const showOutline = ref(false)
const linkOpen = ref(false)
const linkUrl = ref('')
const linkError = ref('')
const linkField = useTemplateRef<HTMLInputElement>('link-field')
let source = props.modelValue
let baseline: ProseMirrorNode | undefined
let exported: string | undefined

const editor = useEditor({
  content: readVisualMarkdown(source) ?? '',
  contentType: 'markdown',
  extensions: noteExtensions,
  injectCSS: false,
  editable: !props.readonly,
  editorProps: {
    attributes: {
      role: 'textbox',
      'aria-label': '笔记正文',
      'aria-multiline': 'true',
    },
    handleDOMEvents: {
      click: (_view, event) => {
        if (event.target instanceof Element && event.target.closest('a'))
          event.preventDefault()
        return false
      },
    },
  },
  onUpdate: () => emit('edited'),
})

function readMarkdown(): string {
  const current = editor.value
  if (!current || baseline?.eq(current.state.doc)) return source
  exported = current.getMarkdown()
  return exported
}

defineExpose({ readMarkdown })

watch(
  editor,
  (current) => {
    if (!current) return
    baseline = current.state.doc
    current.commands.focus('start')
  },
  { flush: 'post' },
)
watch(
  () => props.readonly,
  (value) => editor.value?.setEditable(!value, false),
)
watch(
  () => props.modelValue,
  (value) => {
    if (value === source) return
    source = value
    const current = editor.value
    if (!current) return
    if (value !== exported) {
      current.commands.setContent(readVisualMarkdown(value) ?? '', {
        contentType: 'markdown',
        emitUpdate: false,
      })
    }
    baseline = current.state.doc
    exported = undefined
  },
)

type Heading = 1 | 2 | 3 | 4 | 5 | 6
const headingLevels: readonly Heading[] = [1, 2, 3, 4, 5, 6]
const heading = computed(() => {
  const current = editor.value
  return current?.isActive('heading')
    ? Number(current.getAttributes('heading').level)
    : 0
})
type OutlineItem = { position: number; level: Heading; text: string }
const outline = shallowRef<OutlineItem[]>([])
watch([showOutline, () => editor.value?.state.doc], ([visible, document]) => {
  const headings: OutlineItem[] = []
  if (visible) {
    document?.descendants((node, position) => {
      if (node.type.name === 'heading' && node.textContent.trim()) {
        headings.push({
          position: position + 1,
          level: node.attrs.level,
          text: node.textContent,
        })
      }
    })
  }
  outline.value = headings
})
const inTable = computed(() => editor.value?.isActive('table') ?? false)

type Format = (chain: ChainedCommands) => ChainedCommands
const formats: { name: string; label: string; title: string; run: Format }[] = [
  {
    name: 'bold',
    label: '粗体',
    title: 'Ctrl+B',
    run: (chain) => chain.toggleBold(),
  },
  {
    name: 'italic',
    label: '斜体',
    title: 'Ctrl+I',
    run: (chain) => chain.toggleItalic(),
  },
  {
    name: 'strike',
    label: '删除线',
    title: 'Alt+Shift+5',
    run: (chain) => chain.toggleStrike(),
  },
  {
    name: 'bulletList',
    label: '无序列表',
    title: 'Ctrl+Shift+]',
    run: (chain) => chain.toggleBulletList(),
  },
  {
    name: 'orderedList',
    label: '有序列表',
    title: 'Ctrl+Shift+[',
    run: (chain) => chain.toggleOrderedList(),
  },
  {
    name: 'taskList',
    label: '待办',
    title: '可勾选的任务列表',
    run: (chain) => chain.toggleTaskList(),
  },
  {
    name: 'blockquote',
    label: '引用',
    title: 'Ctrl+Shift+Q',
    run: (chain) => chain.toggleBlockquote(),
  },
  {
    name: 'code',
    label: '行内代码',
    title: 'Ctrl+Shift+反引号',
    run: (chain) => chain.toggleCode(),
  },
  {
    name: 'codeBlock',
    label: '代码块',
    title: 'Ctrl+Shift+K',
    run: (chain) => chain.toggleCodeBlock(),
  },
  {
    name: 'table',
    label: '表格',
    title: 'Ctrl+T',
    run: (chain) =>
      chain.insertTable({ rows: 3, cols: 3, withHeaderRow: true }),
  },
]

const formatKeys: Record<string, string> = {
  'Shift+BracketRight': 'bulletList',
  'Shift+BracketLeft': 'orderedList',
  'Shift+KeyQ': 'blockquote',
  'Shift+Backquote': 'code',
  'Shift+KeyK': 'codeBlock',
  KeyT: 'table',
}

function apply(run: Format) {
  if (editor.value && !props.readonly) run(editor.value.chain().focus()).run()
}

function changeHeading(level: number) {
  if (level === 0) apply((chain) => chain.setParagraph())
  else if (headingLevels.some((value) => value === level))
    apply((chain) => chain.setHeading({ level: level as Heading }))
}

function showLink() {
  if (!editor.value || props.readonly) return
  const href = editor.value.getAttributes('link').href
  linkUrl.value = typeof href === 'string' ? href : ''
  linkError.value = ''
  linkOpen.value = true
  void nextTick(() => linkField.value?.focus())
}

function closeLink() {
  linkOpen.value = false
  editor.value?.commands.focus()
}

function setLink() {
  const href = linkUrl.value.trim()
  if (!/^(https?:\/\/|mailto:)\S+$/iu.test(href)) {
    linkError.value = '请输入 http://、https:// 或 mailto: 链接。'
    return
  }
  const current = editor.value
  if (current?.state.selection.empty && !current.isActive('link')) {
    apply((chain) =>
      chain.insertContent({
        type: 'text',
        text: href,
        marks: [{ type: 'link', attrs: { href } }],
      }),
    )
  } else {
    apply((chain) => chain.extendMarkRange('link').setLink({ href }))
  }
  closeLink()
}

function claimKey(event: KeyboardEvent) {
  event.preventDefault()
  event.stopPropagation()
}

function keyboard(event: KeyboardEvent) {
  if (isCompositionKey(event) || props.readonly) return
  const current = editor.value
  if (
    !current ||
    !(event.target instanceof HTMLElement) ||
    !event.target.closest('.tiptap') ||
    event.target.closest('button, input, select')
  )
    return
  const { code, ctrlKey, metaKey, altKey, shiftKey } = event
  if (event.key === 'Enter' && !ctrlKey && !metaKey && !altKey && !shiftKey) {
    const { empty, $from } = current.state.selection
    const prefix = /^(\x60{3}|~{3})([\w+#-]*)$/u.exec($from.parent.textContent)
    if (
      empty &&
      $from.parent.type.name === 'paragraph' &&
      $from.pos === $from.end() &&
      prefix
    ) {
      claimKey(event)
      current
        .chain()
        .deleteRange({ from: $from.start(), to: $from.end() })
        .setCodeBlock(prefix[2] ? { language: prefix[2] } : undefined)
        .run()
    }
    return
  }
  if (altKey && shiftKey && code === 'Digit5' && !ctrlKey && !metaKey) {
    claimKey(event)
    apply((chain) => chain.toggleStrike())
    return
  }
  if (!(ctrlKey || metaKey) || altKey) return
  const format = formats.find(
    ({ name }) => name === formatKeys[(shiftKey ? 'Shift+' : '') + code],
  )
  if (format) {
    claimKey(event)
    apply(format.run)
  } else if (!shiftKey && /^Digit[0-6]$/u.test(code)) {
    claimKey(event)
    changeHeading(Number(code.at(-1)))
  } else if (!shiftKey && code === 'KeyK') {
    claimKey(event)
    showLink()
  } else if (!shiftKey && code === 'Backslash') {
    claimKey(event)
    apply((chain) => chain.unsetAllMarks().clearNodes())
  } else if (shiftKey && code === 'Digit1') {
    claimKey(event)
    showOutline.value = !showOutline.value
  }
}
</script>

<template>
  <section
    class="rich-note"
    aria-label="可视化 Markdown 编辑"
    @keydown.capture="keyboard"
  >
    <div class="rich-note-toolbar" role="group" aria-label="Markdown 格式">
      <select
        :value="heading"
        :disabled="readonly || !editor"
        aria-label="段落和标题"
        title="Ctrl+0 至 Ctrl+6"
        @change="
          changeHeading(Number(($event.target as HTMLSelectElement).value))
        "
      >
        <option value="0">正文</option>
        <option v-for="level in headingLevels" :key="level" :value="level">
          标题 {{ level }}
        </option>
      </select>
      <button
        v-for="format in formats"
        :key="format.name"
        type="button"
        :title="format.title"
        :aria-pressed="editor?.isActive(format.name) ?? false"
        :disabled="readonly || !editor"
        @mousedown.prevent
        @click="apply(format.run)"
      >
        {{ format.label }}
      </button>
      <button
        type="button"
        title="Ctrl+K"
        :disabled="readonly || !editor"
        @mousedown.prevent
        @click="showLink"
      >
        链接
      </button>
      <button
        type="button"
        title="Ctrl+Shift+1"
        :aria-pressed="showOutline"
        @click="showOutline = !showOutline"
      >
        大纲
      </button>
    </div>
    <form
      v-if="linkOpen"
      class="rich-note-toolbar"
      @submit.prevent="setLink"
      @keydown.esc.stop.prevent="closeLink"
    >
      <input
        ref="link-field"
        v-model="linkUrl"
        aria-label="链接地址"
        placeholder="https://example.com"
      />
      <button type="submit" :disabled="readonly">应用链接</button>
      <button type="button" @click="closeLink">取消</button>
      <span v-if="linkError" role="alert">{{ linkError }}</span>
    </form>
    <div
      v-if="inTable"
      class="rich-note-toolbar"
      role="group"
      aria-label="表格编辑"
    >
      <button
        type="button"
        :disabled="readonly"
        @mousedown.prevent
        @click="apply((chain) => chain.addRowAfter())"
      >
        加行
      </button>
      <button
        type="button"
        :disabled="readonly"
        @mousedown.prevent
        @click="apply((chain) => chain.addColumnAfter())"
      >
        加列
      </button>
      <button
        type="button"
        :disabled="readonly"
        @mousedown.prevent
        @click="apply((chain) => chain.deleteRow())"
      >
        删行
      </button>
      <button
        type="button"
        :disabled="readonly"
        @mousedown.prevent
        @click="apply((chain) => chain.deleteColumn())"
      >
        删列
      </button>
      <button
        type="button"
        :disabled="readonly"
        @mousedown.prevent
        @click="apply((chain) => chain.deleteTable())"
      >
        删除表格
      </button>
    </div>
    <div class="rich-note-workspace">
      <nav v-if="showOutline" class="rich-note-outline" aria-label="笔记大纲">
        <p v-if="!outline.length">暂无标题</p>
        <button
          v-for="item in outline"
          :key="item.position"
          type="button"
          :style="{ '--heading-indent': item.level - 1 }"
          @click="editor?.commands.focus(item.position)"
        >
          {{ item.text }}
        </button>
      </nav>
      <EditorContent
        :editor="editor"
        class="note-rendered rich-note-document"
      />
    </div>
  </section>
</template>
