export type OpenTargetKind = 'app' | 'folder' | 'file' | 'url'

export type DirectoryOperation =
  | { action: 'pwd' | 'open_current' }
  | { action: 'cd'; path?: string }
  | { action: 'ls'; path?: string }
  | {
      action: 'complete_path'
      prefix: string
      directories_only: boolean
    }
  | { action: 'mkdir'; path: string }
  | { action: 'copy'; source: string; destination: string; recursive: boolean }
  | { action: 'move'; source: string; destination: string }
  | { action: 'remove'; path: string; recursive: boolean }
  | { action: 'finish_remove'; id: number; confirmed: boolean }

export type PathCandidate = { path: string; directory: boolean }

export type DirectoryOutcome = {
  directory: string
  text: string
  removal?: { id: number; path: string }
  candidates?: PathCandidate[]
}

export type NoteFormat = 'md' | 'txt'

export type NoteCommand =
  | { action: 'list' }
  | { action: 'new' | 'edit' | 'remove'; title: string }
  | { action: 'export'; title: string; format: NoteFormat; output?: string }

export type NoteOperation =
  | NoteCommand
  | { action: 'save'; title: string; content: string; create: boolean }
  | { action: 'remove'; title: string; confirmed: true }

export type NoteDraft = { title: string; content: string; create: boolean }

export type NoteOutcome =
  | ({ type: 'editor' } & NoteDraft)
  | { type: 'confirm_remove'; title: string }
  | { type: 'message'; text: string }

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
  | { type: 'directory'; operation: DirectoryOperation }
  | { type: 'note'; operation: NoteCommand }
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
