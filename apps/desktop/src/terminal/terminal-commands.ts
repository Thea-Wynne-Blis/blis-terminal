import { version } from '../../package.json'
import { aliasDefinition, aliasKey } from '../features/tools/alias-command'
import { backgroundDefinition } from '../features/appearance/background-command'
import { calcDefinition } from '../features/tools/calc-command'
import type { CommandDefinition, CommandResult } from './command-types'
import { dateDefinition } from '../features/tools/date-command'
import {
  cdDefinition,
  codeDefinition,
  cpDefinition,
  lsDefinition,
  mkdirDefinition,
  mvDefinition,
  pwdDefinition,
  rmDefinition,
} from '../features/files/directory-command'
import { completeGame, gameDefinition } from '../features/games/game-command'
import { noteDefinition } from '../features/notes/note-command'
import { openDefinition } from '../features/files/open-command'
import { pickDefinition } from '../features/tools/pick-command'
import { searchDefinition } from '../features/search/search-command'
import { describeShortcuts } from './terminal-keyboard'
import { timerDefinition } from '../features/tasks/timer-command'
import { todoDefinition } from '../features/tasks/todo-command'
import { welcomeDefinition } from '../features/appearance/welcome-command'

export const terminalVersion = version
const commandLine = /^(\s*)(\S+)(\s*)([\s\S]*)$/u
const noAliases: ReadonlyMap<string, string> = new Map()

const commands = new Map<string, CommandDefinition>([
  [
    'help',
    {
      summary: '查看命令说明',
      usage: ['help', 'help <命令>'],
      details: ['每条命令也支持单独使用 --help 或 -h 查看说明。'],
      examples: ['help search', 'echo --help'],
      run: showHelp,
    },
  ],
  ['search', searchDefinition],
  ['open', openDefinition],
  ['pwd', pwdDefinition],
  ['cd', cdDefinition],
  ['ls', lsDefinition],
  ['code', codeDefinition],
  ['mkdir', mkdirDefinition],
  ['cp', cpDefinition],
  ['mv', mvDefinition],
  ['rm', rmDefinition],
  ['background', backgroundDefinition],
  ['welcome', welcomeDefinition],
  ['alias', aliasDefinition],
  ['calc', calcDefinition],
  ['date', dateDefinition],
  ['timer', timerDefinition],
  ['pick', pickDefinition],
  ['todo', todoDefinition],
  ['game', gameDefinition],
  ['note', noteDefinition],
  [
    'shortcut',
    {
      summary: '查看终端快捷键',
      usage: ['shortcut'],
      details: ['清屏后仍可用上下键或 history 查看本次会话的历史。'],
      examples: ['shortcut'],
      run: () => ({ type: 'text', text: describeShortcuts() }),
    },
  ],
  [
    'exit',
    {
      summary: '退出 Blis Terminal',
      usage: ['exit'],
      details: ['关闭整个程序，并停止后台托盘和全局快捷键。'],
      examples: ['exit'],
      run: (args) =>
        args.trim()
          ? {
              type: 'error',
              text: 'exit 不需要参数。输入 help exit 查看说明。',
            }
          : { type: 'exit' },
    },
  ],
  [
    'echo',
    {
      summary: '输出后面的文字',
      usage: ['echo <文字>'],
      examples: ['echo 你好，Blis'],
      run: (args) => ({ type: 'text', text: args }),
    },
  ],
  [
    'history',
    {
      summary: '查看本次会话执行过的命令',
      usage: ['history'],
      details: ['可用鼠标拖选和复制历史文本。上下方向键可取回命令继续编辑。'],
      examples: ['history'],
      run: (_args, history) => ({
        type: 'text',
        text: history.join('\n') || '暂无历史命令。',
      }),
    },
  ],
  [
    'clear',
    {
      summary: '清空屏幕上的输出',
      usage: ['clear'],
      details: ['此前输入的命令仍可通过 history 和上下方向键查看。'],
      examples: ['clear'],
      run: () => ({ type: 'clear' }),
    },
  ],
  [
    'version',
    {
      summary: '查看 Blis Terminal 版本',
      usage: ['version'],
      examples: ['version'],
      run: () => ({ type: 'text', text: `Blis Terminal ${terminalVersion}` }),
    },
  ],
])

