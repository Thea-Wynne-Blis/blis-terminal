export type ArgumentToken = { value: string; literal: boolean }

export function tokenizeArguments(
  input: string,
  { preserveBackslashes = false } = {},
): ArgumentToken[] | null {
  const tokens: ArgumentToken[] = []
  let token: ArgumentToken | undefined
  let quote: string | undefined

  for (let index = 0; index < input.length; index++) {
    const character = input[index]
    if (!quote && /\s/u.test(character)) {
      if (token) tokens.push(token)
      token = undefined
      continue
    }

    token ??= { value: '', literal: character === '"' || character === "'" }
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

  if (quote) return null
  if (token) tokens.push(token)
  return tokens
}
