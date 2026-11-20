import { describe, expect, it } from 'vitest'
import { executeTerminalInput } from '../../terminal/terminal-commands'

describe('background', () => {
  it('opens the background picker and registers help', () => {
    expect(executeTerminalInput('background', [])).toEqual({
      type: 'background',
      operation: { action: 'get' },
    })
    expect(executeTerminalInput('help background', [])).toMatchObject({
      type: 'text',
      text: expect.stringContaining('background reset'),
    })
  })

  it('keeps a quoted Windows image path and supports named colors', () => {
    const path = String.raw`C:\My Wallpapers\夜景.jpg`
    expect(
      executeTerminalInput(`background add 夜景 --image "${path}"`, []),
    ).toEqual({
      type: 'background',
      operation: { action: 'add_image', name: '夜景', path },
    })
    expect(
      executeTerminalInput('background add 海蓝 --color #172942', []),
    ).toEqual({
      type: 'background',
      operation: { action: 'add_color', name: '海蓝', color: '#172942' },
    })
  })

  it.each([0, 55, 100])('accepts %i percent opacity', (value) => {
    expect(executeTerminalInput(`background opacity ${value}`, [])).toEqual({
      type: 'background',
      operation: { action: 'opacity', value },
    })
  })

  it.each(['-1', '101', '1.5', 'nan', ''])(
    'rejects opacity outside integer bounds',
    (value) => {
      expect(
        executeTerminalInput(`background opacity ${value}`, []),
      ).toMatchObject({ type: 'error' })
    },
  )
})
