import { Extension } from '@tiptap/core'
import { Plugin, TextSelection, type EditorState } from '@tiptap/pm/state'
import type { EditorView } from '@tiptap/pm/view'
import { isCompositionKey } from '../../terminal/terminal-keyboard'
import {
  indentEdits,
  isPairKey,
  newlineEdit,
  pairedBackspace,
  pairEdit,
} from './code-editing'

function codeSelection(
  state: EditorState,
  from = state.selection.from,
  to = state.selection.to,
) {
  const $from = state.doc.resolve(from)
  const $to = from === to ? $from : state.doc.resolve(to)
  if (!$from.sameParent($to) || $from.parent.type.name !== 'codeBlock') return
  return {
    start: $from.start(),
    source: $from.parent.textContent,
    language: $from.parent.attrs.language ?? '',
    range: { from: $from.parentOffset, to: $to.parentOffset },
  }
}

function applyEdit(
  view: EditorView,
  start: number,
  edit: NonNullable<ReturnType<typeof pairEdit>>,
) {
  const { state } = view
  const tr = state.tr
  if (edit.text || edit.from !== edit.to) {
    tr.insertText(edit.text, start + edit.from, start + edit.to)
  }
  const { from, to } = edit.select
  const backward = state.selection.anchor > state.selection.head
  tr.setSelection(
    TextSelection.create(
      tr.doc,
      start + (backward ? to : from),
      start + (backward ? from : to),
    ),
  )
  view.dispatch(tr.scrollIntoView())
}

export const codeBlockEditing = Extension.create({
  name: 'noteCodeBlockEditing',
  priority: 1000,
  addProseMirrorPlugins() {
    return [
      new Plugin({
        props: {
          handleTextInput(view, from, to, text) {
            if (!view.editable || view.composing || !isPairKey(text))
              return false
            const code = codeSelection(view.state, from, to)
            if (!code) return false
            const edit = pairEdit(code.source, code.range, text)
            if (!edit) return false
            applyEdit(view, code.start, edit)
            return true
          },
          handleKeyDown(view, event) {
            if (
              !view.editable ||
              view.composing ||
              isCompositionKey(event) ||
              event.ctrlKey ||
              event.metaKey ||
              event.altKey ||
              (event.key !== 'Tab' &&
                event.key !== 'Enter' &&
                event.key !== 'Backspace') ||
              (event.target instanceof Element &&
                event.target.closest('input, select, button'))
            )
              return false
            const code = codeSelection(view.state)
            if (!code) return false
            const { start, source, range, language } = code
            if (event.key === 'Tab') {
              const tr = view.state.tr
              const edits = indentEdits(source, range, event.shiftKey)
              for (const edit of edits.reverse()) {
                tr.insertText(edit.text, start + edit.from, start + edit.to)
              }
              if (edits.length) view.dispatch(tr.scrollIntoView())
              return true
            }
            if (event.shiftKey) return false
            if (event.key === 'Enter') {
              // Leave the editor's existing double-blank-line exit available.
              if (range.from === source.length && source.endsWith('\n\n'))
                return false
              applyEdit(view, start, newlineEdit(source, range, language))
              return true
            }
            if (range.from !== range.to) return false
            const pair = pairedBackspace(source, range.from)
            if (!pair) return false
            view.dispatch(
              view.state.tr
                .delete(start + pair.from, start + pair.to)
                .scrollIntoView(),
            )
            return true
          },
        },
      }),
    ]
  },
})
