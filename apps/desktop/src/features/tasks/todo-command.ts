import { tokenizeArguments } from '../../terminal/command-arguments'
import type {
  CommandDefinition,
  CommandResult,
} from '../../terminal/command-types'

export const todoDefinition: CommandDefinition = {
  summary: '管理本地待办事项',
  usage: ['todo add <内容>', 'todo list', 'todo done <编号>'],
  details: [
    '待办保存在本机，重启后仍可使用。',
    'todo list 显示未完成的待办和编号。用 todo done 加上编号标记完成。',
  ],
  examples: ['todo add "补发版说明"', 'todo list', 'todo done 3'],
  run(input) {
    const tokens = tokenizeArguments(input, { preserveBackslashes: true })
    if (!tokens) return todoError('引号没有闭合。')
    const [verb, ...args] = tokens.map(({ value }) => value)
    switch (verb?.toLowerCase()) {
      case 'add': {
        const title = args.join(' ').trim()
        if (!title) return todoError('请输入待办内容。')
        return { type: 'todo', operation: { action: 'add', title } }
      }
      case 'list':
        if (args.length === 0)
          return { type: 'todo', operation: { action: 'list' } }
        break
      case 'done': {
        const id = Number(args[0])
        if (
          args.length === 1 &&
          /^\d+$/u.test(args[0]) &&
          id > 0 &&
          id <= 0xffffffff
        )
          return { type: 'todo', operation: { action: 'done', id } }
        return todoError('请输入 todo list 中显示的待办编号。')
      }
    }
    return todoError('用法为 todo add、todo list 或 todo done。')
  },
}

function todoError(message: string): CommandResult {
  return { type: 'error', text: message + ' 输入 help todo 查看用法。' }
}
