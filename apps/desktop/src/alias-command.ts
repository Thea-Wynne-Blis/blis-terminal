import type { CommandDefinition, CommandResult } from './command-types'

export const aliasDefinition: CommandDefinition = {
  summary: '保存和管理命令别名',
  usage: [
    'alias add <别名> <Blis 命令及参数>',
    'alias list',
    'alias remove <别名>',
  ],
  details: [
    '输入别名时，附加参数会接在原命令后面。别名会保存在本机，重启后仍可使用。',
    '别名支持中文和 Tab 补全。使用内置的 Blis 命令作为目标。',
    '内置命令名已保留。要更改已有别名，请先移除再添加。',
  ],
  examples: [
    'alias add bilibili search bilibili',
    'bilibili csgo',
    'alias list',
    'alias remove bilibili',
  ],
  run: parseAlias,
}

export function aliasKey(name: string): string {
  return name.replace(/[A-Z]/g, (letter) => letter.toLowerCase())
}

function parseAlias(input: string): CommandResult {
  const args = input.trim()
  if (!args || /^list$/iu.test(args))
    return { type: 'alias', operation: { action: 'list' } }

  const [, verb, name = '', target] =
    /^(\S+)\s+(\S+)(?:\s+([\s\S]+))?$/u.exec(args) ?? []
  if (verb?.toLowerCase() === 'add' && target)
    return {
      type: 'alias',
      operation: { action: 'add', name: aliasKey(name), target },
    }
  if (verb?.toLowerCase() === 'remove' && !target)
    return {
      type: 'alias',
      operation: { action: 'remove', name: aliasKey(name) },
    }

  return { type: 'error', text: '用法有误。输入 help alias 查看。' }
}
