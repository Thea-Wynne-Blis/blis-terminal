import { describe, expect, it, vi } from 'vitest'
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
    'pwd',
    'cd',
    'ls',
    'mkdir',
    'cp',
    'mv',
    'rm',
    'alias',
    'calc',
    'date',
    'note',
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

describe('directory commands', () => {
  it.each([
    ['pwd', { action: 'pwd' }],
    ['cd', { action: 'cd' }],
    ['cd "G:\\项目 文件"', { action: 'cd', path: 'G:\\项目 文件' }],
    ['cd ..', { action: 'cd', path: '..' }],
    ['cd ~', { action: 'cd', path: '~' }],
    ['ls', { action: 'ls' }],
    ['ls -al', { action: 'ls', all: true, detailed: true }],
    [
      'ls -l -a "G:\\项目 文件"',
      { action: 'ls', path: 'G:\\项目 文件', all: true, detailed: true },
    ],
    ['ls -a -- -目录', { action: 'ls', path: '-目录', all: true }],
    ['ls "..\\项目 文件"', { action: 'ls', path: '..\\项目 文件' }],
    ['open .', { action: 'open_current' }],
  ])('routes %s to the directory session', (input, operation) => {
    expect(executeTerminalInput(input, [])).toEqual({
      type: 'directory',
      operation,
    })
  })

  it.each([
    'pwd extra',
    'cd Program Files',
    'ls "missing quote',
    'open . extra',
  ])('rejects invalid input %s', (input) =>
    expect(executeTerminalInput(input, [])).toMatchObject({ type: 'error' }),
  )

  it.each([
    ['pw', 'pwd'],
    ['cd', 'cd'],
    ['l', 'ls'],
  ])('completes %s', (prefix, name) => {
    expect(completeCommand(prefix, prefix.length)).toMatchObject({
      value: name + ' ',
    })
  })
})

describe('file commands', () => {
  it.each([
    ['mkdir "项目 备份"', { action: 'mkdir', path: '项目 备份' }],
    [
      'cp -r ".\\项目 A" "备份 B"',
      {
        action: 'copy',
        source: '.\\项目 A',
        destination: '备份 B',
        recursive: true,
      },
    ],
    [
      'mv "草稿 A.md" "完成 B.md"',
      { action: 'move', source: '草稿 A.md', destination: '完成 B.md' },
    ],
    ['rm -r "备份 B"', { action: 'remove', path: '备份 B', recursive: true }],
    [
      'rm -- -文件.txt',
      { action: 'remove', path: '-文件.txt', recursive: false },
    ],
  ])('parses %s', (input, operation) => {
    expect(executeTerminalInput(input, [])).toEqual({
      type: 'directory',
      operation,
    })
  })

  it.each([
    'rm --force test',
    'cp missing-destination',
    'mv -r x y',
    'mkdir ""',
  ])('rejects %s', (input) =>
    expect(executeTerminalInput(input, [])).toMatchObject({ type: 'error' }),
  )

  it.each([
    ['mk', 'mkdir'],
    ['cp', 'cp'],
    ['mv', 'mv'],
    ['r', 'rm'],
  ])('completes %s', (prefix, name) =>
    expect(completeCommand(prefix, prefix.length)).toMatchObject({
      value: name + ' ',
    }),
  )
})

