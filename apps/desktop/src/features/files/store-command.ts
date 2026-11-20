import { tokenizeArguments } from '../../terminal/command-arguments'
import type { CommandDefinition } from '../../terminal/command-types'

export const storeDefinition: CommandDefinition = {
  summary: '查看或迁移 Blis 数据目录',
  usage: ['store', 'store set <绝对目录路径>', 'store reset'],
  details: [
    '数据包括命令别名、搜索与打开设置、背景、欢迎语、待办和笔记。',
    '目标必须为空目录或尚未创建的目录。路径带空格时请加引号。',
    '复制并核对成功后立即切换，再删除旧目录中已迁移的数据。',
    'store reset 把数据迁回默认目录，定位文件始终保存在默认目录。',
  ],
  examples: ['store', 'store set "D:\\Blis 数据"', 'store reset'],
  run: (args) => {
    const tokens = tokenizeArguments(args, { preserveBackslashes: true })
    const verb = tokens?.[0]?.value.toLowerCase()
    if (tokens?.length === 0)
      return { type: 'store', operation: { action: 'get' } }
    if (verb === 'reset' && tokens?.length === 1)
      return { type: 'store', operation: { action: 'reset' } }
    if (verb === 'set' && tokens?.length === 2 && tokens[1].value.trim())
      return {
        type: 'store',
        operation: { action: 'set', path: tokens[1].value },
      }
    return {
      type: 'error',
      text: '存储命令或路径有误。输入 help store 查看用法。',
    }
  },
}
