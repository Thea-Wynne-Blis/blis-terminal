import { describe, expect, it } from 'vitest'
import {
  advanceSnake,
  createSnake,
  setSnakePaused,
  snakeColumns,
  snakeRows,
  steerSnake,
} from './snake'

describe('Snake rules', () => {
  it('accepts one legal turn per step and prevents reversing into the neck', () => {
    const initial = createSnake(() => 0)
    expect(steerSnake(initial, 'left')).toBe(initial)
    const turned = steerSnake(initial, 'up')
    expect(steerSnake(turned, 'left')).toBe(turned)
    const advanced = advanceSnake(turned)
    expect(advanced.body[0]).toEqual({ x: 5, y: 6 })
    expect(advanced.direction).toBe('up')
    expect(steerSnake(advanced, 'left').turn).toBe('left')
  })

  it('pauses movement until explicitly resumed', () => {
    const state = createSnake(() => 0)
    const paused = setSnakePaused(state, true)
    expect(advanceSnake(paused)).toBe(paused)
    expect(steerSnake(paused, 'up')).toBe(paused)
    expect(advanceSnake(setSnakePaused(paused, false)).body[0]).toEqual({
      x: 6,
      y: 7,
    })
  })

  it('grows, scores, and places new food in a free cell', () => {
    const state = { ...createSnake(() => 0), food: { x: 6, y: 7 } }
    const advanced = advanceSnake(state, () => 0.5)
    expect(advanced.score).toBe(10)
    expect(advanced.body).toHaveLength(4)
    expect(advanced.food).toBeDefined()
    expect(advanced.body).not.toContainEqual(advanced.food)
  })

  it('ends at the wall or the body but allows the tail cell to vacate', () => {
    const state = createSnake(() => 0)
    const edge = {
      ...state,
      body: [
        { x: snakeColumns - 1, y: 7 },
        { x: snakeColumns - 2, y: 7 },
        { x: snakeColumns - 3, y: 7 },
      ],
    }
    expect(advanceSnake(edge).status).toBe('lost')

    const loop = [
      { x: 2, y: 2 },
      { x: 1, y: 2 },
      { x: 1, y: 3 },
      { x: 2, y: 3 },
    ]
    expect(
      advanceSnake(steerSnake({ ...state, body: loop }, 'down')).status,
    ).toBe('running')
    const longer = { ...state, body: [...loop, { x: 3, y: 3 }] }
    expect(advanceSnake(steerSnake(longer, 'down')).status).toBe('lost')
  })

  it('wins when the last free cell is eaten', () => {
    const cells = Array.from(
      { length: snakeColumns * snakeRows },
      (_, index) => {
        const y = Math.floor(index / snakeColumns)
        const column = index % snakeColumns
        return { x: y % 2 ? snakeColumns - 1 - column : column, y }
      },
    )
    const food = cells.pop()
    const state = {
      ...createSnake(),
      body: cells.reverse(),
      direction: 'left' as const,
      food,
    }
    const advanced = advanceSnake(state)
    expect(advanced.status).toBe('won')
    expect(advanced.food).toBeUndefined()
    expect(advanced.body).toHaveLength(snakeColumns * snakeRows)
  })
})