function showHelp(args: string): CommandResult {
  const name = args.trim().toLowerCase()
  if (!name) {
    const width =
      Math.max(...Array.from(commands.keys(), (key) => key.length)) + 2
    const summaries = Array.from(
      commands,
      ([key, command]) => key.padEnd(width) + command.summary,
    )
    return {
      type: 'text',
      text: [...summaries, '', '输入 help <命令> 查看详细用法。'].join('\n'),
    }
  }

  const command = commands.get(name)
  if (!command) return unknownCommand(name)

  return {
    type: 'text',
    text: [
      `${name}  ${command.summary}`,
      '',
      '用法',
      ...command.usage.map((line) => `  ${line}`),
      ...(command.details ? ['', ...command.details] : []),
      '',
      '示例',
      ...command.examples.map((line) => `  ${line}`),
    ].join('\n'),
  }
}

function unknownCommand(command: string): CommandResult {
  return {
    type: 'error',
    text: `未知命令「${command}」。输入 help 查看可用命令。`,
  }
}

type CompletionOptions = {
  leading: string
  separator: string
  args: string
  names: string[]
}

export type CommandCompletion = {
  value: string
  caret: number
  cycle?: { options: CompletionOptions; index: number }
}

function selectCompletion(
  options: CompletionOptions,
  index: number,
): CommandCompletion {
  const { leading, separator, args, names } = options
  const prefix = leading + names[index] + separator
  return {
    value: prefix + args,
    caret: prefix.length,
    cycle: names.length > 1 ? { options, index } : undefined,
  }
}

export function completeCommand(
  value: string,
  caret: number,
  previous?: CommandCompletion,
  aliases = noAliases,
): CommandCompletion | undefined {
  if (previous?.cycle && previous.value === value && previous.caret === caret) {
    const { options, index } = previous.cycle
    return selectCompletion(options, (index + 1) % options.names.length)
  }

  const game = completeGame(value, caret)
  if (game) return selectCompletion(game, 0)
  const [, leading, word, separator, args] = commandLine.exec(value) ?? []
  if (!word || caret !== leading.length + word.length) return undefined

  const key = aliasKey(word)
  const names = [...new Set([...commands.keys(), ...aliases.keys()])].filter(
    (name) => name.startsWith(key),
  )
  if (names.length) {
    return selectCompletion(
      { leading, separator: separator || ' ', args, names },
      0,
    )
  }
}

export function executeTerminalInput(
  line: string,
  history: readonly string[],
  aliases = noAliases,
): CommandResult {
  let parsed = commandLine.exec(line)
  const name = parsed?.[2] ?? ''
  const target =
    !commands.has(name.toLowerCase()) && aliases.get(aliasKey(name))
  if (target) {
    const args = parsed?.[4]
    parsed = commandLine.exec(args ? target + ' ' + args : target)
  }

  const [, , commandName = '', , args = ''] = parsed ?? []
  const key = commandName.toLowerCase()
  const command = commands.get(key)
  if (!command) return unknownCommand(name)

  if (args.trim() === '--help' || args.trim() === '-h') return showHelp(key)
  const result = command.run(args, history)
  if (result.type === 'alias' && result.operation.action === 'add') {
    const { name, target } = result.operation
    if (commands.has(name.toLowerCase()))
      return {
        type: 'error',
        text: '「' + name + '」是内置命令，请换一个别名。',
      }
    const targetName = commandLine.exec(target)?.[2].toLowerCase()
    if (!targetName || !commands.has(targetName))
      return {
        type: 'error',
        text: '请选择一个内置 Blis 命令作为别名的目标。输入 help 查看。',
      }
  }
  return result
}
