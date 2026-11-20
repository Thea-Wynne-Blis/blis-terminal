import { tokenizeArguments } from '../../terminal/command-arguments'
import type { CommandDefinition } from '../../terminal/command-types'

function fileCommand(name: 'cat' | 'vim'): CommandDefinition {
  const preview = name === 'cat'
  return {
    summary: preview ? '预览文本文件' : '打开内置文本编辑器',
    usage: [`${name} <文件路径>`],
    details: [
      '支持相对路径、绝对路径和 Tab 补全，包含空格的路径请加引号。',
      preview
        ? '读取 UTF-8 文本，长文件只显示开头。'
        : '支持 UTF-8 文本与新建文件。按 i 编辑、Esc 返回普通模式，再用 :w 保存、:q 退出、:wq 保存并退出或 :q! 放弃退出。',
    ],
    examples: [`${name} README.md`, `${name} "项目计划.txt"`],
    run: (args) => {
      const tokens = tokenizeArguments(args, { preserveBackslashes: true })
      const path = tokens?.[0]?.value
      if (tokens?.length !== 1 || !path)
        return {
          type: 'error',
          text: `请输入一个文件路径。输入 help ${name} 查看用法。`,
        }
      return {
        type: 'directory',
        operation: { action: preview ? 'preview_text' : 'edit_text', path },
      }
    },
  }
}

export const catDefinition = fileCommand('cat')
export const vimDefinition = fileCommand('vim')
