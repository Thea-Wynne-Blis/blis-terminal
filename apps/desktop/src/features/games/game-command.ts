import type { CommandDefinition, GameId } from '../../terminal/command-types'

const games = {
  snake: '贪吃蛇 · 在棋盘上吃食物',
  runner: 'Blis 跑酷 · 跳过 BUG、404 和 TIMEOUT',
  dodge: '躲避掉落物 · 左右移动，避开越来越密的障碍',
} satisfies Record<GameId, string>
const gameNames = Object.keys(games) as GameId[]

export const gameDefinition: CommandDefinition = {
  summary: '在终端输出区玩小游戏',
  usage: ['game list', 'game snake', 'game runner', 'game dodge'],
  details: [
    'snake 用方向键或 WASD 移动，按空格暂停或继续。',
    'runner 用空格或 ↑ 跳跃，按 P 暂停或继续。',
    'dodge 用 ←、→ 或 A、D 移动，按 P 暂停或继续。',
    '隐藏窗口会暂停游戏。结束后按 R 重开，Esc 返回终端，最高纪录保存在本机。',
  ],
  examples: ['game list', 'game runner', 'game dodge'],
  run(input) {
    const command = input.trim().toLowerCase()
    if (!command || command === 'list')
      return {
        type: 'text',
        text: gameNames
          .map((name) => ('game ' + name).padEnd(15) + games[name])
          .join('\n'),
      }
    const game = gameNames.find((name) => name === command)
    return game
      ? { type: 'game', game }
      : {
          type: 'error',
          text: '找不到这个游戏。输入 game list 查看全部游戏，或用 help game 查看用法。',
        }
  },
}

export function completeGame(value: string, caret: number) {
  const match = /^(\s*game\s+)([^\s]*)$/iu.exec(value)
  if (!match || caret !== value.length) return
  const prefix = match[2].toLowerCase()
  const names = ['list', ...gameNames].filter((name) => name.startsWith(prefix))
  if (names.length)
    return { leading: match[1], separator: ' ', args: '', names }
}
