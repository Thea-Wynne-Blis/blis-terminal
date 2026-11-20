import { describe, expect, it } from 'vitest'
import {
  cyclePathCompletion,
  pathContext,
  startPathCompletion,
} from './path-completion'

describe('path completion', () => {
  it('tracks the active path argument and preserves text after it', () => {
    const line = 'cp -r "旧 项" "目标目录"'
    const caret = line.indexOf('" "')
    const context = pathContext(line, caret)
    if (!context) throw new Error('Expected a path argument')
    expect(context).toMatchObject({ prefix: '旧 项', directoriesOnly: false })
    expect(
      startPathCompletion(context, [{ path: '旧 项目\\', directory: true }]),
    ).toMatchObject({
      value: 'cp -r "旧 项目\\" "目标目录"',
      caret: 'cp -r "旧 项目\\'.length,
    })
  })

  it('finishes files, quotes spaces, and handles ls options', () => {
    const input = 'cp source "..\\文'
    const context = pathContext(input, input.length)
    if (!context) throw new Error('Expected a path argument')
    expect(
      startPathCompletion(context, [
        { path: '..\\文 件.txt', directory: false },
      ]),
    ).toMatchObject({
      value: 'cp source "..\\文 件.txt" ',
    })
    expect(pathContext('ls -al ', 'ls -al '.length)).toMatchObject({
      prefix: '',
      directoriesOnly: true,
      all: true,
    })
    expect(pathContext('ls -a', 'ls -a'.length)).toBeUndefined()
    expect(pathContext('cp a b c', 'cp a b c'.length)).toBeUndefined()
  })

  it('extends shared prefixes and cycles matching names', () => {
    const input = 'rm files/文'
    const context = pathContext(input, input.length)
    if (!context) throw new Error('Expected a path argument')
    const first = startPathCompletion(context, [
      { path: 'files/文件A.txt', directory: false },
      { path: 'files/文件B.txt', directory: false },
    ])
    if (!first) throw new Error('Expected a shared prefix')
    expect(first.value).toBe('rm files/文件')
    const next = cyclePathCompletion(first, first.value, first.caret)
    if (!next) throw new Error('Expected the first match')
    expect(next.value).toBe('rm files/文件A.txt ')
    expect(cyclePathCompletion(next, next.value, next.caret)?.value).toBe(
      'rm files/文件B.txt ',
    )
  })

  it('offers files and directories for the code command', () => {
    const input = 'code .\\会议'
    expect(pathContext(input, input.length)).toMatchObject({
      prefix: '.\\会议',
      directoriesOnly: false,
    })
  })
})
