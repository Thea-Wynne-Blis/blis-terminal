import { describe, expect, it } from 'vitest'
import { executeTerminalInput } from '../../terminal/terminal-commands'

describe('open', () => {
  it.each([
    ['app', String.raw`C:\Program Files\App\App.exe`],
    ['folder', 'C:\\'],
    ['folder', String.raw`\\server\shared folder`],
    ['file', 'D:/文档/汇报 1.pdf'],
    ['url', 'https://example.com/?q=a%26b'],
  ])('preserves a %s target as a single value', (kind, target) => {
    expect(
      executeTerminalInput(`open add 常用 --${kind} "${target}"`, []),
    ).toEqual({
      type: 'open-target',
      operation: { action: 'add', alias: '常用', kind, target },
    })
  })

  it('parses launch, list and remove', () => {
    expect(executeTerminalInput('open VSCode', [])).toEqual({
      type: 'open-target',
      operation: { action: 'launch', alias: 'VSCode' },
    })
    expect(executeTerminalInput('open list', [])).toEqual({
      type: 'open-target',
      operation: { action: 'list' },
    })
    expect(executeTerminalInput('open remove 项目', [])).toEqual({
      type: 'open-target',
      operation: { action: 'remove', alias: '项目' },
    })
  })

  it.each([
    'open',
    'open list extra',
    'open remove',
    'open add notes --file',
    'open add notes --folder ""',
    'open add notes --unknown D:/Docs',
    'open add notes --file "D:/Docs',
    'open add notes --file D:/My Docs',
  ])('explains invalid input %s', (input) => {
    expect(executeTerminalInput(input, [])).toMatchObject({
      type: 'error',
      text: expect.stringContaining('help open'),
    })
  })
})
