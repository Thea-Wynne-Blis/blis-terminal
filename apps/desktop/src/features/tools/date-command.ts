import type {
  CommandDefinition,
  CommandResult,
} from '../../terminal/command-types'

const weekdays = ['日', '一', '二', '三', '四', '五', '六']

export const dateDefinition: CommandDefinition = {
  summary: '查看日期、时间和时区',
  usage: ['date [-u|--utc] [--iso]'],
  details: [
    '默认显示本机时间。-u 显示 UTC。--iso 输出 ISO 8601 格式，可和 -u 一起使用。',
  ],
  examples: ['date', 'date -u', 'date --iso', 'date -u --iso'],
  run: showDate,
}

function showDate(input: string): CommandResult {
  const flags = input.trim() ? input.trim().split(/\s+/u) : []
  const invalid = flags.find((flag) => !['-u', '--utc', '--iso'].includes(flag))
  if (invalid)
    return {
      type: 'error',
      text: '未知选项「' + invalid + '」。输入 help date 查看用法。',
    }

  const utc = flags.includes('-u') || flags.includes('--utc')
  const iso = flags.includes('--iso')
  const now = new Date()
  if (utc && iso) return { type: 'text', text: now.toISOString() }

  const offset = utc ? 0 : -now.getTimezoneOffset()
  const local = new Date(now.getTime() + offset * 60_000)
  const [date, time] = local.toISOString().slice(0, 19).split('T')
  const minutes = Math.abs(offset)
  const hoursPart = String(Math.floor(minutes / 60)).padStart(2, '0')
  const minutesPart = String(minutes % 60).padStart(2, '0')
  const zone = (offset < 0 ? '-' : '+') + hoursPart + ':' + minutesPart
  const timezone = utc ? 'UTC' : 'UTC' + zone
  const weekday = weekdays[local.getUTCDay()]
  return {
    type: 'text',
    text: iso
      ? date + 'T' + time + zone
      : date + ' 星期' + weekday + ' ' + time + ' ' + timezone,
  }
}
