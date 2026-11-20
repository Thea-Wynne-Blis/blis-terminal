import { computed, onScopeDispose, ref, watch, type Ref } from 'vue'
import { executeNote } from '../../platform/desktop-platform'
import { createPreviewQueue } from './preview-queue'

export function useNotePreview(
  content: Readonly<Ref<string>>,
  enabled: Readonly<Ref<boolean>>,
  body: Readonly<Ref<HTMLElement | null>>,
) {
  const preview = ref('')
  const previewError = ref('')
  const queue = createPreviewQueue(
    async (text) => {
      const outcome = await executeNote({ action: 'preview', content: text })
      if (outcome.type !== 'preview') throw new Error('无法生成预览。')
      return outcome.text
    },
    (html) => {
      preview.value = html
      previewError.value = ''
    },
    () => {
      previewError.value = '暂时无法生成预览。'
    },
  )

  const source = computed(() => (enabled.value ? content.value : ''))
  watch(
    source,
    (text) => {
      if (text.trim()) queue.update(text)
      else {
        queue.clear()
        preview.value = ''
        previewError.value = ''
      }
    },
    { immediate: true },
  )
  onScopeDispose(queue.clear)

  watch(
    [preview, body],
    ([, element], _previous, onCleanup) => {
      if (!element?.querySelector('pre > code[class^="language-"]')) return
      let current = true
      onCleanup(() => {
        current = false
      })
      void import('./markdown-code')
        .then(({ decorateMarkdownCode }) => {
          if (current) decorateMarkdownCode(element)
        })
        .catch(() => {
          // The HTML preview remains available if highlighting fails.
        })
    },
    { flush: 'post' },
  )
  return { preview, previewError }
}
