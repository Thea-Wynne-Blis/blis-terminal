import { tokenizeArguments } from '../../terminal/command-arguments'
import type {
  CommandDefinition,
  CommandResult,
} from '../../terminal/command-types'

export const defaultWelcome = '欢迎使用 Blis Terminal'

export const welcomeDefinition: CommandDefinition = {
  summary: '查看或修改终端顶部的欢迎语',
  usage: ['welcome', 'welcome set <欢迎语>', 'welcome reset'],
  details: [
    '欢迎语会立即更新并保存在本机，重启后继续使用。',
    '请输入一行文字，最多 200 个字符。可用引号保留文字中的多个空格。',
  ],
  examples: [
    'welcome',
    'welcome set "欢迎回来，今天也写点好代码！"',
    'welcome reset',
  ],
  run(input) {
    const tokens = tokenizeArguments(input, { preserveBackslashes: true })
    if (!tokens) return welcomeError('引号没有闭合。')
    if (!tokens.length) return { type: 'welcome', operation: { action: 'get' } }
    const [verb, ...words] = tokens.map(({ value }) => value)
    if (verb.toLowerCase() === 'set') {
      const message = words.join(' ').trim()
      if (!message) return welcomeError('请输入欢迎语。')
      return { type: 'welcome', operation: { action: 'set', message } }
    }
    if (verb.toLowerCase() === 'reset' && words.length === 0)
      return { type: 'welcome', operation: { action: 'reset' } }
    return welcomeError('欢迎语命令参数有误。')
  },
}

function welcomeError(message: string): CommandResult {
  return { type: 'error', text: message + ' 输入 help welcome 查看用法。' }
}
