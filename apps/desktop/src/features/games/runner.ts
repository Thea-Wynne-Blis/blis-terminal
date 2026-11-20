export const runnerWidth = 480
export const runnerHeight = 208
export const runnerGround = 174
export const runnerPlayerX = 66
export const runnerPlayerWidth = 16
export const runnerPlayerHeight = 24
export const runnerStartSpeed = 126
export const runnerMaxSpeed = 254
const acceleration = 5.8
const gravity = 842
const jumpSpeed = 327
const flightTime = (2 * jumpSpeed) / gravity

export const runnerObstacles = {
  BUG: { width: 27, height: 24 },
  '404': { width: 32, height: 27 },
  TIMEOUT: { width: 42, height: 34 },
}
export type RunnerObstacle = keyof typeof runnerObstacles
const trainingOrder: RunnerObstacle[] = [
  'BUG',
  '404',
  'TIMEOUT',
  '404',
  'BUG',
  'TIMEOUT',
]
const obstacleKinds = Object.keys(runnerObstacles) as RunnerObstacle[]

export type RunnerState = {
  speed: number
  distance: number
  height: number
  verticalSpeed: number
  obstacles: { x: number; kind: RunnerObstacle }[]
  spawned: number
  nextObstacleAt: number
  status: 'running' | 'paused' | 'lost'
  hit?: RunnerObstacle
}

export function createRunner(): RunnerState {
  return {
    speed: runnerStartSpeed,
    distance: 0,
    height: 0,
    verticalSpeed: 0,
    obstacles: [],
    spawned: 0,
    nextObstacleAt: 52,
    status: 'running',
  }
}

export function runnerMeters(state: RunnerState) {
  return Math.floor(state.distance * 0.45)
}

export function jumpRunner(state: RunnerState): RunnerState {
  if (state.status !== 'running' || state.height > 0 || state.verticalSpeed > 0)
    return state
  return { ...state, verticalSpeed: jumpSpeed }
}

export function runnerSpacing(speed: number, previousWidth: number) {
  const recoveryTime = flightTime + 0.32
  // Account for acceleration while the obstacle travels from the edge of the board.
  const travelTime = runnerWidth / runnerStartSpeed
  const futureSpeed = Math.min(
    runnerMaxSpeed,
    speed + acceleration * (travelTime + recoveryTime),
  )
  return (
    previousWidth + runnerPlayerWidth + Math.ceil(futureSpeed * recoveryTime)
  )
}

export function advanceRunner(
  state: RunnerState,
  seconds: number,
  random = Math.random,
): RunnerState {
  if (state.status !== 'running') return state
  const speed = Math.min(runnerMaxSpeed, state.speed + acceleration * seconds)
  const offset = ((state.speed + speed) / 2) * seconds
  const distance = state.distance + offset
  const height = Math.max(
    0,
    state.height + state.verticalSpeed * seconds - (gravity * seconds ** 2) / 2,
  )
  const verticalSpeed = height > 0 ? state.verticalSpeed - gravity * seconds : 0
  const obstacles = state.obstacles
    .map(({ x, kind }) => ({ x: x - offset, kind }))
    .filter(({ x, kind }) => x + runnerObstacles[kind].width > 0)

  let { spawned, nextObstacleAt } = state
  if (distance >= nextObstacleAt) {
    const kind =
      trainingOrder[spawned] ??
      obstacleKinds[Math.floor(random() * obstacleKinds.length)]
    obstacles.push({ x: runnerWidth - (distance - nextObstacleAt), kind })
    nextObstacleAt += runnerSpacing(speed, runnerObstacles[kind].width)
    if (spawned >= trainingOrder.length)
      nextObstacleAt += Math.floor(random() * 40)
    spawned++
  }

  const collision = obstacles.find(({ x, kind }) => {
    const obstacle = runnerObstacles[kind]
    return (
      x < runnerPlayerX + runnerPlayerWidth &&
      x + obstacle.width > runnerPlayerX &&
      height < obstacle.height
    )
  })
  return {
    speed,
    distance,
    height,
    verticalSpeed,
    obstacles,
    spawned,
    nextObstacleAt,
    status: collision ? 'lost' : 'running',
    hit: collision?.kind,
  }
}
