// Keep Markdown outside the visual editor's schema editable as source.
export function readVisualMarkdown(source: string): string | undefined {
  let fence: string | undefined
  const lines = source.split('\n')
  for (const [index, line] of lines.entries()) {
    const delimiter = /^ {0,3}(\x60{3,}|~{3,})(.*)$/u.exec(line)
    if (fence) {
      if (delimiter && delimiter[1].startsWith(fence) && !delimiter[2].trim())
        fence = undefined
      continue
    }
    if (delimiter) {
      fence = delimiter[1]
      continue
    }
    if (/^(?: {4}|\t)/u.test(line)) continue
    if (
      /<(?:\/?[a-z][\w-]*(?:\s|\/?>)|!|\?)/iu.test(line) ||
      /\[\^[^\]]+\]/u.test(line)
    )
      return undefined
    lines[index] = line.replace(/^( {0,3}#{1,6})(?=[^#\s])/u, '$1 ')
  }
  return lines.join('\n')
}
