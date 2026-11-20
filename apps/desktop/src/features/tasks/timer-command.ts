import { tokenizeArguments } from '../../terminal/command-arguments'
import type {
  CommandDefinition,
  CommandResult,
} from '../../terminal/command-types'

const unitSeconds = new Map([
  ['d', 86400],
  ['h', 3600],
  ['m', 60],
  ['s', 1],
])

export const timerDefinition: CommandDefinition = {
  summary: '设置后台倒计时',
  usage: ['timer <时长> [名称]'],
  details: [
    '时长使用正整数加一个单位。d 为天，h 为小时，m 为分钟，s 为秒。',
    'Blis 隐藏在托盘时会继续计时，到点弹出提醒。完全退出 Blis 会结束计时。',
  ],
  examples: ['timer 25m "写代码"', 'timer 10s', 'timer 1h "休息一下"'],
  run(input) {
    const tokens = tokenizeArguments(input, { preserveBackslashes: true })
    if (!tokens) return timerError('引号没有闭合。')
    const [duration, ...name] = tokens.map(({ value }) => value)
    const amount = /^(\d+)([dhms])$/u.exec(duration ?? '')
    if (!amount) return timerError('请输入正整数和 d、h、m、s 中的一个单位。')
    const seconds = Number(amount[1]) * (unitSeconds.get(amount[2]) ?? 0)
    if (!Number.isInteger(seconds) || seconds < 1 || seconds > 0xffffffff)
      return timerError('倒计时长度超出范围。')
    const label = name.join(' ').trim() || '倒计时'
    return { type: 'timer', operation: { seconds, label } }
  },
}

function timerError(message: string): CommandResult {
  return { type: 'error', text: message + ' 输入 help timer 查看用法。' }
}
