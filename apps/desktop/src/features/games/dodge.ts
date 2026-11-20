export const dodgeWidth = 480
export const dodgeHeight = 208
export const dodgeLanes = 7
export const dodgePlayerY = 171
export const dodgePlayerHeight = 19
export const dodgeHazardSize = 23
const hazardKinds = ['DROP', 'WARN', 'ERR'] as const
export type HazardKind = (typeof hazardKinds)[number]

export type DodgeState = {
  lane: number
  elapsed: number
  nextWaveAt: number
  hazards: { lane: number; y: number; kind: HazardKind }[]
  score: number
  status: 'running' | 'paused' | 'lost'
  hit?: HazardKind
}

export function createDodge(): DodgeState {
  return {
    lane: 3,
    elapsed: 0,
    nextWaveAt: 0.55,
    hazards: [],
    score: 0,
    status: 'running',
  }
}

export function moveDodge(state: DodgeState, step: -1 | 1): DodgeState {
  if (state.status !== 'running') return state
  const lane = Math.min(dodgeLanes - 1, Math.max(0, state.lane + step))
  return lane === state.lane ? state : { ...state, lane }
}

export function advanceDodge(
  state: DodgeState,
  seconds: number,
  random = Math.random,
): DodgeState {
  if (state.status !== 'running') return state
  const elapsed = state.elapsed + seconds
  const fallSpeed = Math.min(202, 77 + elapsed * 3.7)
  const hazards = state.hazards.map((hazard) => ({
    ...hazard,
    y: hazard.y + fallSpeed * seconds,
  }))
  let nextWaveAt = state.nextWaveAt
  if (elapsed >= nextWaveAt) {
    const lane = Math.floor(random() * dodgeLanes)
    const kind = () => hazardKinds[Math.floor(random() * hazardKinds.length)]
    hazards.push({ lane, y: -dodgeHazardSize, kind: kind() })
    if (elapsed >= 10 && random() < 0.62) {
      const offset = 1 + Math.floor(random() * (dodgeLanes - 1))
      hazards.push({
        lane: (lane + offset) % dodgeLanes,
        y: -dodgeHazardSize,
        kind: kind(),
      })
    }
    nextWaveAt += Math.max(0.44, 1.13 - elapsed * 0.019)
  }

  const collision = hazards.find(
    ({ lane, y }) =>
      lane === state.lane &&
      y < dodgePlayerY + dodgePlayerHeight &&
      y + dodgeHazardSize > dodgePlayerY,
  )
  const remaining = hazards.filter(
    ({ y }) => y < dodgePlayerY + dodgePlayerHeight,
  )
  return {
    lane: state.lane,
    elapsed,
    nextWaveAt,
    hazards: remaining,
    score: state.score + hazards.length - remaining.length,
    status: collision ? 'lost' : 'running',
    hit: collision?.kind,
  }
}
