import {
  tokenizeArguments,
  type ArgumentToken,
} from '../../terminal/command-arguments'
import type {
  CommandDefinition,
  CommandResult,
  SearchOperation,
} from '../../terminal/command-types'

const siteUsage = [
  'search site add <站点> -t "<HTTP 或 HTTPS 搜索模板>"',
  'search site list',
  'search site remove <站点>',
]

const browserUsage = [
  'search browser add <浏览器> --app "<绝对路径.exe>"',
  'search browser list',
  'search browser default <浏览器|system>',
  'search browser remove <浏览器>',
]

const siteDetails = [
  '-t 和 --template 都可指定模板。在查询参数中使用一个完整的 {query} 值，例如 ?q={query}。',
  '自定义站点会保存。baidu 是默认搜索站点，始终可用。',
]
const browserDetails = [
  'system 跟随 Windows 默认浏览器。路径含空格时请加引号。',
  'search browser default 修改 Blis 的默认搜索浏览器。-b 或 --browser 仅覆盖当前命令。',
  '自定义浏览器和默认设置会保存，移除当前默认项会恢复为 system。',
]

const siteExample =
  'search site add devdocs --template "https://developer.mozilla.org/en-US/search?q={query}"'
const browserExample =
  'search browser add edge --app "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe"'

export const searchDefinition: CommandDefinition = {
  summary: '按站点搜索、打开网址和管理搜索设置',
  usage: [
    'search [<站点>] <关键词> [-s|--site <站点>] [-b|--browser <浏览器>]',
    'search <HTTP 或 HTTPS 网址> [-b|--browser <浏览器>]',
    'search site --help',
    'search browser --help',
  ],
  details: [
    '首个词匹配已保存的站点时，会在该站点搜索。省略站点时使用百度。',
    '-- 后面的词按字面搜索。用引号包住站点名也可以把它作为关键词。',
    '输入 search site list 或 search browser list 查看可用名称。',
  ],
  examples: [
    'search bilibili csgo',
    'search -s bilibili -b chrome "csgo 教程"',
    'search browser default chrome',
    siteExample,
    'search https://github.com -b chrome',
  ],
  run: searchCommand,
}

function searchError(message: string): CommandResult {
  return { type: 'error', text: message + ' 输入 help search 查看用法。' }
}

function execute(operation: SearchOperation): CommandResult {
  return { type: 'search', operation }
}

function optionParts(option: string): [string, string | undefined] {
  const separator = option.indexOf('=')
  return separator < 0
    ? [option, undefined]
    : [option.slice(0, separator), option.slice(separator + 1)]
}

function searchCommand(input: string): CommandResult {
  // Executable paths can contain literal Windows backslashes.
  const browserManagement = /^browser(?:\s|$)/iu.test(input)
  const tokens = tokenizeArguments(input, {
    preserveBackslashes: browserManagement,
  })
  if (!tokens) return searchError('引号没有闭合。')

  const [first, ...rest] = tokens
  if (first && !first.literal) {
    const group = first.value.toLowerCase()
    if (group === 'site' || group === 'browser') return manage(group, rest)
  }
  return runSearch(tokens)
}

function manage(
  group: 'site' | 'browser',
  tokens: ArgumentToken[],
): CommandResult {
  const [verb, name, option, value] = tokens.map((token) => token.value)
  const site = group === 'site'
  const usage = site ? siteUsage : browserUsage
  if (!verb || (tokens.length === 1 && ['-h', '--help'].includes(verb))) {
    const details = site ? siteDetails : browserDetails
    const example = site ? siteExample : browserExample
    return {
      type: 'text',
      text: [
        '用法',
        ...usage.map((line) => '  ' + line),
        '',
        ...details,
        '',
        '示例',
        '  ' + example,
      ].join('\n'),
    }
  }

  switch (verb.toLowerCase()) {
    case 'list':
      if (tokens.length === 1)
        return execute({ action: site ? 'site_list' : 'browser_list' })
      break
    case 'remove':
      if (tokens.length === 2 && name)
        return execute({
          action: site ? 'site_remove' : 'browser_remove',
          name,
        })
      break
    case 'default':
      if (!site && tokens.length === 2 && name)
        return execute({ action: 'browser_default', name })
      break
    case 'add': {
      const [flag, inline] = optionParts(option ?? '')
      const expected = site ? ['-t', '--template'] : ['--app']
      const target = inline ?? value
      if (
        name &&
        target &&
        expected.includes(flag) &&
        tokens.length === (inline === undefined ? 4 : 3)
      ) {
        return execute(
          site
            ? { action: 'site_add', name, template: target }
            : { action: 'browser_add', name, path: target },
        )
      }
      break
    }
  }
  return {
    type: 'error',
    text: '用法有误。输入 search ' + group + ' --help 查看。',
  }
}

function runSearch(tokens: ArgumentToken[]): CommandResult {
  const keywords: ArgumentToken[] = []
  const options: Pick<
    Extract<SearchOperation, { action: 'run' }>,
    'site' | 'browser'
  > = {}
  let afterOptions = false

  for (let index = 0; index < tokens.length; index++) {
    const token = tokens[index]
    if (
      afterOptions ||
      token.literal ||
      !token.value.startsWith('-') ||
      token.value === '-'
    ) {
      keywords.push(
        afterOptions ? { value: token.value, literal: true } : token,
      )
      continue
    }
    if (token.value === '--') {
      afterOptions = true
      continue
    }

    const [flag, inline] = optionParts(token.value)
    const key =
      flag === '-s' || flag === '--site'
        ? 'site'
        : flag === '-b' || flag === '--browser'
          ? 'browser'
          : undefined
    if (!key) return searchError('未知选项「' + token.value + '」。')
    if (options[key]) return searchError('请只指定一个 --' + key + '。')
    const value = inline ?? tokens[index + 1]?.value
    if (!value || value.startsWith('-'))
      return searchError(
        flag + ' 需要一个' + (key === 'site' ? '站点名称。' : '浏览器名称。'),
      )
    if (inline === undefined) index++
    options[key] = value
  }

  const words = keywords.map(({ value }) => value)
  if (!words.join(' ').trim()) return searchError('请输入关键词或网址。')
  return execute({
    action: 'run',
    ...options,
    words,
    inferSite: !keywords[0]?.literal,
    allowUrl: !afterOptions,
  })
}
