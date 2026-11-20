const indentWidth = 4
const pairs = new Map([
  ['(', ')'],
  ['[', ']'],
  ['{', '}'],
  ['"', '"'],
  ["'", "'"],
  ['\x60', '\x60'],
])
const closing = new Set(pairs.values())
const quoted = new Set(['"', "'", '\x60'])
const colonIndented = new Set(['python', 'py', 'yaml', 'yml'])

export function isPairKey(typed: string) {
  return pairs.has(typed) || closing.has(typed)
}

type Range = { from: number; to: number }
type Replacement = Range & { text: string }
type TextEdit = Replacement & { select: Range }

function lineStart(source: string, at: number): number {
  return at === 0 ? 0 : source.lastIndexOf('\n', at - 1) + 1
}

function isEscaped(source: string, at: number): boolean {
  let before = at
  while (source[before - 1] === '\\') before--
  return (at - before) % 2 === 1
}

export function pairEdit(
  source: string,
  { from, to }: Range,
  typed: string,
): TextEdit | undefined {
  const close = pairs.get(typed)
  if (from !== to) {
    if (!close) return
    return {
      from,
      to,
      text: typed + source.slice(from, to) + close,
      select: { from: from + 1, to: to + 1 },
    }
  }
  const next = source[from] ?? ''
  if (quoted.has(typed) && isEscaped(source, from)) return
  if (closing.has(typed) && next === typed) {
    return { from, to, text: '', select: { from: from + 1, to: from + 1 } }
  }
  if (!close || (next && !/^[\s)\]};,.:]$/u.test(next))) return
  if (quoted.has(typed) && /[\p{L}\p{N}_]/u.test(source[from - 1] ?? '')) return
  return {
    from,
    to,
    text: typed + close,
    select: { from: from + 1, to: from + 1 },
  }
}

export function pairedBackspace(source: string, at: number): Range | undefined {
  const open = source[at - 1]
  const close = source[at]
  if (
    open &&
    close &&
    pairs.get(open) === close &&
    !(quoted.has(open) && isEscaped(source, at - 1))
  ) {
    return { from: at - 1, to: at + 1 }
  }
}

export function newlineEdit(
  source: string,
  { from, to }: Range,
  language = '',
): TextEdit {
  const before = source.slice(lineStart(source, from), from)
  const base = /^[\t ]*/u.exec(before)?.[0] ?? ''
  const body = before.trimEnd()
  const opener = body.at(-1) ?? ''
  const paired = opener === '{' || opener === '[' || opener === '('
  const colonBlock =
    opener === ':' &&
    colonIndented.has(language.toLowerCase()) &&
    !body.trimStart().startsWith('#')
  const indent = base + (paired || colonBlock ? ' '.repeat(indentWidth) : '')
  const rightSide = /^[\t ]*/u.exec(source.slice(to))?.[0] ?? ''
  const samePair =
    from === to && paired && source[to + rightSide.length] === pairs.get(opener)
  const caret = from + 1 + indent.length
  return {
    from,
    to: samePair ? to + rightSide.length : to,
    text: '\n' + indent + (samePair ? '\n' + base : ''),
    select: { from: caret, to: caret },
  }
}

export function indentEdits(
  source: string,
  { from, to }: Range,
  outdent: boolean,
): Replacement[] {
  const first = lineStart(source, from)
  if (!outdent && from === to) {
    let column = 0
    for (const character of source.slice(first, from)) {
      column += character === '\t' ? indentWidth - (column % indentWidth) : 1
    }
    return [
      { from, to, text: ' '.repeat(indentWidth - (column % indentWidth)) },
    ]
  }

  const last = lineStart(source, Math.max(from, to - 1))
  const edits: Replacement[] = []
  let start = first
  while (start <= last) {
    if (outdent) {
      if (source[start] === '\t') {
        edits.push({ from: start, to: start + 1, text: '' })
      } else {
        const spaces = /^ +/u.exec(
          source.slice(start, start + indentWidth),
        )?.[0].length
        if (spaces) edits.push({ from: start, to: start + spaces, text: '' })
      }
    } else {
      edits.push({ from: start, to: start, text: ' '.repeat(indentWidth) })
    }
    const end = source.indexOf('\n', start)
    if (end === -1) break
    start = end + 1
  }
  return edits
}
