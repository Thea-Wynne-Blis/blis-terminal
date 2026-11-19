export type ArgumentToken = { value: string; literal: boolean }
type ArgumentSpan = ArgumentToken & { start: number; end: number }

export function tokenizeArguments(
  input: string,
  { preserveBackslashes = false } = {},
): ArgumentToken[] | null {
  const { tokens, quote } = scanArguments(input, preserveBackslashes)
  return quote ? null : tokens.map(({ value, literal }) => ({ value, literal }))
}

export function scanArguments(input: string, preserveBackslashes: boolean) {
  const tokens: ArgumentSpan[] = []
  let token: ArgumentSpan | undefined
  let quote: string | undefined

  for (let index = 0; index < input.length; index++) {
    const character = input[index]
    if (!quote && /\s/u.test(character)) {
      if (token) {
        token.end = index
        tokens.push(token)
      }
      token = undefined
      continue
    }

    token ??= {
      value: '',
      literal: character === '"' || character === "'",
      start: index,
      end: input.length,
    }
    const next = input[index + 1]
    if (
      !preserveBackslashes &&
      character === '\\' &&
      next &&
      (next === '\\' ||
        next === quote ||
        (!quote && (next === '"' || next === "'")))
    ) {
      token.value += next
      index++
    } else if (quote) {
      if (character === quote) quote = undefined
      else token.value += character
    } else if (
      character === '"' ||
      (character === "'" && (!token.value || token.value.endsWith('=')))
    ) {
      quote = character
    } else {
      token.value += character
    }
  }

  if (token) tokens.push(token)
  return { tokens, quote }
}
