import { describe, expect, it } from 'vitest'
import {
  completeCommand,
  executeTerminalInput,
  terminalVersion,
} from './terminal-commands'

describe('terminal commands', () => {
  it.each([
    'help',
    'search',
    'open',
    'shortcut',
    'exit',
    'echo',
    'history',
    'clear',
    'version',
  ])('provides help for %s', (name) => {
    const help = executeTerminalInput(`help ${name}`, [])
    expect(help).toMatchObject({
      type: 'text',
      text: expect.stringContaining('用法'),
    })
    expect(help).toMatchObject({ text: expect.stringContaining('示例') })
    expect(executeTerminalInput(`${name} --help`, [])).toEqual(help)
    expect(executeTerminalInput(`${name} -h`, [])).toEqual(help)
  })

  it('accepts mixed-case commands and preserves text as entered', () => {
    expect(executeTerminalInput('  EcHo  你好  <b>shell</b> ', [])).toEqual({
      type: 'text',
      text: '你好  <b>shell</b> ',
    })
  })

  it('provides an explicit exit command', () => {
    expect(executeTerminalInput('EXIT', [])).toEqual({ type: 'exit' })
    expect(executeTerminalInput('exit extra', [])).toMatchObject({
      type: 'error',
    })
  })

  it('lists prior input and supports clearing the screen', () => {
    expect(executeTerminalInput('history', ['help', 'echo hi'])).toEqual({
      type: 'text',
      text: 'help\necho hi',
    })
    expect(executeTerminalInput('clear', [])).toEqual({ type: 'clear' })
  })

  it('lists the keyboard shortcuts and text selection', () => {
    const shortcut = executeTerminalInput('shortcut', [])
    expect(shortcut).toMatchObject({ type: 'text' })
    if (shortcut.type !== 'text') throw new Error('Expected shortcut text')
    for (const key of [
      'Ctrl + L',
      'Ctrl + O',
      'Ctrl + C',
      'Ctrl + V',
      'Tab',
      '↑',
      '↓',
      '鼠标拖选',
    ]) {
      expect(shortcut.text).toContain(key)
    }
  })

  it('reports the application version and explains unknown commands', () => {
    expect(executeTerminalInput('version', [])).toEqual({
      type: 'text',
      text: `Blis Terminal ${terminalVersion}`,
    })
    expect(executeTerminalInput('help', [])).toMatchObject({
      type: 'text',
      text: expect.stringContaining('echo'),
    })
    expect(executeTerminalInput('dir', [])).toMatchObject({
      type: 'error',
      text: expect.stringContaining('help'),
    })
  })
})

describe('command completion', () => {
  it.each([
    ['se', 'search'],
    ['OP', 'open'],
    ['bac', 'background'],
    ['hi', 'history'],
  ])('completes %s and places the caret after a space', (prefix, name) => {
    expect(completeCommand(prefix, prefix.length)).toMatchObject({
      value: name + ' ',
      caret: name.length + 1,
    })
  })

  it('cycles matches and preserves any existing arguments and spaces', () => {
    const first = completeCommand('  s  bilibili csgo', 3)
    expect(first).toMatchObject({ value: '  search  bilibili csgo', caret: 10 })
    if (!first) throw new Error('Expected a first match')
    const next = completeCommand(first.value, first.caret, first)
    expect(next).toMatchObject({
      value: '  shortcut  bilibili csgo',
      caret: 12,
    })
    if (!next) throw new Error('Expected a second match')
    expect(completeCommand(next.value, next.caret, next)).toEqual(first)
    expect(completeCommand('op', 2, next)).toMatchObject({ value: 'open ' })
    expect(completeCommand(next.value, next.value.length, next)).toBeUndefined()
  })

  it.each([
    ['', 0],
    ['xyz', 3],
    ['search bilibili', 15],
    ['sea', 2],
  ])('does not complete %j at the given caret', (value, caret) => {
    expect(completeCommand(value, caret)).toBeUndefined()
  })
})
