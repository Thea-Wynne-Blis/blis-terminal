import {
  dodgeHazardSize,
  dodgeHeight,
  dodgeLanes,
  dodgePlayerHeight,
  dodgePlayerY,
  dodgeWidth,
  type DodgeState,
} from './dodge'
import {
  runnerGround,
  runnerHeight,
  runnerObstacles,
  runnerPlayerHeight,
  runnerPlayerX,
  runnerWidth,
  type RunnerState,
} from './runner'

const themes = {
  dark: {
    sky: '#111c27',
    structure: '#1b303d',
    line: '#314959',
    floor: '#1b3439',
    edge: '#6cc8ae',
    bright: '#a9f5d5',
    suit: '#49a189',
    visor: '#0c2730',
    warm: '#ffad75',
    obstacleInk: '#1b232a',
    shadow: '#297283',
  },
  light: {
    sky: '#e7f0ee',
    structure: '#d4e5e4',
    line: '#adc8ce',
    floor: '#c0d8d0',
    edge: '#27775f',
    bright: '#075641',
    suit: '#218368',
    visor: '#d4f3e6',
    warm: '#a33b26',
    obstacleInk: '#ffffff',
    shadow: '#508a9a',
  },
}

function rect(
  ctx: CanvasRenderingContext2D,
  color: string,
  x: number,
  y: number,
  w: number,
  h: number,
) {
  ctx.fillStyle = color
  ctx.fillRect(Math.round(x), Math.round(y), w, h)
}

function tag(
  ctx: CanvasRenderingContext2D,
  text: string,
  color: string,
  x: number,
  y: number,
) {
  ctx.fillStyle = color
  ctx.font = 'bold 8px monospace'
  ctx.textAlign = 'center'
  ctx.fillText(text, Math.round(x), Math.round(y))
}

export function drawRunner(
  ctx: CanvasRenderingContext2D,
  state: RunnerState,
  light: boolean,
) {
  const p = light ? themes.light : themes.dark
  rect(ctx, p.sky, 0, 0, runnerWidth, runnerHeight)
  // Slowly moving traces behind the faster ground ticks give the scene depth.
  for (let index = 0; index < 9; index++) {
    const x = index * 62 - ((state.distance * 0.25) % 62)
    const y = 61 + ((index * 29) % 66)
    rect(ctx, p.structure, x, y, 27, 3)
    rect(ctx, p.structure, x + 23, y, 4, runnerGround - y)
    rect(ctx, p.line, x + 23, y, 6, 6)
  }
  rect(ctx, p.floor, 0, runnerGround, runnerWidth, runnerHeight - runnerGround)
  rect(ctx, p.edge, 0, runnerGround, runnerWidth, 2)
  for (let x = -(state.distance % 28); x < runnerWidth; x += 28) {
    rect(ctx, p.line, x, runnerGround + 10, 12, 3)
    rect(ctx, p.structure, x + 10, runnerGround + 21, 7, 3)
  }
  for (const { x, kind } of state.obstacles) {
    const { width, height } = runnerObstacles[kind]
    const y = runnerGround - height
    const inset = kind === '404' ? 3 : 0
    rect(ctx, p.shadow, x + 2, y + 3, width, height - 3)
    rect(ctx, p.warm, x + inset, y, width - inset * 2, height)
    rect(ctx, p.structure, x + inset + 2, y + 3, width - inset * 2 - 4, 2)
    if (kind === 'TIMEOUT') {
      rect(ctx, p.obstacleInk, x + 2, y + height - 6, width - 4, 2)
    }
    tag(ctx, kind, p.obstacleInk, x + width / 2, y + height / 2 + 5)
  }

  const x = runnerPlayerX
  const y = runnerGround - state.height - runnerPlayerHeight
  const stride = Math.floor(state.distance / 10) % 4
  const offset = state.height > 0 ? 1 : [0, 2, 4, 2][stride]
  rect(ctx, p.shadow, x, y + 8, 5, 10)
  rect(ctx, p.warm, x + 1, y + 10, 2, 3)
  rect(ctx, p.suit, x + 5, y + 8, 9, 10)
  rect(ctx, p.bright, x + 6, y, 10, 8)
  rect(ctx, p.visor, x + 11, y + 2, 5, 3)
  rect(ctx, p.suit, x + 5 + offset, y + 17, 4, 6)
  rect(ctx, p.bright, x + 10 - offset, y + 18, 5, 6)
  rect(ctx, p.warm, x + 12, y + 10, 3, 4)
}

export function drawDodge(
  ctx: CanvasRenderingContext2D,
  state: DodgeState,
  light: boolean,
) {
  const p = light ? themes.light : themes.dark
  const left = 23
  const laneWidth = (dodgeWidth - left * 2) / dodgeLanes
  const laneX = (lane: number) => left + lane * laneWidth
  const bottom = dodgePlayerY + dodgePlayerHeight
  rect(ctx, p.sky, 0, 0, dodgeWidth, dodgeHeight)
  rect(ctx, p.structure, laneX(state.lane), 0, laneWidth, bottom)
  for (let lane = 0; lane <= dodgeLanes; lane++) {
    rect(ctx, p.line, laneX(lane), 0, 1, bottom)
    for (let y = 23; y < dodgePlayerY; y += 30) {
      rect(ctx, p.line, laneX(lane) - 2, y, 5, 1)
    }
  }
  rect(ctx, p.floor, 0, bottom, dodgeWidth, dodgeHeight - bottom)
  rect(ctx, p.edge, 0, bottom, dodgeWidth, 2)
  for (const hazard of state.hazards) {
    const x = laneX(hazard.lane) + (laneWidth - dodgeHazardSize) / 2
    const color =
      hazard.kind === 'ERR'
        ? p.warm
        : hazard.kind === 'WARN'
          ? p.shadow
          : p.suit
    rect(ctx, p.line, x + 3, hazard.y - 13, 2, 12)
    rect(ctx, p.line, x + 17, hazard.y - 9, 2, 8)
    rect(ctx, color, x, hazard.y, dodgeHazardSize, dodgeHazardSize)
    rect(ctx, p.sky, x + 2, hazard.y + 2, dodgeHazardSize - 4, 3)
    tag(ctx, hazard.kind, p.obstacleInk, x + dodgeHazardSize / 2, hazard.y + 17)
  }

  const x = laneX(state.lane) + laneWidth / 2
  rect(ctx, p.suit, x - 11, dodgePlayerY + 6, 22, 10)
  rect(ctx, p.bright, x - 7, dodgePlayerY + 2, 14, 12)
  rect(ctx, p.visor, x - 4, dodgePlayerY + 5, 8, 4)
  rect(ctx, p.warm, x - 8, dodgePlayerY + 15, 4, 4)
  rect(ctx, p.warm, x + 4, dodgePlayerY + 15, 4, 4)
}
