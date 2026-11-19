<script setup lang="ts">
import { nextTick, onMounted, ref, useTemplateRef } from 'vue'
import type { BackgroundTheme } from './command-types'
import { getHistoryStep, isCompositionKey } from './terminal-keyboard'

const props = defineProps<{
  themes: readonly BackgroundTheme[]
  selected: string
  busy: boolean
  error: string
}>()
const emit = defineEmits<{ choose: [name: string]; close: [] }>()
const list = useTemplateRef<HTMLDivElement>('background-options')
const highlighted = ref(
  Math.max(
    0,
    props.themes.findIndex(({ id }) => id === props.selected),
  ),
)

function pick(index: number) {
  const theme = props.themes[index]
  if (theme && !props.busy) emit('choose', theme.id)
}

function handleKeydown(event: KeyboardEvent) {
  if (isCompositionKey(event)) return
  const step = getHistoryStep(event)
  if (step) {
    event.preventDefault()
    if (!props.busy) {
      highlighted.value =
        (highlighted.value + step + props.themes.length) % props.themes.length
      void nextTick(() =>
        list.value?.children[highlighted.value]?.scrollIntoView({
          block: 'nearest',
        }),
      )
    }
  } else if (event.key === 'Enter' && !event.repeat) {
    event.preventDefault()
    pick(highlighted.value)
  } else if (event.key === 'Escape' && !event.repeat) {
    event.preventDefault()
    emit('close')
  }
}

onMounted(() => list.value?.focus())
</script>

<template>
  <section class="background-picker" role="dialog" aria-label="选择背景">
    <p class="picker-heading">选择背景</p>
    <div
      ref="background-options"
      class="picker-options"
      role="listbox"
      aria-label="背景"
      :aria-activedescendant="`background-option-${highlighted}`"
      :aria-busy="busy"
      tabindex="0"
      @keydown="handleKeydown"
    >
      <div
        v-for="(theme, index) in themes"
        :id="`background-option-${index}`"
        :key="theme.id"
        :aria-selected="index === highlighted"
        class="picker-option"
        role="option"
        @click="pick(index)"
      >
        <span>{{ theme.label }}</span>
        <span class="muted">{{
          theme.id === selected ? '当前' : theme.builtIn ? '内置' : ''
        }}</span>
      </div>
    </div>
    <p class="muted picker-hint">↑ ↓ 选择 · Enter 应用 · Esc 关闭</p>
    <p v-if="error" class="error" role="alert">{{ error }}</p>
  </section>
</template>
