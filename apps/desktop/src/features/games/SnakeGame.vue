<script setup lang="ts">
import { shallowRef, useTemplateRef, watch } from 'vue'
import {
  advanceSnake,
  createSnake,
  setSnakePaused,
  snakeColumns,
  snakeRows,
  steerSnake,
  type Direction,
  type Point,
} from './snake'
import { useTerminalGame } from './use-terminal-game'

const props = defineProps<{ light: boolean }>()
const emit = defineEmits<{ finish: [summary: string] }>()
const panel = useTemplateRef<HTMLElement>('snake-panel')
const canvas = useTemplateRef<HTMLCanvasElement>('snake-board')
const state = shallowRef(createSnake())
const size = 16
const width = snakeColumns * size
const height = snakeRows * size

const directions = new Map<string, Direction>([
  ['arrowup', 'up'],
  ['w', 'up'],
  ['arrowright', 'right'],
  ['d', 'right'],
  ['arrowdown', 'down'],
  ['s', 'down'],
  ['arrowleft', 'left'],
  ['a', 'left'],
])

const colors = {
  dark: {
    surface: '#121a1e',
    grid: '#243137',
    head: '#b3f1d5',
    body: '#67b898',
    food: '#ffae7f',
  },
  light: {
    surface: '#e7efeb',
    grid: '#c7d6d0',
    head: '#075641',
    body: '#218368',
    food: '#a43220',
  },
}

function drawBoard() {
  const ctx = canvas.value?.getContext('2d')
  if (!ctx) return
  const palette = props.light ? colors.light : colors.dark
  ctx.fillStyle = palette.surface
  ctx.fillRect(0, 0, width, height)
  ctx.beginPath()
  ctx.strokeStyle = palette.grid
  for (let x = size; x < width; x += size) {
    ctx.moveTo(x + 0.5, 0)
    ctx.lineTo(x + 0.5, height)
  }
  for (let y = size; y < height; y += size) {
    ctx.moveTo(0, y + 0.5)
    ctx.lineTo(width, y + 0.5)
  }
  ctx.stroke()

  const square = ({ x, y }: Point, inset: number, color: string) => {
    ctx.fillStyle = color
    ctx.fillRect(
      x * size + inset,
      y * size + inset,
      size - inset * 2,
      size - inset * 2,
    )
  }

  state.value.body.forEach((point, index) =>
    square(
      point,
      index === 0 ? 1 : 2,
      index === 0 ? palette.head : palette.body,
    ),
  )
  const food = state.value.food
  if (food) square(food, 4, palette.food)
}

function handleKey(key: string, repeat: boolean) {
  const direction = directions.get(key)
  if (direction) {
    state.value = steerSnake(state.value, direction)
    return true
  }
  if (key === ' ' || key === 'r') {
    if (repeat) return true
    if (key === ' ') {
      state.value = setSnakePaused(
        state.value,
        state.value.status === 'running',
      )
    } else if (state.value.status === 'lost' || state.value.status === 'won') {
      state.value = createSnake()
      drawBoard()
    }
    return true
  }
  return false
}

const best = useTerminalGame({
  id: 'snake',
  panel,
  interval: 140,
  running: () => state.value.status === 'running',
  score: () => state.value.score,
  tick: () => {
    state.value = advanceSnake(state.value)
  },
  draw: drawBoard,
  pause: () => {
    state.value = setSnakePaused(state.value, true)
  },
  key: handleKey,
  finish: (record) =>
    emit(
      'finish',
      '本局结束：得分 ' + state.value.score + '，最高分 ' + record + '。',
    ),
})
watch(() => props.light, drawBoard)
</script>

<template>
  <section ref="snake-panel" class="snake" aria-label="贪吃蛇" tabindex="-1">
    <p class="score">
      <strong>SNAKE</strong> · 得分：{{ state.score }} · 最高分：{{ best }}
    </p>
    <p class="muted controls">方向键 / WASD 移动 · 空格暂停或继续 · Esc 退出</p>
    <canvas
      ref="snake-board"
      :width="width"
      :height="height"
      aria-label="贪吃蛇棋盘"
    />
    <p class="status" role="status">
      <template v-if="state.status === 'paused'">已暂停 · 按空格继续</template>
      <template v-else-if="state.status === 'lost'"
        >本局结束 · 按 R 重开或 Esc 退出</template
      >
      <template v-else-if="state.status === 'won'"
        >棋盘已填满 · 按 R 重开或 Esc 退出</template
      >
    </p>
  </section>
</template>

<style scoped>
.snake {
  max-width: max-content;
  margin-top: 0.45em;
}

.snake:focus {
  outline: none;
}

.score strong {
  color: var(--accent);
}

.controls {
  margin: 0.2em 0 0.5em;
}

canvas {
  display: block;
  max-width: 100%;
  height: auto;
  outline: 1px solid var(--menu-highlight);
  image-rendering: pixelated;
}

.status {
  min-height: 1.55em;
  margin-top: 0.4em;
  color: var(--accent);
}
</style>
