import type { CommandDefinition, CommandResult } from './command-types'

export const calcDefinition: CommandDefinition = {
  summary: '计算数学表达式',
  usage: ['calc <表达式>'],
  details: [
    '支持加减乘除、括号、正负号和小数。小数结果最多显示 15 位有效数字。',
  ],
  examples: ['calc 12 + 3 * (8 - 2)', 'calc -2.5 / 2'],
  run: calculate,
}

function calculate(input: string): CommandResult {
  if (!input.trim())
    return { type: 'error', text: '请输入表达式。输入 help calc 查看用法。' }

  const tokens = input.match(/\d+(?:\.\d*)?|\.\d+|[^\s]/gu) ?? []
  let index = 0

  function expression(depth: number): number {
    let result = product(depth)
    while (tokens[index] === '+' || tokens[index] === '-') {
      const operator = tokens[index++]
      const next = product(depth)
      result = finite(operator === '+' ? result + next : result - next)
    }
    return result
  }

  function product(depth: number): number {
    let result = operand(depth)
    while (tokens[index] === '*' || tokens[index] === '/') {
      const operator = tokens[index++]
      const next = operand(depth)
      if (operator === '/' && next === 0) throw new Error('除数不能为 0。')
      result = finite(operator === '*' ? result * next : result / next)
    }
    return result
  }

  function operand(depth: number): number {
    let sign = 1
    while (tokens[index] === '+' || tokens[index] === '-') {
      if (tokens[index++] === '-') sign = -sign
    }
    const token = tokens[index++]
    if (token === '(') {
      if (depth >= 64) throw new Error('括号嵌套过深。')
      const value = expression(depth + 1)
      if (tokens[index++] !== ')') throw new Error('需要一个右括号。')
      return sign * value
    }
    if (!token || !/^\d+(?:\.\d*)?$|^\.\d+$/u.test(token))
      throw new Error('此处需要数字或左括号。')
    return finite(sign * Number(token))
  }

  try {
    const result = expression(0)
    if (index < tokens.length)
      return { type: 'error', text: '无法解析后续内容。请检查运算符和括号。' }
    const formatted = Number.isSafeInteger(result)
      ? result
      : Number(result.toPrecision(15))
    return { type: 'text', text: String(formatted) }
  } catch (error) {
    if (!(error instanceof Error)) throw error
    return { type: 'error', text: error.message }
  }
}

function finite(result: number): number {
  if (!Number.isFinite(result)) throw new Error('计算结果超出支持的范围。')
  return result
}
