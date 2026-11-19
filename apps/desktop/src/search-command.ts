import { tokenizeArguments, type ArgumentToken } from './command-arguments'
import type { CommandDefinition, CommandResult } from './command-types'

const defaultEngine = {
  id: 'baidu',
  name: '百度',
  url: 'https://www.baidu.com/s',
  parameter: 'wd',
}
const engines = [
  defaultEngine,
  {
    id: 'bing',
    name: 'Bing',
    url: 'https://www.bing.com/search',
    parameter: 'q',
  },
  {
    id: 'google',
    name: 'Google',
    url: 'https://www.google.com/search',
    parameter: 'q',
  },
  {
    id: 'github',
    name: 'GitHub',
    url: 'https://github.com/search',
    parameter: 'q',
  },
]
const maxUrlLength = 8192

export const searchDefinition: CommandDefinition = {
  summary: '搜索关键词或打开网址',
  usage: [
    'search <关键词>',
    'search <引擎> <关键词>',
    'search <关键词> --engine <引擎>',
    'search <HTTP 或 HTTPS 网址>',
  ],
  details: [
    `默认使用${defaultEngine.name}。可选引擎为 ${engines.map(({ id }) => id).join('、')}。`,
    '--engine 可简写为 -e，也可用 --engine=bing。用引号包住带空格的参数。',
    '-- 后面的参数作为关键词。结果交给系统默认浏览器打开。',
  ],
  examples: [
    'search Rust 入门教程',
    'search github "vue terminal"',
    'search "Windows 托盘" --engine=bing',
    'search bing -- -example',
    'search https://github.com/liyupi/yuindex',
  ],
  run: searchCommand,
}

function searchError(text: string): CommandResult {
  return { type: 'error', text }
}

function searchCommand(input: string): CommandResult {
  const tokens = tokenizeArguments(input)
  if (!tokens) return searchError('引号没有闭合。输入 help search 查看用法。')

  const keywords: ArgumentToken[] = []
  let engine: (typeof engines)[number] | undefined
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

    const option = /^(?:-e|--engine)(?:=(.*))?$/u.exec(token.value)
    if (!option)
      return searchError(
        `未知选项「${token.value}」。输入 help search 查看用法。`,
      )
    if (engine) return searchError('请只指定一个 --engine。')

    const value = option[1] ?? tokens[index + 1]?.value
    if (!value || value.startsWith('-'))
      return searchError('--engine 需要一个引擎名称。')
    if (option[1] === undefined) index++
    engine = engines.find(({ id }) => id === value.toLowerCase())
    if (!engine)
      return searchError(
        `未知引擎「${value}」。输入 help search 查看支持的引擎。`,
      )
  }

  const first = keywords[0]
  if (!engine && first && !first.literal) {
    engine = engines.find(({ id }) => id === first.value.toLowerCase())
    if (engine) keywords.shift()
  }

  const query = keywords
    .map(({ value }) => value)
    .join(' ')
    .trim()
  if (!query)
    return searchError('请输入关键词或网址。输入 help search 查看用法。')

  let url: URL
  let destination: string
  if (
    !engine &&
    !afterOptions &&
    keywords.length === 1 &&
    /^[a-z][a-z\d+.-]*:/iu.test(query)
  ) {
    if (!/^https?:\/\/[^/\\\s]/iu.test(query))
      return searchError('请输入完整的 HTTP 或 HTTPS 网址。')
    try {
      url = new URL(query)
    } catch {
      return searchError('网址格式有误。')
    }
    if (url.username || url.password)
      return searchError('网址不能包含账号或密码。')
    destination = url.hostname
  } else {
    const provider = engine ?? defaultEngine
    url = new URL(provider.url)
    url.searchParams.set(provider.parameter, query)
    destination = provider.name
  }

  if (url.href.length > maxUrlLength)
    return searchError('网址过长，请缩短输入。')
  return {
    type: 'open-web',
    url: url.href,
    text: `已交给默认浏览器打开 ${destination}。`,
  }
}
