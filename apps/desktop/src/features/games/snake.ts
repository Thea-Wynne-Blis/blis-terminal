export const snakeColumns = 20
export const snakeRows = 14

export type Direction = 'up' | 'right' | 'down' | 'left'
export type Point = { x: number; y: number }
export type SnakeState = {
  body: Point[]
  food?: Point
  direction: Direction
  turn?: Direction
  score: number
  status: 'running' | 'paused' | 'lost' | 'won'
}

const offsets: Record<Direction, Point> = {
  up: { x: 0, y: -1 },
  right: { x: 1, y: 0 },
  down: { x: 0, y: 1 },
  left: { x: -1, y: 0 },
}

const opposite: Record<Direction, Direction> = {
  up: 'down',
  right: 'left',
  down: 'up',
  left: 'right',
}

function sameCell(a: Point, b: Point): boolean {
  return a.x === b.x && a.y === b.y
}

function placeFood(body: Point[], random: () => number): Point | undefined {
  const occupied = new Set(body.map(({ x, y }) => y * snakeColumns + x))
  const freeCount = snakeColumns * snakeRows - occupied.size
  if (!freeCount) return

  let remaining = Math.floor(random() * freeCount)
  for (let position = 0; position < snakeColumns * snakeRows; position++) {
    if (occupied.has(position)) continue
    if (remaining === 0)
      return {
        x: position % snakeColumns,
        y: Math.floor(position / snakeColumns),
      }
    remaining--
  }
}

export function createSnake(random = Math.random): SnakeState {
  const body = [
    { x: 5, y: 7 },
    { x: 4, y: 7 },
    { x: 3, y: 7 },
  ]
  return {
    body,
    food: placeFood(body, random),
    direction: 'right',
    score: 0,
    status: 'running',
  }
}

export function steerSnake(
  state: SnakeState,
  direction: Direction,
): SnakeState {
  if (
    state.status !== 'running' ||
    state.turn ||
    direction === state.direction ||
    direction === opposite[state.direction]
  )
    return state
  return { ...state, turn: direction }
}

export function setSnakePaused(state: SnakeState, paused: boolean): SnakeState {
  if (state.status !== 'running' && state.status !== 'paused') return state
  const status = paused ? 'paused' : 'running'
  return state.status === status ? state : { ...state, status }
}

export function advanceSnake(
  state: SnakeState,
  random = Math.random,
): SnakeState {
  if (state.status !== 'running') return state

  const direction = state.turn ?? state.direction
  const head = state.body[0]
  const offset = offsets[direction]
  const next = { x: head.x + offset.x, y: head.y + offset.y }
  const eats = state.food !== undefined && sameCell(next, state.food)
  // The tail vacates its cell on this move unless the snake grows.
  const retained = eats ? state.body : state.body.slice(0, -1)
  if (
    next.x < 0 ||
    next.x >= snakeColumns ||
    next.y < 0 ||
    next.y >= snakeRows ||
    retained.some((cell) => sameCell(cell, next))
  )
    return { ...state, status: 'lost' }

  const body = [next, ...retained]
  const food = eats ? placeFood(body, random) : state.food
  return {
    body,
    food,
    direction,
    score: state.score + (eats ? 10 : 0),
    status: food ? 'running' : 'won',
  }
}
