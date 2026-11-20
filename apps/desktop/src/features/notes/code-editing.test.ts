import { describe, expect, it } from 'vitest'
import {
  indentEdits,
  newlineEdit,
  pairedBackspace,
  pairEdit,
} from './code-editing'

const caret = (position: number) => ({ from: position, to: position })

describe('code bracket editing', () => {
  it('adds a right bracket and puts the cursor between the pair', () => {
    expect(pairEdit('call', caret(4), '(')).toEqual({
      from: 4,
      to: 4,
      text: '()',
      select: caret(5),
    })
  })

  it('wraps selected code and skips an existing matching closer', () => {
    expect(pairEdit('count', { from: 0, to: 5 }, '[')).toMatchObject({
      text: '[count]',
      select: { from: 1, to: 6 },
    })
    expect(pairEdit('()', caret(1), ')')).toMatchObject({
      text: '',
      select: caret(2),
    })
  })

  it('leaves apostrophes in words and escaped quotes to normal text entry', () => {
    expect(pairEdit('can', caret(3), "'")).toBeUndefined()
    expect(pairEdit('\\', caret(1), '"')).toBeUndefined()
    expect(pairedBackspace('\\""', 2)).toBeUndefined()
    expect(pairedBackspace('[]', 1)).toEqual({ from: 0, to: 2 })
  })
})

describe('code indentation', () => {
  it('starts an indented line between braces and aligns the closing brace', () => {
    expect(newlineEdit('    if (ready) {}', caret(16))).toEqual({
      from: 16,
      to: 16,
      text: '\n        \n    ',
      select: caret(25),
    })
  })

  it('retains indentation and recognizes Python block colons', () => {
    expect(newlineEdit('\tprint(x)', caret(9)).text).toBe('\n\t')
    expect(newlineEdit('    if ready:', caret(13), 'python').text).toBe(
      '\n        ',
    )
    expect(newlineEdit('    # comment:', caret(14), 'python').text).toBe(
      '\n    ',
    )
  })

  it('indents to the next tab stop and adjusts only selected lines', () => {
    expect(indentEdits('  ', caret(2), false)).toEqual([
      { ...caret(2), text: '  ' },
    ])
    const source = '    one\n\tsecond\nthird'
    const range = { from: 2, to: source.indexOf('third') }
    expect(indentEdits(source, range, true)).toEqual([
      { from: 0, to: 4, text: '' },
      { from: 8, to: 9, text: '' },
    ])
    expect(indentEdits(source, range, false)).toEqual([
      { ...caret(0), text: '    ' },
      { ...caret(8), text: '    ' },
    ])
  })
})
