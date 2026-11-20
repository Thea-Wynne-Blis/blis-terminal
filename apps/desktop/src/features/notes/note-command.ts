import { tokenizeArguments } from '../../terminal/command-arguments'
import type {
  CommandDefinition,
  CommandResult,
  NoteFormat,
} from '../../terminal/command-types'

export const noteDefinition: CommandDefinition = {
  summary: '管理和导出本地笔记',
  usage: [
    'note new <标题> [-f|--format <md|txt>]',
    'note edit <标题>',
    'note list',
    'note remove <标题>',
    'note export <标题> <-f|--format> <md|txt> [-o|--out <绝对文件路径>]',
  ],
  details: [
    '标题含空格时请加引号。创建时默认使用 MD，编辑时自动识别格式。',
    'Markdown 支持可视化、源码和分栏视图，以及浅色、深色和纸张主题。',
    'Ctrl+S 保存，Ctrl+/ 切换源码，Ctrl+0–6 设置正文或标题。',
    '可视化编辑中，Ctrl+B / Ctrl+I 设置粗体或斜体，Ctrl+K 插入链接，Ctrl+T 插入表格，Ctrl+Shift+K 插入代码块。',
    '代码块会补齐括号和引号。Enter 延续缩进，Tab / Shift+Tab 调整缩进，Ctrl+Enter 返回正文。',
    'note remove 显示删除确认。选择取消或按 Esc 可保留笔记。',
    '默认导出到「文档」里的 Blis Notes 文件夹。-o 和 --out 都可指定绝对文件路径，已有文件会保留。',
    'MD 导出笔记原文，TXT 导出纯文本。',
  ],
  examples: [
    'note new "项目计划"',
    'note new "临时记录" -f txt',
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
  if (action === 'new') {
    if (!options.length) return { type: 'note', operation: { action, title } }
    const [flag, format] = options
    if (
      options.length !== 2 ||
      (flag !== '-f' && flag !== '--format') ||
      (format !== 'md' && format !== 'txt')
    )
      return noteError('创建格式请选择 md 或 txt。')
    return { type: 'note', operation: { action, title, format } }
  }
  if ((action === 'edit' || action === 'remove') && options.length === 0)
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
