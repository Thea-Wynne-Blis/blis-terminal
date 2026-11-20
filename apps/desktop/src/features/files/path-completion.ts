import { scanArguments } from '../../terminal/command-arguments'
import type { PathCandidate } from '../../terminal/command-types'

const pathSlots = new Map<
  string,
  { count: number; directoriesOnly: boolean; options?: boolean }
>([
  ['cd', { count: 1, directoriesOnly: true }],
  ['ls', { count: 1, directoriesOnly: true, options: true }],
  ['code', { count: 1, directoriesOnly: false }],
  ['cat', { count: 1, directoriesOnly: false }],
  ['vim', { count: 1, directoriesOnly: false }],
  ['mkdir', { count: 1, directoriesOnly: true, options: true }],
  ['cp', { count: 2, directoriesOnly: false, options: true }],
  ['mv', { count: 2, directoriesOnly: false, options: true }],
  ['rm', { count: 1, directoriesOnly: false, options: true }],
])

type PathContext = {
  line: string
  start: number
  end: number
  prefix: string
  quote?: string
  directoriesOnly: boolean
  all: boolean
}

export type PathCompletion = {
  value: string
  caret: number
  cycle?: { context: PathContext; candidates: PathCandidate[]; index: number }
}

export function pathContext(
  line: string,
  caret: number,
): PathContext | undefined {
  const head = /^\s*(\S+)\s/u.exec(line)
  if (!head || caret < head[0].length) return
  const command = head[1].toLowerCase()
  const slots = pathSlots.get(command)
  if (!slots) return

  const offset = head[0].length
  const cursor = caret - offset
  const args = line.slice(offset)
  const { tokens } = scanArguments(args, true)
  const token = tokens.find(
    ({ start, end }) => start <= cursor && cursor <= end,
  )
  const raw = token ? args.slice(token.start, token.end) : ''
  const quote = raw[0] === '"' || raw[0] === "'" ? raw[0] : undefined
  if (
    token &&
    cursor !== token.end &&
    !(quote && raw.endsWith(quote) && cursor === token.end - 1)
  )
    return

  let supplied = 0
  let afterOptions = false
  let all = false
  for (const earlier of tokens) {
    if (earlier === token || earlier.start >= cursor) break
    if (
      slots.options &&
      !afterOptions &&
      !earlier.literal &&
      earlier.value.startsWith('-') &&
      earlier.value !== '-'
    ) {
      if (earlier.value === '--') afterOptions = true
      else if (command === 'ls' && /^-[al]+$/u.test(earlier.value))
        all ||= earlier.value.includes('a')
    } else {
      supplied++
    }
  }
  if (supplied >= slots.count) return
  if (
    slots.options &&
    !afterOptions &&
    token &&
    !token.literal &&
    token.value.startsWith('-')
  )
    return

  return {
    line,
    start: token ? offset + token.start : caret,
    end: token ? offset + token.end : caret,
    prefix: token?.value ?? '',
    quote,
    directoriesOnly: slots.directoriesOnly,
    all,
  }
}

function render(
  context: PathContext,
  path: string,
  completeFile: boolean,
): PathCompletion {
  const { line, start, end } = context
  let quote = context.quote
  if (quote && path.includes(quote)) quote = undefined
  if (!quote && (/[\s"']/u.test(path) || path.startsWith('-')))
    quote = path.includes('"') ? "'" : '"'
  const token = quote ? quote + path + quote : path
  const after = line.slice(end)
  const space = completeFile && !after ? ' ' : ''
  return {
    value: line.slice(0, start) + token + space + after,
    caret:
      start + token.length + space.length - (quote && !completeFile ? 1 : 0),
  }
}

function select(
  context: PathContext,
  candidates: PathCandidate[],
  index: number,
): PathCompletion {
  const candidate = candidates[index]
  return {
    ...render(context, candidate.path, !candidate.directory),
    cycle: candidates.length > 1 ? { context, candidates, index } : undefined,
  }
}

export function startPathCompletion(
  context: PathContext,
  candidates: PathCandidate[],
): PathCompletion | undefined {
  const usable = candidates.filter(
    ({ path }) => !path.includes('"') || !path.includes("'"),
  )
  if (!usable.length) return
  if (usable.length === 1) return select(context, usable, 0)

  const first = usable[0].path
  let sharedLength = first.length
  for (
    let i = 1;
    i < usable.length && sharedLength > context.prefix.length;
    i++
  ) {
    const { path } = usable[i]
    let index = 0
    while (index < sharedLength) {
      const character = first.codePointAt(index)
      if (character === undefined || character !== path.codePointAt(index))
        break
      index += character > 0xffff ? 2 : 1
    }
    sharedLength = index
  }
  if (sharedLength > context.prefix.length) {
    return {
      ...render(context, first.slice(0, sharedLength), false),
      cycle: { context, candidates: usable, index: -1 },
    }
  }
  return select(context, usable, 0)
}

export function cyclePathCompletion(
  previous: PathCompletion | undefined,
  value: string,
  caret: number,
): PathCompletion | undefined {
  if (!previous?.cycle || previous.value !== value || previous.caret !== caret)
    return
  const { context, candidates, index } = previous.cycle
  return select(context, candidates, (index + 1) % candidates.length)
}
