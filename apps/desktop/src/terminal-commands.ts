import { version } from '../package.json'
import { backgroundDefinition } from './background-command'
import type { CommandDefinition, CommandResult } from './command-types'
import { openDefinition } from './open-command'
import { searchDefinition } from './search-command'
import { describeShortcuts } from './terminal-keyboard'

export const terminalVersion = version

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
  ['background', backgroundDefinition],
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

export function executeTerminalInput(
  line: string,
  history: readonly string[],
): CommandResult {
  const [, name = '', args = ''] =
    /^\s*(\S+)(?:\s+([\s\S]*))?$/.exec(line) ?? []
  const key = name.toLowerCase()
  const command = commands.get(key)
  if (!command) return unknownCommand(name)

  if (args.trim() === '--help' || args.trim() === '-h') return showHelp(key)
  return command.run(args, history)
}
