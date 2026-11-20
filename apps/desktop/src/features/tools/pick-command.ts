import { tokenizeArguments } from '../../terminal/command-arguments'
import type { CommandDefinition } from '../../terminal/command-types'

export const pickDefinition: CommandDefinition = {
  summary: '从给定选项里随机选一个',
  usage: ['pick <选项一> <选项二> [更多选项]'],
  details: ['至少提供两个选项。一个选项含空格时请加引号。'],
  examples: ['pick 火锅 烧烤 拉面', 'pick "牛肉 米线" "番茄 意面"'],
  run(input) {
    const options = tokenizeArguments(input, { preserveBackslashes: true })
    if (
      !options ||
      options.length < 2 ||
      options.some(({ value }) => !value.trim())
    )
      return {
        type: 'error',
        text: '请提供至少两个有效选项，含空格时请加引号。输入 help pick 查看用法。',
      }
    return {
      type: 'text',
      text:
        '随机选中「' +
        options[Math.floor(Math.random() * options.length)].value +
        '」。',
    }
  },
}
