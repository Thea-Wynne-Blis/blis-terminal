import { tokenizeArguments } from '../../terminal/command-arguments'
import type { CommandDefinition } from '../../terminal/command-types'

const usageError = {
  type: 'error' as const,
  text: '请输入一个主机名或 IP，次数为 1–10。输入 help ping 查看用法。',
}

export const pingDefinition: CommandDefinition = {
  summary: '检查一个主机的网络连通性',
  usage: ['ping <主机名或 IP> [-c 次数]', 'ping -n <次数> <主机名或 IP>'],
  details: ['支持 IPv4、IPv6 和域名。默认发送 4 次，每次最多等待 1 秒。'],
  examples: ['ping 127.0.0.1', 'ping example.com -c 3'],
  run: (args) => {
    const tokens = tokenizeArguments(args)
    if (!tokens) return usageError
    let host: string | undefined
    let count: number | undefined
    for (let index = 0; index < tokens.length; index++) {
      const { value } = tokens[index]
      if (value === '-c' || value === '-n') {
        const amount = tokens[++index]?.value
        if (count !== undefined || !amount || !/^(?:[1-9]|10)$/u.test(amount))
          return usageError
        count = Number(amount)
      } else {
        if (
          host ||
          !/^[\da-z:][\da-z.:-]*$/iu.test(value) ||
          value.length > 254
        )
          return usageError
        host = value
      }
    }
    if (!host) return usageError
    return { type: 'ping', operation: { host, count: count ?? 4 } }
  },
}
