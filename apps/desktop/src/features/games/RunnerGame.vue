<script setup lang="ts">
import { shallowRef, useTemplateRef, watch } from 'vue'
import { drawRunner } from './arcade-art'
import {
  advanceRunner,
  createRunner,
  jumpRunner,
  runnerHeight,
  runnerMeters,
  runnerWidth,
} from './runner'
import { useTerminalGame } from './use-terminal-game'

const props = defineProps<{ light: boolean }>()
const emit = defineEmits<{ finish: [summary: string] }>()
const panel = useTemplateRef<HTMLElement>('runner-panel')
const canvas = useTemplateRef<HTMLCanvasElement>('runner-board')
const state = shallowRef(createRunner())

function draw() {
  const context = canvas.value?.getContext('2d')
  if (context) drawRunner(context, state.value, props.light)
}

function handleKey(key: string, repeat: boolean) {
  if (key === ' ' || key === 'arrowup') {
    if (!repeat) state.value = jumpRunner(state.value)
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
        state.value = createRunner()
        draw()
      }
    }
    return true
  }
  return false
}

const best = useTerminalGame({
  id: 'runner',
  panel,
  interval: 1000 / 60,
  score: () => runnerMeters(state.value),
  running: () => state.value.status === 'running',
  draw,
  tick: (seconds) => {
    state.value = advanceRunner(state.value, seconds)
  },
  pause: () => {
    if (state.value.status === 'running')
      state.value = { ...state.value, status: 'paused' }
  },
  key: handleKey,
  finish: (record) => {
    const reason = state.value.hit
      ? '被 ' + state.value.hit + ' 绊倒。'
      : '已退出。'
    emit(
      'finish',
      '本次奔跑结束：' +
        reason +
        '\n距离：' +
        runnerMeters(state.value) +
        ' m\n最高纪录：' +
        record +
        ' m',
    )
  },
})
watch(() => props.light, draw)
</script>

<template>
  <section
    ref="runner-panel"
    class="arcade-panel"
    aria-label="Blis 跑酷"
    tabindex="-1"
  >
    <p>
      <strong>RUNNER</strong> · 距离：{{ runnerMeters(state) }} m · 最高纪录：{{
        best
      }}
      m
    </p>
    <p class="muted arcade-controls">空格 / ↑ 跳跃 · P 暂停或继续 · Esc 退出</p>
    <canvas
      ref="runner-board"
      :width="runnerWidth"
      :height="runnerHeight"
      aria-label="跑酷场景"
    />
    <p class="arcade-status" role="status">
      <template v-if="state.status === 'paused'">已暂停 · 按 P 继续</template>
      <template v-else-if="state.status === 'lost'"
        >被 {{ state.hit }} 绊倒 · 按 R 重开或 Esc 退出</template
      >
    </p>
  </section>
</template>
