export type OpenTargetKind = 'app' | 'folder' | 'file' | 'url'

export type AliasOperation =
  | { action: 'list' }
  | { action: 'add'; name: string; target: string }
  | { action: 'remove'; name: string }

export type OpenOperation =
  | { action: 'add'; alias: string; kind: OpenTargetKind; target: string }
  | { action: 'list' }
  | { action: 'launch' | 'remove'; alias: string }

export type SearchOperation =
  | {
      action: 'run'
      site?: string
      browser?: string
      words: string[]
      inferSite: boolean
      allowUrl: boolean
    }
  | { action: 'site_add'; name: string; template: string }
  | { action: 'browser_add'; name: string; path: string }
  | { action: 'site_list' | 'browser_list' }
  | {
      action: 'site_remove' | 'browser_remove' | 'browser_default'
      name: string
    }

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
  | { type: 'alias'; operation: AliasOperation }
  | { type: 'search'; operation: SearchOperation }
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
