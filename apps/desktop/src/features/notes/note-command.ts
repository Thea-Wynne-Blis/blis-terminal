import { tokenizeArguments } from '../../terminal/command-arguments'
import type {
  CommandDefinition,
  CommandResult,
  NoteFormat,
} from '../../terminal/command-types'

export const noteDefinition: CommandDefinition = {
  summary: '管理和导出本地笔记',
  usage: [
    'note new <标题>',
    'note edit <标题>',
    'note list',
    'note remove <标题>',
    'note export <标题> <-f|--format> <md|txt> [-o|--out <绝对文件路径>]',
  ],
  details: [
    '标题含空格时请加引号。在编辑器里按 Ctrl+S 保存 Markdown。',
    'note remove 显示删除确认。选择取消或按 Esc 可保留笔记。',
    '默认导出到「文档」里的 Blis Notes 文件夹。-o 和 --out 都可指定绝对文件路径，已有文件会保留。',
    'MD 保留笔记原文，TXT 导出纯文本。',
  ],
  examples: [
    'note new "项目计划"',
    'note edit "项目计划"',
    'note list',
    'note remove "项目计划"',
    'note export "项目计划" -f md',
    'note export "项目计划" --format txt -o "D:/笔记/项目计划.txt"',
  ],
  run: parseNote,
}

function noteError(text: string): CommandResult {
  return { type: 'error', text: text + ' 输入 help note 查看用法。' }
}

function parseNote(input: string): CommandResult {
  const tokens = tokenizeArguments(input, { preserveBackslashes: true })
  if (!tokens) return noteError('引号没有闭合。')
  const [verb, title, ...options] = tokens.map(({ value }) => value)
  const action = verb?.toLowerCase()

  if (action === 'list' && tokens.length === 1)
    return { type: 'note', operation: { action } }
  if (!title?.trim()) return noteError('请输入笔记标题。')
  if (
    (action === 'new' || action === 'edit' || action === 'remove') &&
    options.length === 0
  )
    return { type: 'note', operation: { action, title } }
  if (action !== 'export') return noteError('笔记命令或参数有误。')

  let format: NoteFormat | undefined
  let output: string | undefined
  for (let index = 0; index < options.length; index += 2) {
    const [flag, value] = [options[index], options[index + 1]]
    if (flag === '-f' || flag === '--format') {
      if (format || (value !== 'md' && value !== 'txt'))
        return noteError('请指定一次导出格式 md 或 txt。')
      format = value
    } else if (
      (flag === '-o' || flag === '--out') &&
      !output &&
      value?.trim() &&
      !value.startsWith('-')
    ) {
      output = value
    } else {
      return noteError('导出参数有误。')
    }
  }
  if (!format) return noteError('请使用 -f 或 --format 指定导出格式。')
  return { type: 'note', operation: { action, title, format, output } }
}
