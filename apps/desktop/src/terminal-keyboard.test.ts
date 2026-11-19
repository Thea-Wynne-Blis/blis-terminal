import { describe, expect, it } from 'vitest'
import {
  getHistoryStep,
  getOutputShortcut,
  isCommandCompletionKey,
  isEscapeKey,
} from './terminal-keyboard'

function keyEvent(overrides: Partial<KeyboardEvent> = {}) {
  return {
    key: 'Escape',
    defaultPrevented: false,
    repeat: false,
    isComposing: false,
    keyCode: 27,
    ...overrides,
  } as KeyboardEvent
}

describe('Escape for menus', () => {
  it('recognizes a single Escape press', () => {
    expect(isEscapeKey(keyEvent())).toBe(true)
  })

  it.each([
    { key: 'Enter' },
    { isComposing: true },
    { keyCode: 229 },
    { defaultPrevented: true },
    { repeat: true },
  ])('ignores other key events for %o', (overrides) => {
    expect(isEscapeKey(keyEvent(overrides))).toBe(false)
  })
})

describe('Tab command completion', () => {
  it('uses Tab in the command input', () => {
    expect(isCommandCompletionKey(keyEvent({ key: 'Tab' }))).toBe(true)
  })

  it.each([
    { shiftKey: true },
    { ctrlKey: true },
    { altKey: true },
    { metaKey: true },
    { isComposing: true },
    { keyCode: 229 },
    { defaultPrevented: true },
  ])('leaves modified or composing Tab to the WebView for %o', (overrides) => {
    expect(isCommandCompletionKey(keyEvent({ key: 'Tab', ...overrides }))).toBe(
      false,
    )
  })
})

describe('terminal shortcuts', () => {
  it('matches the clear and folding chords while leaving copy and paste to the WebView', () => {
    expect(getOutputShortcut(keyEvent({ key: 'L', ctrlKey: true }))).toBe(
      'clear',
    )
    expect(getOutputShortcut(keyEvent({ key: 'o', ctrlKey: true }))).toBe(
      'toggle-output',
    )
    expect(
      getOutputShortcut(keyEvent({ key: 'v', ctrlKey: true })),
    ).toBeUndefined()
    expect(
      getOutputShortcut(keyEvent({ key: 'c', ctrlKey: true })),
    ).toBeUndefined()
    expect(getOutputShortcut(keyEvent({ key: 'l' }))).toBeUndefined()
  })

  it.each([
    { altKey: true },
    { shiftKey: true },
    { metaKey: true },
    { isComposing: true },
    { keyCode: 229 },
    { defaultPrevented: true },
  ])('leaves unrelated or composed chords alone for %o', (overrides) => {
    expect(
      getOutputShortcut(keyEvent({ key: 'l', ctrlKey: true, ...overrides })),
    ).toBeUndefined()
  })

  it('navigates history with unmodified arrows and allows repeats', () => {
    expect(getHistoryStep(keyEvent({ key: 'ArrowUp', repeat: true }))).toBe(-1)
    expect(getHistoryStep(keyEvent({ key: 'ArrowDown' }))).toBe(1)
    expect(
      getHistoryStep(keyEvent({ key: 'ArrowUp', ctrlKey: true })),
    ).toBeUndefined()
    expect(
      getHistoryStep(keyEvent({ key: 'ArrowUp', isComposing: true })),
    ).toBeUndefined()
  })
})
