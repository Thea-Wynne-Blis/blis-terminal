import { describe, expect, it } from 'vitest'
import { executeTerminalInput } from '../../terminal/terminal-commands'

function search(input: string) {
  return executeTerminalInput('search ' + input, [])
}

describe('search', () => {
  it('passes the first word and keywords to the saved-site resolver', () => {
    expect(search('Bilibili C++ & 中文 #攻略')).toEqual({
      type: 'search',
      operation: {
        action: 'run',
        words: ['Bilibili', 'C++', '&', '中文', '#攻略'],
        inferSite: true,
        allowUrl: true,
      },
    })
  })

  it.each([
    ['-s bilibili -b chrome "csgo 教程"', 'csgo 教程'],
    ['csgo --site=bilibili --browser="chrome"', 'csgo'],
  ])('combines a site and a browser in %s', (input, keyword) => {
    expect(search(input)).toMatchObject({
      type: 'search',
      operation: {
        action: 'run',
        site: 'bilibili',
        browser: 'chrome',
        words: [keyword],
      },
    })
  })

  it('supports direct URLs with a browser override and literal keywords', () => {
    const direct = 'https://example.com/docs?q=a%26b#heading'
    expect(search(direct + ' -b chrome')).toMatchObject({
      operation: { allowUrl: true, words: [direct], browser: 'chrome' },
    })
    expect(search('-- ' + direct)).toMatchObject({
      operation: { allowUrl: false, words: [direct], inferSite: false },
    })
    expect(search('"github"')).toMatchObject({
      operation: { words: ['github'], inferSite: false },
    })
    expect(search("bing -- Rust's -example")).toMatchObject({
      operation: { words: ['bing', "Rust's", '-example'], inferSite: true },
    })
  })

  it.each(['-t', '--template'])('registers a template with %s', (flag) => {
    const template = 'https://example.com/search?type=post&q={query}'
    expect(search('site add custom ' + flag + ' "' + template + '"')).toEqual({
      type: 'search',
      operation: { action: 'site_add', name: 'custom', template },
    })
  })

  it('preserves a quoted Windows browser path', () => {
    const path = 'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe'
    expect(search('browser add chrome --app "' + path + '"')).toEqual({
      type: 'search',
      operation: { action: 'browser_add', name: 'chrome', path },
    })
  })

  it.each([
    ['site list', { action: 'site_list' }],
    ['site remove zhihu', { action: 'site_remove', name: 'zhihu' }],
    ['browser list', { action: 'browser_list' }],
    ['browser default system', { action: 'browser_default', name: 'system' }],
    ['browser remove edge', { action: 'browser_remove', name: 'edge' }],
  ])('dispatches %s', (input, operation) => {
    expect(search(input)).toEqual({ type: 'search', operation })
  })

  it.each([
    ['', '关键词'],
    ['"missing closing quote', '引号'],
    ['--site', '需要'],
    ['-b -s google rust', '浏览器名称'],
    ['-e bing rust', '未知选项'],
    ['--engine bing rust', '未知选项'],
    ['--unknown rust', '未知选项'],
    ['-s bing --site google rust', '只指定一个'],
    ['site add custom --url https://example.com', 'site --help'],
    ['browser default edge chrome', 'browser --help'],
  ])('rejects invalid input %s', (input, feedback) => {
    expect(search(input)).toMatchObject({
      type: 'error',
      text: expect.stringContaining(feedback),
    })
  })

  it('exposes help for search and both subcommands', () => {
    expect(executeTerminalInput('help search', [])).toMatchObject({
      type: 'text',
      text: expect.stringContaining('--site'),
    })
    expect(search('site --help')).toMatchObject({
      type: 'text',
      text: expect.stringContaining('--template'),
    })
    expect(search('browser -h')).toMatchObject({
      type: 'text',
      text: expect.stringContaining('browser default'),
    })
  })
})
