import { Extension, InputRule } from '@tiptap/core'
import CodeBlockLowlight from '@tiptap/extension-code-block-lowlight'
import Image from '@tiptap/extension-image'
import { TaskItem, TaskList } from '@tiptap/extension-list'
import { TableKit } from '@tiptap/extension-table'
import { Markdown } from '@tiptap/markdown'
import StarterKit from '@tiptap/starter-kit'
import { VueNodeViewRenderer } from '@tiptap/vue-3'
import { codeBlockEditing } from './code-block-editing'
import { syntax } from './markdown-code'
import NoteCodeBlock from './NoteCodeBlock.vue'

const headingsWithoutSpace = Extension.create({
  name: 'headingsWithoutSpace',
  addInputRules() {
    return [
      new InputRule({
        find: /^(#{1,6})([^#\s].*)$/u,
        handler: ({ state, range, match }) => {
          const paragraph = state.doc.resolve(range.from).parent
          if (paragraph.type.name !== 'paragraph') return null
          state.tr
            .setBlockType(range.from, range.from, state.schema.nodes.heading, {
              level: match[1].length,
            })
            .insertText(match[2], range.from, range.to)
        },
      }),
    ]
  },
})

const editableCode = CodeBlockLowlight.extend({
  addNodeView() {
    return VueNodeViewRenderer(NoteCodeBlock)
  },
}).configure({ lowlight: syntax })

export const noteExtensions = [
  StarterKit.configure({
    codeBlock: false,
    link: { openOnClick: false, autolink: false, linkOnPaste: false },
    trailingNode: false,
    underline: false,
  }),
  Markdown.configure({ markedOptions: { gfm: true } }),
  editableCode,
  codeBlockEditing,
  TaskList,
  TaskItem.configure({ nested: true }),
  TableKit,
  Image.configure({ inline: true }),
  headingsWithoutSpace,
]
