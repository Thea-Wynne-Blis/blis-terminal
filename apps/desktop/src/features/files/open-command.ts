import { tokenizeArguments } from '../../terminal/command-arguments'
import type {
  CommandDefinition,
  CommandResult,
  OpenTargetKind,
} from '../../terminal/command-types'

const kinds = new Map<string, OpenTargetKind>([
  ['--app', 'app'],
  ['--folder', 'folder'],
  ['--file', 'file'],
  ['--url', 'url'],
])

export const openDefinition: CommandDefinition = {
  summary: '保存并打开常用应用、路径和网址',
  usage: [
    'open <别名>',
    'open .',
    'open add <别名> --app <绝对路径.exe>',
    'open add <别名> --folder <绝对路径>',
    'open add <别名> --file <绝对路径>',
    'open add <别名> --url <HTTP 或 HTTPS 网址>',
    'open list',
    'open remove <别名>',
  ],
  details: [
    'open . 用资源管理器打开 Blis 的当前目录。',
    '清单保存在应用配置目录里，重新启动 Blis 后可继续使用。路径含空格时用引号包住。',
    '别名可用中文及字母数字，英文字母不区分大小写。同名记录须先移除再重新添加。',
    'open remove 只移除清单中的记录。',
  ],
  examples: [
    'open .',
    'open add vscode --app "D:/Apps/Microsoft VS Code/Code.exe"',
    'open add 项目 --folder "G:/web-ai-code/blis-terminal"',
    'open add 报告 --file "D:/Documents/报告.pdf"',
    'open add github --url https://github.com',
    'open vscode',
    'open remove github',
  ],
  run: openCommand,
}

function openError(text: string): CommandResult {
  return { type: 'error', text: `${text} 输入 help open 查看用法。` }
}

function openCommand(input: string): CommandResult {
  // Windows drive, UNC and trailing separators are literal in paths.
  const tokens = tokenizeArguments(input, { preserveBackslashes: true })
  if (!tokens) return openError('引号没有闭合。')
  const [verb, alias, option, target] = tokens.map(({ value }) => value)

  switch (verb?.toLowerCase()) {
    case '.':
      if (tokens.length !== 1) return openError('用法为 open .。')
      return { type: 'directory', operation: { action: 'open_current' } }
    case 'list':
      if (tokens.length !== 1) return openError('用法为 open list。')
      return { type: 'open-target', operation: { action: 'list' } }
    case 'remove':
      if (tokens.length !== 2 || !alias)
        return openError('用法为 open remove <别名>。')
      return { type: 'open-target', operation: { action: 'remove', alias } }
    case 'add': {
      const kind = option && kinds.get(option.toLowerCase())
      if (tokens.length !== 4 || !alias || !kind || !target)
        return openError(
          '用法为 open add <别名> <--app|--folder|--file|--url> <目标>。',
        )
      return {
        type: 'open-target',
        operation: { action: 'add', alias, kind, target },
      }
    }
    default:
      if (tokens.length !== 1 || !verb) return openError('用法为 open <别名>。')
      return {
        type: 'open-target',
        operation: { action: 'launch', alias: verb },
      }
  }
}