describe('utility commands', () => {
  it.each([
    ['calc 12 + 3 * (8 - 2)', '30'],
    ['calc -(2.5 + .5) / 2', '-1.5'],
    ['calc 0.1 + 0.2', '0.3'],
  ])('evaluates %s', (input, text) => {
    expect(executeTerminalInput(input, [])).toEqual({ type: 'text', text })
  })

  it.each(['calc 1 / 0', 'calc (5 + 2', 'calc 2 * * 3', 'calc console.log(1)'])(
    'rejects %s',
    (input) =>
      expect(executeTerminalInput(input, [])).toMatchObject({ type: 'error' }),
  )

  it('formats UTC and local ISO with the actual timezone offset', () => {
    vi.useFakeTimers()
    try {
      vi.setSystemTime(new Date('2026-11-19T23:04:05Z'))
      expect(executeTerminalInput('date -u', [])).toEqual({
        type: 'text',
        text: '2026-11-19 星期四 23:04:05 UTC',
      })
      expect(executeTerminalInput('date --utc --iso', [])).toEqual({
        type: 'text',
        text: '2026-11-19T23:04:05.000Z',
      })
      const local = executeTerminalInput('date --iso', [])
      if (local.type !== 'text') throw new Error('Expected ISO output')
      expect(local.text).toMatch(/[+-]\d{2}:\d{2}$/u)
      expect(new Date(local.text).getTime()).toBe(Date.now())
      expect(executeTerminalInput('date --bad', [])).toMatchObject({
        type: 'error',
      })
    } finally {
      vi.useRealTimers()
    }
  })

  it.each(['-o', '--out'])('parses Windows export paths with %s', (flag) => {
    expect(executeTerminalInput('note new "项目 计划"', [])).toEqual({
      type: 'note',
      operation: { action: 'new', title: '项目 计划' },
    })
    const output = 'D:\\笔记 备份\\项目计划.txt'
    expect(
      executeTerminalInput(
        'note export "项目 计划" --format txt ' + flag + ' "' + output + '"',
        [],
      ),
    ).toEqual({
      type: 'note',
      operation: {
        action: 'export',
        title: '项目 计划',
        format: 'txt',
        output,
      },
    })
    expect(executeTerminalInput('note export test -f pdf', [])).toMatchObject({
      type: 'error',
    })
  })

  it('requests note removal without granting confirmation', () => {
    expect(executeTerminalInput('note remove "项目 计划"', [])).toEqual({
      type: 'note',
      operation: { action: 'remove', title: '项目 计划' },
    })
    expect(
      executeTerminalInput('note remove "项目 计划" --force', []),
    ).toMatchObject({
      type: 'error',
    })
    expect(executeTerminalInput('help note', [])).toMatchObject({
      type: 'text',
      text: expect.stringContaining('note remove'),
    })
  })

  it.each([
    ['ca', 'calc'],
    ['da', 'date'],
    ['no', 'note'],
  ])('completes %s', (prefix, name) =>
    expect(completeCommand(prefix, prefix.length)).toMatchObject({
      value: name + ' ',
    }),
  )
})

describe('command aliases', () => {
  const aliases = new Map([
    ['bilibili', 'search bilibili'],
    ['项目', 'open workspace'],
    ['问候', 'echo 你好'],
  ])

  it('keeps quoted arguments and backslashes when saving the target', () => {
    const command = 'open add docs --folder "C:\\My Files\\docs"'
    expect(executeTerminalInput('alias add DOCS ' + command, [])).toEqual({
      type: 'alias',
      operation: { action: 'add', name: 'docs', target: command },
    })
  })

  it('expands aliases through the existing dispatcher and appends arguments', () => {
    expect(
      executeTerminalInput('BILIBILI "csgo 中文" -b chrome', [], aliases),
    ).toEqual(executeTerminalInput('search bilibili "csgo 中文" -b chrome', []))
    expect(executeTerminalInput('项目', [], aliases)).toEqual(
      executeTerminalInput('open workspace', []),
    )
    expect(executeTerminalInput('问候  世界', [], aliases)).toEqual({
      type: 'text',
      text: '你好 世界',
    })
    expect(completeCommand('bi', 2, undefined, aliases)).toMatchObject({
      value: 'bilibili ',
      caret: 9,
    })
  })

  it('lists and removes aliases and reserves built-in command names', () => {
    expect(executeTerminalInput('alias list', [])).toEqual({
      type: 'alias',
      operation: { action: 'list' },
    })
    expect(executeTerminalInput('alias remove BILIBILI', [])).toEqual({
      type: 'alias',
      operation: { action: 'remove', name: 'bilibili' },
    })
    for (const input of [
      'alias add open echo hello',
      'alias add cycle bilibili',
      'alias add missing',
      'alias remove bilibili extra',
    ]) {
      expect(executeTerminalInput(input, [], aliases)).toMatchObject({
        type: 'error',
      })
    }
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
