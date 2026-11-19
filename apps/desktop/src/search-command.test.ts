import { describe, expect, it } from 'vitest'
import { executeTerminalInput } from './terminal-commands'

function searchUrl(input: string): URL {
  const result = executeTerminalInput(`search ${input}`, [])
  if (result.type !== 'open-web') throw new Error(JSON.stringify(result))
  return new URL(result.url)
}

describe('search', () => {
  it('uses Baidu by default and encodes special characters as keyword text', () => {
    const url = searchUrl('C++ & Rust 中文 #入门')
    expect(url.origin + url.pathname).toBe('https://www.baidu.com/s')
    expect([...url.searchParams.entries()]).toEqual([
      ['wd', 'C++ & Rust 中文 #入门'],
    ])
    expect(url.hash).toBe('')
  })

  it.each([
    ['GitHub "vue terminal"', 'https://github.com/search', 'vue terminal'],
    ['-e bing "Rust 教程"', 'https://www.bing.com/search', 'Rust 教程'],
    ['"两 个词" --engine="google"', 'https://www.google.com/search', '两 个词'],
    [
      "bing -- Rust's -example",
      'https://www.bing.com/search',
      "Rust's -example",
    ],
    ['bing "a \\"quote\\""', 'https://www.bing.com/search', 'a "quote"'],
  ])('parses %s', (input, endpoint, keyword) => {
    const url = searchUrl(input)
    expect(url.origin + url.pathname).toBe(endpoint)
    expect(url.searchParams.get('q')).toBe(keyword)
  })

  it('preserves a direct URL and supports searching for that URL explicitly', () => {
    const direct = 'https://example.com/docs?q=a%26b#heading'
    expect(searchUrl(direct).href).toBe(direct)
    expect(searchUrl(`-- ${direct}`).searchParams.get('wd')).toBe(direct)
    expect(searchUrl('"github"').searchParams.get('wd')).toBe('github')
  })

  it.each([
    ['', '关键词'],
    ['bing', '关键词'],
    ['"missing closing quote', '引号'],
    ['--engine', '需要'],
    ['--engine unknown rust', '未知引擎'],
    ['--unknown rust', '未知选项'],
    ['-e bing --engine google rust', '只指定一个'],
    ['file:///C:/Windows', 'HTTP'],
    ['javascript:alert(1)', 'HTTP'],
    ['https://name:password@example.com', '账号或密码'],
    ['中文'.repeat(500), '过长'],
  ])('rejects invalid input or options', (input, feedback) => {
    expect(executeTerminalInput(`search ${input}`, [])).toMatchObject({
      type: 'error',
      text: expect.stringContaining(feedback),
    })
  })

  it('exposes command-specific help', () => {
    expect(executeTerminalInput('help search', [])).toMatchObject({
      type: 'text',
      text: expect.stringContaining('--engine'),
    })
  })
})
