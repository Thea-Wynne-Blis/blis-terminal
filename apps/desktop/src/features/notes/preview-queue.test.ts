import { afterEach, describe, expect, it, vi } from 'vitest'
import { createPreviewQueue } from './preview-queue'

afterEach(() => vi.useRealTimers())

function defer() {
  let resolve!: (value: string) => void
  let reject!: (reason: unknown) => void
  const promise = new Promise<string>((pass, fail) => {
    resolve = pass
    reject = fail
  })
  return { promise, resolve, reject }
}

describe('Markdown preview queue', () => {
  it('serializes rendering and keeps only the newest waiting draft', async () => {
    vi.useFakeTimers()
    const first = defer()
    const render = vi.fn(async (source: string) =>
      source === 'first' ? first.promise : source,
    )
    const publish = vi.fn()
    const queue = createPreviewQueue(render, publish, vi.fn())
    queue.update('first')
    await vi.advanceTimersByTimeAsync(45)
    queue.update('discarded')
    await vi.advanceTimersByTimeAsync(45)
    queue.update('latest')
    await vi.advanceTimersByTimeAsync(45)
    expect(render.mock.calls).toEqual([['first']])
    first.resolve('stale')
    await vi.advanceTimersByTimeAsync(0)
    expect(render.mock.calls).toEqual([['first'], ['latest']])
    expect(publish.mock.calls).toEqual([['latest']])
  })

  it('ignores an in-flight failure after leaving the preview', async () => {
    vi.useFakeTimers()
    const response = defer()
    const publish = vi.fn()
    const reportError = vi.fn()
    const queue = createPreviewQueue(
      () => response.promise,
      publish,
      reportError,
    )
    queue.update('first')
    await vi.advanceTimersByTimeAsync(45)
    queue.clear()
    response.reject(new Error('old failure'))
    await vi.advanceTimersByTimeAsync(0)
    expect(publish).not.toHaveBeenCalled()
    expect(reportError).not.toHaveBeenCalled()
  })
})
