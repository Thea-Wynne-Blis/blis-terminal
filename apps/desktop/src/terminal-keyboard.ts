const outputShortcuts = [
  { key: 'l', action: 'clear', label: 'Ctrl + L', description: '清屏' },
  {
    key: 'o',
    action: 'toggle-output',
    label: 'Ctrl + O',
    description: '折叠或展开命令输出',
  },
] as const

const historyShortcuts = [
  { key: 'ArrowUp', step: -1, label: '↑', description: '上一条命令' },
  { key: 'ArrowDown', step: 1, label: '↓', description: '下一条命令' },
] as const

export function describeShortcuts(): string {
  const shortcuts = [
    ...outputShortcuts,
    ...historyShortcuts,
    { label: 'Ctrl + C', description: '复制选中的文本' },
    { label: 'Ctrl + V', description: '粘贴到输入框' },
    { label: 'Esc', description: '隐藏终端窗口' },
    { label: '鼠标拖选', description: '选择命令或输出文本' },
  ]
  return shortcuts
    .map(({ label, description }) => `${label.padEnd(14)}${description}`)
    .join('\n')
}

export function isCompositionKey(event: KeyboardEvent): boolean {
  // Windows IMEs can report keyCode 229 for a composition event.
  return event.isComposing || event.keyCode === 229
}

export function getOutputShortcut(event: KeyboardEvent) {
  if (
    event.defaultPrevented ||
    isCompositionKey(event) ||
    !event.ctrlKey ||
    event.altKey ||
    event.metaKey ||
    event.shiftKey
  )
    return undefined

  return outputShortcuts.find(({ key }) => key === event.key.toLowerCase())
    ?.action
}

export function getHistoryStep(event: KeyboardEvent) {
  if (
    event.defaultPrevented ||
    isCompositionKey(event) ||
    event.ctrlKey ||
    event.altKey ||
    event.metaKey ||
    event.shiftKey
  )
    return undefined

  return historyShortcuts.find(({ key }) => key === event.key)?.step
}

export function shouldHideTerminal(event: KeyboardEvent): boolean {
  return (
    event.key === 'Escape' &&
    !event.defaultPrevented &&
    !event.repeat &&
    !isCompositionKey(event)
  )
}
