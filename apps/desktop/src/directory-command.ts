import { tokenizeArguments } from './command-arguments'
import type {
  CommandDefinition,
  CommandResult,
  DirectoryOperation,
} from './command-types'

export const pwdDefinition: CommandDefinition = {
  summary: '显示当前目录的绝对路径',
  usage: ['pwd'],
  examples: ['pwd'],
  run: (input) =>
    input.trim()
      ? directoryError('pwd', 'pwd 不需要参数。')
      : directoryCommand({ action: 'pwd' }),
}

export const cdDefinition: CommandDefinition = {
  summary: '切换当前目录',
  usage: ['cd [目录]'],
  details: [
    '省略目录或输入 cd ~ 可回到当前 Windows 用户目录。用 cd .. 返回上一级。',
    '可用跨盘绝对路径。目录名含空格时请加引号，支持反斜杠和正斜杠。',
  ],
  examples: ['cd ..', 'cd ~', 'cd "C:\\Program Files"', 'cd G:/'],
  run: changeDirectory,
}

export const lsDefinition: CommandDefinition = {
  summary: '列出目录中的文件和子目录',
  usage: ['ls [目录]'],
  details: [
    '默认列出当前目录。可传入相对路径或绝对路径，目录名含空格时请加引号。',
    '子目录以 / 结尾，隐藏项目默认省略。',
  ],
  examples: ['ls', 'ls ..', 'ls "C:\\Program Files"'],
  run: list,
}

function changeDirectory(input: string): CommandResult {
  const tokens = tokenizeArguments(input, { preserveBackslashes: true })
  if (!tokens) return directoryError('cd', '引号没有闭合。')
  if (tokens.length > 1)
    return directoryError('cd', '请输入一个目录，含空格时请加引号。')
  const path = tokens[0]?.value
  if (path === '') return directoryError('cd', '请输入有效的目录路径。')
  return directoryCommand(
    path === undefined ? { action: 'cd' } : { action: 'cd', path },
  )
}

function list(input: string): CommandResult {
  const tokens = tokenizeArguments(input, { preserveBackslashes: true })
  if (!tokens) return directoryError('ls', '引号没有闭合。')
  if (tokens.length > 1)
    return directoryError('ls', '请输入一个目录，含空格时请加引号。')
  const path = tokens[0]?.value
  if (path === '') return directoryError('ls', '请输入有效的目录路径。')
  return directoryCommand(
    path === undefined ? { action: 'ls' } : { action: 'ls', path },
  )
}

function directoryCommand(operation: DirectoryOperation): CommandResult {
  return { type: 'directory', operation }
}

function directoryError(
  command: 'pwd' | 'cd' | 'ls',
  text: string,
): CommandResult {
  return { type: 'error', text: text + ' 输入 help ' + command + ' 查看用法。' }
}
