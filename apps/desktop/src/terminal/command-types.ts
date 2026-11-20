export type OpenTargetKind = 'app' | 'folder' | 'file' | 'url'

export type DirectoryOperation =
  | { action: 'pwd' | 'open_current' }
  | { action: 'cd'; path?: string }
  | { action: 'open_in_code'; path: string }
  | { action: 'preview_text' | 'edit_text'; path: string }
  | { action: 'save_text'; id: number; content: string }
  | { action: 'close_text'; id: number }
  | { action: 'ls'; path?: string; all?: boolean; detailed?: boolean }
  | {
      action: 'complete_path'
      prefix: string
      directories_only: boolean
      all: boolean
    }
  | { action: 'mkdir'; path: string }
  | { action: 'copy'; source: string; destination: string; recursive: boolean }
  | { action: 'move'; source: string; destination: string }
  | { action: 'remove'; path: string; recursive: boolean }
  | { action: 'finish_remove'; id: number; confirmed: boolean }

export type PathCandidate = { path: string; directory: boolean }

export type DirectoryEntry = {
  name: string
  kind: 'directory' | 'file' | 'link' | 'other'
  bytes: number | null
  modified_ms: number | null
}

export type DirectoryOutcome = {
  directory: string
  text: string
  removal?: { id: number; path: string }
  listing?: DirectoryEntry[]
  candidates?: PathCandidate[]
  editor?: TextDraft
}

export type TextDraft = { id: number; path: string; content: string }

export type NoteFormat = 'md' | 'txt'

export type NoteCommand =
  | { action: 'list' }
  | { action: 'new'; title: string; format?: NoteFormat }
  | { action: 'edit' | 'remove'; title: string }
  | { action: 'export'; title: string; format: NoteFormat; output?: string }

export type NoteOperation =
  | NoteCommand
  | {
      action: 'save'
      title: string
      content: string
      create: boolean
      format: NoteFormat
    }
  | { action: 'remove'; title: string; confirmed: true; format: NoteFormat }
  | { action: 'preview'; content: string }

export type NoteDraft = {
  title: string
  content: string
  create: boolean
  format: NoteFormat
}

export type NoteOutcome =
  | ({ type: 'editor' } & NoteDraft)
  | { type: 'confirm_remove'; title: string; format: NoteFormat }
  | { type: 'saved'; text: string; content: string }
  | { type: 'preview'; text: string }
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

export type TimerOperation = { seconds: number; label: string }

export type WelcomeOperation =
  { action: 'get' | 'reset' } | { action: 'set'; message: string }

export type TodoOperation =
  | { action: 'add'; title: string }
  | { action: 'list' }
  | { action: 'done'; id: number }

export type StoreOperation =
  { action: 'get' | 'reset' } | { action: 'set'; path: string }

export type DesktopCommand =
  | { type: 'alias'; operation: AliasOperation }
  | { type: 'directory'; operation: DirectoryOperation }
  | { type: 'note'; operation: NoteCommand }
  | { type: 'search'; operation: SearchOperation }
  | { type: 'open-target'; operation: OpenOperation }
  | { type: 'background'; operation: AppearanceOperation }
  | { type: 'timer'; operation: TimerOperation }
  | { type: 'todo'; operation: TodoOperation }
  | { type: 'welcome'; operation: WelcomeOperation }
  | { type: 'ping'; operation: { host: string; count: number } }
  | { type: 'store'; operation: StoreOperation }
  | { type: 'exit' }

export type GameId = 'snake' | 'runner' | 'dodge'

export type CommandResult =
  | { type: 'text' | 'error'; text: string }
  | { type: 'clear' }
  | { type: 'game'; game: GameId }
  | DesktopCommand

export type CommandDefinition = {
  summary: string
  usage: readonly [string, ...string[]]
  details?: readonly string[]
  examples: readonly [string, ...string[]]
  run: (args: string, history: readonly string[]) => CommandResult
}
