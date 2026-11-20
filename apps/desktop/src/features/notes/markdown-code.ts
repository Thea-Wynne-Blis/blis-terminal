import bash from 'highlight.js/lib/languages/bash'
import c from 'highlight.js/lib/languages/c'
import cpp from 'highlight.js/lib/languages/cpp'
import csharp from 'highlight.js/lib/languages/csharp'
import css from 'highlight.js/lib/languages/css'
import go from 'highlight.js/lib/languages/go'
import java from 'highlight.js/lib/languages/java'
import javascript from 'highlight.js/lib/languages/javascript'
import json from 'highlight.js/lib/languages/json'
import kotlin from 'highlight.js/lib/languages/kotlin'
import lua from 'highlight.js/lib/languages/lua'
import markdown from 'highlight.js/lib/languages/markdown'
import php from 'highlight.js/lib/languages/php'
import powershell from 'highlight.js/lib/languages/powershell'
import python from 'highlight.js/lib/languages/python'
import ruby from 'highlight.js/lib/languages/ruby'
import rust from 'highlight.js/lib/languages/rust'
import sql from 'highlight.js/lib/languages/sql'
import swift from 'highlight.js/lib/languages/swift'
import typescript from 'highlight.js/lib/languages/typescript'
import xml from 'highlight.js/lib/languages/xml'
import yaml from 'highlight.js/lib/languages/yaml'
import { createLowlight } from 'lowlight'

const languages = {
  bash,
  c,
  cpp,
  csharp,
  css,
  go,
  java,
  javascript,
  json,
  kotlin,
  lua,
  markdown,
  php,
  powershell,
  python,
  ruby,
  rust,
  sql,
  swift,
  typescript,
  xml,
  yaml,
}
const lowlight = createLowlight(languages)
export const codeLanguages = Object.keys(languages)

const fencedCode = 'pre > code[class^="language-"]'
const maxHighlightedLength = 32_768

type Highlight = ReturnType<typeof lowlight.highlight>
type Token = Highlight['children'][number]

function plainHighlight(value: string): Highlight {
  return { type: 'root', children: [{ type: 'text', value }] }
}

export const syntax = {
  listLanguages: lowlight.listLanguages,
  registered: lowlight.registered,
  highlight(language: string, value: string) {
    return value.length <= maxHighlightedLength
      ? lowlight.highlight(language, value)
      : plainHighlight(value)
  },
  // Fences without a known language stay plain, avoiding a guess across all grammars.
  highlightAuto: plainHighlight,
}

function tokenNodes(tokens: readonly Token[]): Node[] {
  return tokens.flatMap((token): Node[] => {
    if (token.type === 'text') return [document.createTextNode(token.value)]
    if (token.type !== 'element') return []
    const span = document.createElement('span')
    const classes = token.properties.className
    if (Array.isArray(classes)) span.className = classes.join(' ')
    span.append(...tokenNodes(token.children))
    return [span]
  })
}

export function decorateMarkdownCode(root: HTMLElement) {
  for (const code of root.querySelectorAll<HTMLElement>(fencedCode)) {
    const block = code.parentElement
    const language = code.classList[0]?.slice('language-'.length).toLowerCase()
    if (!block || !language || block.parentElement?.matches('.note-code-block'))
      continue

    const frame = document.createElement('figure')
    frame.className = 'note-code-block'
    const caption = document.createElement('figcaption')
    caption.textContent = language
    block.replaceWith(frame)
    frame.append(block, caption)

    const source = code.textContent ?? ''
    if (!syntax.registered(language)) continue
    code.replaceChildren(
      ...tokenNodes(syntax.highlight(language, source).children),
    )
  }
}
