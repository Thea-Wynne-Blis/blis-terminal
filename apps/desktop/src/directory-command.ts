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

export const mkdirDefinition: CommandDefinition = {
  summary: '新建目录',
  usage: ['mkdir [--] <目录>'],
  details: ['父目录需要已存在。目标重名时会提示。路径含空格时请加引号。'],
  examples: ['mkdir "项目 备份"', 'mkdir "资料/项目 A"'],
  run: (input) => editFiles('mkdir', input),
}

export const cpDefinition: CommandDefinition = {
  summary: '复制文件或目录',
  usage: ['cp [-r] [--] <源路径> <目标路径>'],
  details: [
    '使用 -r 复制目录及其内容。目标是现有目录时，复制到该目录内。目标重名时会提示。',
    '路径含空格时请加引号。以短横线开头的路径可放在 -- 后面。',
  ],
  examples: ['cp "项目计划.md" "计划 副本.md"', 'cp -r "项目资料" "备份"'],
  run: (input) => editFiles('cp', input),
}

export const mvDefinition: CommandDefinition = {
  summary: '移动或重命名文件、目录',
  usage: ['mv [--] <源路径> <目标路径>'],
  details: [
    '目标是现有目录时，移入该目录。目标重名时会提示。路径含空格时请加引号。',
    '文件支持跨盘移动。移动目录到其他盘时，可先用 cp -r 复制，再回收原目录。',
  ],
  examples: ['mv "计划 草稿.md" "项目计划.md"', 'mv "项目资料" "归档"'],
  run: (input) => editFiles('mv', input),
}

export const rmDefinition: CommandDefinition = {
  summary: '确认后将文件或目录送入回收站',
  usage: ['rm [-r] [--] <路径>'],
  details: [
    '使用 -r 回收目录及其内容。输入后会显示完整路径，按 Esc 或选择取消可保留文件。',
    '回收站不可用时会报告错误并保留目标。路径含空格时请加引号。',
  ],
  examples: ['rm "计划 副本.md"', 'rm -r "项目 备份"'],
  run: (input) => editFiles('rm', input),
}

type FileCommand = 'mkdir' | 'cp' | 'mv' | 'rm'

function editFiles(command: FileCommand, input: string): CommandResult {
  const tokens = tokenizeArguments(input, { preserveBackslashes: true })
  if (!tokens) return directoryError(command, '引号没有闭合。')
  const paths: string[] = []
  let recursive = false
  let afterOptions = false
  for (const { value, literal } of tokens) {
    if (!afterOptions && !literal && value === '--') {
      afterOptions = true
    } else if (
      !afterOptions &&
      !literal &&
      value.startsWith('-') &&
      value !== '-'
    ) {
      if (value !== '-r' || recursive || (command !== 'cp' && command !== 'rm'))
        return directoryError(command, '未知或重复选项「' + value + '」。')
      recursive = true
    } else {
      paths.push(value)
    }
  }

  const arity = command === 'cp' || command === 'mv' ? 2 : 1
  if (paths.length !== arity || paths.some((path) => !path))
    return directoryError(
      command,
      '请提供 ' + arity + ' 个有效路径，含空格时请加引号。',
    )
  const [path, destination] = paths
  switch (command) {
    case 'mkdir':
      return directoryCommand({ action: 'mkdir', path })
    case 'cp':
      return directoryCommand({
        action: 'copy',
        source: path,
        destination,
        recursive,
      })
    case 'mv':
      return directoryCommand({ action: 'move', source: path, destination })
    case 'rm':
      return directoryCommand({ action: 'remove', path, recursive })
  }
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
  command: 'pwd' | 'cd' | 'ls' | FileCommand,
  text: string,
): CommandResult {
  return { type: 'error', text: text + ' 输入 help ' + command + ' 查看用法。' }
}
