export type OpenTargetKind = 'app' | 'folder' | 'file' | 'url'

export type OpenOperation =
  | { action: 'add'; alias: string; kind: OpenTargetKind; target: string }
  | { action: 'list' }
  | { action: 'launch' | 'remove'; alias: string }

export type AppearanceOperation =
  | { action: 'get' | 'reset' }
  | { action: 'select' | 'remove'; name: string }
  | { action: 'add_color'; name: string; color: string }
  | { action: 'add_image'; name: string; path: string }
  | { action: 'opacity'; value: number }

export type BackgroundTheme = {
  id: string
  label: string
  builtIn: boolean
  color: string
  imagePath: string | null
  light: boolean
}

export type Appearance = {
  selected: string
  opacity: number
  themes: readonly BackgroundTheme[]
}

export type DesktopCommand =
  | { type: 'open-web'; url: string; text: string }
  | { type: 'open-target'; operation: OpenOperation }
  | { type: 'background'; operation: AppearanceOperation }
  | { type: 'exit' }

export type CommandResult =
  { type: 'text' | 'error'; text: string } | { type: 'clear' } | DesktopCommand

export type CommandDefinition = {
  summary: string
  usage: readonly [string, ...string[]]
  details?: readonly string[]
  examples: readonly [string, ...string[]]
  run: (args: string, history: readonly string[]) => CommandResult
}
