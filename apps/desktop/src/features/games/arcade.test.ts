import { describe, expect, it } from 'vitest'
import {
  advanceDodge,
  createDodge,
  dodgeLanes,
  dodgePlayerY,
  moveDodge,
} from './dodge'
import {
  advanceRunner,
  createRunner,
  jumpRunner,
  runnerMaxSpeed,
  runnerObstacles,
  runnerPlayerWidth,
  runnerPlayerX,
  runnerSpacing,
  runnerStartSpeed,
  type RunnerObstacle,
} from './runner'

describe('Runner rules', () => {
  it('starts with a clear runway and accelerates up to the speed limit', () => {
    let state = createRunner()
    for (let frame = 0; frame < 120; frame++)
      state = advanceRunner(state, 1 / 60)
    expect(state.status).toBe('running')
    expect(state.speed).toBeGreaterThan(runnerStartSpeed)
    expect(state.obstacles[0].x).toBeGreaterThan(
      runnerPlayerX + runnerPlayerWidth,
    )
    expect(
      advanceRunner({ ...createRunner(), speed: runnerMaxSpeed }, 1 / 60).speed,
    ).toBe(runnerMaxSpeed)
  })

  it.each(['BUG', '404', 'TIMEOUT'] as const)(
    'gives %s a usable jump window at every tested speed',
    (kind: RunnerObstacle) => {
      for (const speed of [runnerStartSpeed, 185, runnerMaxSpeed]) {
        const timings = Array.from({ length: 81 }, (_, lead) => lead * 2)
        const successful = timings.filter((lead) => {
          let state = jumpRunner({
            ...createRunner(),
            speed,
            nextObstacleAt: Infinity,
            obstacles: [{ x: runnerPlayerX + runnerPlayerWidth + lead, kind }],
          })
          for (
            let frame = 0;
            frame < 120 && state.status === 'running';
            frame++
          )
            state = advanceRunner(state, 1 / 60)
          return state.status === 'running' && state.height === 0
        })
        expect(successful.length).toBeGreaterThanOrEqual(5)
      }
    },
  )

  it('increases the gap at speed and records the obstacle that caused the collision', () => {
    const { width } = runnerObstacles.TIMEOUT
    expect(runnerSpacing(runnerMaxSpeed, width)).toBeGreaterThan(
      runnerSpacing(runnerStartSpeed, width),
    )
    expect(
      runnerSpacing(runnerMaxSpeed, width) - width - runnerPlayerWidth,
    ).toBeGreaterThanOrEqual(runnerMaxSpeed * 1.08)
    const collision = advanceRunner(
      {
        ...createRunner(),
        obstacles: [{ x: runnerPlayerX + runnerPlayerWidth, kind: 'BUG' }],
      },
      1 / 60,
    )
    expect(collision).toMatchObject({ status: 'lost', hit: 'BUG' })
    expect(advanceRunner(collision, 1 / 60)).toBe(collision)
  })
})

describe('Dodge rules', () => {
  it('keeps movement on the board and does not move while paused', () => {
    let state = createDodge()
    for (let step = 0; step < dodgeLanes; step++) state = moveDodge(state, -1)
    expect(state.lane).toBe(0)
    for (let step = 0; step < dodgeLanes; step++) state = moveDodge(state, 1)
    expect(state.lane).toBe(dodgeLanes - 1)
    const paused = { ...state, status: 'paused' as const }
    expect(moveDodge(paused, -1)).toBe(paused)
    expect(advanceDodge(paused, 1 / 60)).toBe(paused)
  })

  it('makes later waves faster and closer without occupying more than two lanes', () => {
    const early = advanceDodge(
      { ...createDodge(), nextWaveAt: 0 },
      1 / 60,
      () => 0,
    )
    const late = advanceDodge(
      { ...createDodge(), elapsed: 30, nextWaveAt: 30 },
      1 / 60,
      () => 0,
    )
    expect(early.hazards).toHaveLength(1)
    expect(late.hazards).toHaveLength(2)
    expect(new Set(late.hazards.map(({ lane }) => lane)).size).toBe(2)
    expect(late.nextWaveAt - 30).toBeLessThan(early.nextWaveAt)
    expect(advanceDodge(late, 1 / 60).hazards[0].y).toBeGreaterThan(
      advanceDodge(early, 1 / 60).hazards[0].y,
    )
  })

  it('scores a missed hazard once and ends on impact', () => {
    const hazard = { lane: 3, y: dodgePlayerY - 20, kind: 'ERR' as const }
    const state = { ...createDodge(), hazards: [hazard] }
    expect(advanceDodge(state, 1 / 60)).toMatchObject({
      status: 'lost',
      hit: 'ERR',
    })
    let safelyPassed = moveDodge(state, 1)
    for (let frame = 0; frame < 60; frame++)
      safelyPassed = advanceDodge(
        { ...safelyPassed, nextWaveAt: Infinity },
        1 / 60,
      )
    expect(safelyPassed).toMatchObject({
      status: 'running',
      score: 1,
      hazards: [],
    })
  })
})
