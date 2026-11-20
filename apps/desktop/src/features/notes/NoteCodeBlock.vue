<script setup lang="ts">
import { NodeViewContent, NodeViewWrapper, nodeViewProps } from '@tiptap/vue-3'
import { codeLanguages } from './markdown-code'

const props = defineProps(nodeViewProps)

function changeLanguage(event: Event) {
  if (!props.editor.isEditable) return
  const { value } = event.target as HTMLSelectElement
  props.updateAttributes({ language: value || null })
}
</script>

<template>
  <NodeViewWrapper as="figure" class="note-code-block">
    <pre><NodeViewContent as="code" /></pre>
    <figcaption contenteditable="false">
      <select
        :value="node.attrs.language ?? ''"
        :disabled="!editor.isEditable"
        aria-label="代码语言"
        @change="changeLanguage"
      >
        <option value="">纯文本</option>
        <option
          v-if="
            node.attrs.language && !codeLanguages.includes(node.attrs.language)
          "
          :value="node.attrs.language"
        >
          {{ node.attrs.language }}
        </option>
        <option
          v-for="language in codeLanguages"
          :key="language"
          :value="language"
        >
          {{ language }}
        </option>
      </select>
    </figcaption>
  </NodeViewWrapper>
</template>
