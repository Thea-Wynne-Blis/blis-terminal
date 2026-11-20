import { tokenizeArguments } from '../../terminal/command-arguments'
import type {
  AppearanceOperation,
  CommandDefinition,
  CommandResult,
} from '../../terminal/command-types'

export const backgroundDefinition: CommandDefinition = {
  summary: '选择、自定义和重置终端背景',
  usage: [
    'background',
    'background add <名称> --image <图片路径>',
    'background add <名称> --color <#RRGGBB>',
    'background remove <名称>',
    'background opacity <0–100>',
    'background reset',
  ],
  details: [
    '在背景菜单用 ↑、↓ 选择，Enter 应用，Esc 关闭。深色和亮色是永久保留的内置背景。',
    '不透明度 0 为完全透明，100 为完全不透明。透明度只作用于背景。',
    '重置恢复深色和 100% 不透明度，保留自定义背景。壁纸支持 PNG、JPEG 和 WebP，单张最大 16 MiB。',
  ],
  examples: [
    'background',
    'background add 夜景 --image "D:/壁纸/夜景.jpg"',
    'background add 海蓝 --color #172942',
    'background opacity 65',
    'background remove 夜景',
  ],
  run: backgroundCommand,
}

function operation(operation: AppearanceOperation): CommandResult {
  return { type: 'background', operation }
}

function usage(): CommandResult {
  return {
    type: 'error',
    text: '背景命令参数有误。输入 help background 查看用法。',
  }
}

function backgroundCommand(input: string): CommandResult {
  const tokens = tokenizeArguments(input, { preserveBackslashes: true })
  if (!tokens) return usage()
  const [verb, name, option, target] = tokens.map(({ value }) => value)
  if (!tokens.length) return operation({ action: 'get' })

  switch (verb.toLowerCase()) {
    case 'reset':
      return tokens.length === 1 ? operation({ action: 'reset' }) : usage()
    case 'remove':
      return tokens.length === 2 && name
        ? operation({ action: 'remove', name })
        : usage()
    case 'opacity':
      if (
        tokens.length === 2 &&
        /^\d{1,3}$/u.test(name) &&
        Number(name) <= 100
      ) {
        return operation({ action: 'opacity', value: Number(name) })
      }
      return { type: 'error', text: '不透明度须是 0 到 100 的整数。' }
    case 'add':
      if (!name) return usage()
      if (tokens.length === 3 && option && !option.startsWith('--')) {
        return operation({ action: 'add_image', name, path: option })
      }
      if (tokens.length !== 4 || !target) return usage()
      if (option === '--image')
        return operation({ action: 'add_image', name, path: target })
      if (option === '--color')
        return operation({ action: 'add_color', name, color: target })
      return usage()
    default:
      return usage()
  }
}
