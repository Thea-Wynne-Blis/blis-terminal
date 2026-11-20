<script setup lang="ts">
import { shallowRef, useTemplateRef, watch } from 'vue'
import { drawDodge } from './arcade-art'
import {
  advanceDodge,
  createDodge,
  dodgeHeight,
  dodgeWidth,
  moveDodge,
} from './dodge'
import { useTerminalGame } from './use-terminal-game'

const props = defineProps<{ light: boolean }>()
const emit = defineEmits<{ finish: [summary: string] }>()
const panel = useTemplateRef<HTMLElement>('dodge-panel')
const canvas = useTemplateRef<HTMLCanvasElement>('dodge-board')
const state = shallowRef(createDodge())

function draw() {
  const context = canvas.value?.getContext('2d')
  if (context) drawDodge(context, state.value, props.light)
}

function handleKey(key: string, repeat: boolean) {
  if (
    key === 'arrowleft' ||
    key === 'a' ||
    key === 'arrowright' ||
    key === 'd'
  ) {
    state.value = moveDodge(
      state.value,
      key === 'arrowleft' || key === 'a' ? -1 : 1,
    )
    draw()
    return true
  }
  if (key === 'p' || key === 'r') {
    if (!repeat) {
      if (key === 'p' && state.value.status !== 'lost')
        state.value = {
          ...state.value,
          status: state.value.status === 'paused' ? 'running' : 'paused',
        }
      if (key === 'r' && state.value.status === 'lost') {
        state.value = createDodge()
        draw()
      }
    }
    return true
  }
  return false
}

const best = useTerminalGame({
  id: 'dodge',
  panel,
  interval: 1000 / 60,
  score: () => state.value.score,
  running: () => state.value.status === 'running',
  draw,
  tick: (seconds) => {
    state.value = advanceDodge(state.value, seconds)
  },
  pause: () => {
    if (state.value.status === 'running')
      state.value = { ...state.value, status: 'paused' }
  },
  key: handleKey,
  finish: (record) => {
    const reason = state.value.hit
      ? '被 ' + state.value.hit + ' 击中。'
      : '已退出。'
    emit(
      'finish',
      '本局躲避结束：' +
        reason +
        '\n躲过：' +
        state.value.score +
        ' 个\n最高纪录：' +
        record +
        ' 个',
    )
  },
})
watch(() => props.light, draw)
</script>

<template>
  <section
    ref="dodge-panel"
    class="arcade-panel"
    aria-label="躲避掉落物"
    tabindex="-1"
  >
    <p>
      <strong>DODGE</strong> · 躲过：{{ state.score }} 个 · 最高纪录：{{ best }}
      个
    </p>
    <p class="muted arcade-controls">
      ← → / A D 左右移动 · P 暂停或继续 · Esc 退出
    </p>
    <canvas
      ref="dodge-board"
      :width="dodgeWidth"
      :height="dodgeHeight"
      aria-label="躲避场景"
    />
    <p class="arcade-status" role="status">
      <template v-if="state.status === 'paused'">已暂停 · 按 P 继续</template>
      <template v-else-if="state.status === 'lost'"
        >被 {{ state.hit }} 击中 · 按 R 重开或 Esc 退出</template
      >
    </p>
  </section>
</template>
