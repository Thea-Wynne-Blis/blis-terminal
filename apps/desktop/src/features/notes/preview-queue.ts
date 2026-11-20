// One native request may run at a time. Only the latest waiting document survives.
export function createPreviewQueue(
  render: (text: string) => Promise<string>,
  publish: (html: string) => void,
  reportError: () => void,
  delay = 45,
) {
  let version = 0
  let running = false
  let timeout: ReturnType<typeof setTimeout> | undefined
  let pending: { text: string; version: number } | undefined

  function clear() {
    version++
    clearTimeout(timeout)
    timeout = undefined
    pending = undefined
  }

  function update(text: string) {
    clear()
    pending = { text, version }
    timeout = setTimeout(() => {
      timeout = undefined
      void run()
    }, delay)
  }

  async function run() {
    if (running || timeout !== undefined || !pending) return
    const request = pending
    pending = undefined
    running = true
    try {
      const html = await render(request.text)
      if (request.version === version) publish(html)
    } catch {
      if (request.version === version) reportError()
    } finally {
      running = false
      void run()
    }
  }

  return { update, clear }
}
