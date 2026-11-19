import { invoke, isTauri } from '@tauri-apps/api/core'
import type {
  AliasOperation,
  Appearance,
  AppearanceOperation,
  DesktopCommand,
  OpenOperation,
  OpenTargetKind,
} from './command-types'

type OpenOutcome =
  | { type: 'added' | 'removed'; alias: string }
  | {
      type: 'launched'
      alias: string
      status:
        | 'window_visible'
        | 'process_running'
        | 'request_submitted'
        | 'system_handoff'
    }
  | {
      type: 'listed'
      targets: Record<string, { kind: OpenTargetKind; target: string }>
    }

const kindNames: Record<OpenTargetKind, string> = {
  app: '应用',
  folder: '文件夹',
  file: '文件',
  url: '网址',
}

let openQueue = Promise.resolve()
let appearanceQueue = Promise.resolve()
let searchQueue = Promise.resolve()

export function executeAlias(
  operation: AliasOperation,
): Promise<Record<string, string>> {
  if (!isTauri())
    return Promise.reject(new Error('请在 Blis 桌面版中执行此命令。'))
  return invoke('execute_alias', { operation })
}

export function executeAppearance(
  operation: AppearanceOperation,
): Promise<Appearance> {
  if (!isTauri())
    return Promise.reject(new Error('请在 Blis 桌面版中执行此命令。'))
  const request = appearanceQueue.then(() =>
    invoke<Appearance>('execute_appearance', { operation }),
  )
  appearanceQueue = request.then(
    () => undefined,
    () => undefined,
  )
  return request
}

async function executeOpenOperation(operation: OpenOperation): Promise<string> {
  // Run list and launch after any previous edit submitted in this terminal.
  const request = openQueue.then(() =>
    invoke<OpenOutcome>('execute_open_target', { operation }),
  )
  openQueue = request.then(
    () => undefined,
    () => undefined,
  )

  const outcome = await request
  switch (outcome.type) {
    case 'added':
      return `已保存「${outcome.alias}」。`
    case 'removed':
      return `已移除「${outcome.alias}」。`
    case 'launched': {
      const { alias, status } = outcome
      const messages = {
        window_visible: `已打开「${alias}」，检测到应用窗口。`,
        process_running: `「${alias}」进程已启动，8 秒内暂未检测到窗口。`,
        request_submitted: `已提交「${alias}」的启动请求，应用可能会复用现有窗口。`,
        system_handoff: `已交给系统打开「${alias}」。`,
      }
      return messages[status]
    }
    case 'listed': {
      const rows = Object.entries(outcome.targets)
      if (!rows.length) return '打开清单为空。输入 help open 查看添加方法。'
      return rows
        .map(
          ([alias, { kind, target }]) =>
            `${alias}  [${kindNames[kind]}]  ${target}`,
        )
        .join('\n')
    }
  }
}

export async function runDesktopCommand(
  command: Exclude<DesktopCommand, { type: 'alias' | 'background' }>,
): Promise<string> {
  if (!isTauri()) throw new Error('请在 Blis 桌面版中执行此命令。')
  switch (command.type) {
    case 'search': {
      const request = searchQueue.then(() =>
        invoke<string>('execute_search', { operation: command.operation }),
      )
      searchQueue = request.then(
        () => undefined,
        () => undefined,
      )
      return request
    }
    case 'open-target':
      return executeOpenOperation(command.operation)
    case 'exit':
      await invoke('exit_app')
      return ''
  }
}
