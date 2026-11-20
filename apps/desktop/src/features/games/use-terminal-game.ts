import { isTauri } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { onMounted, onUnmounted, ref, watch, type Ref } from 'vue'
import type { GameId } from '../../terminal/command-types'
import { isCompositionKey } from '../../terminal/terminal-keyboard'

type Session = {
  id: GameId
  panel: Readonly<Ref<HTMLElement | null>>
  interval: number
  running: () => boolean
  score: () => number
  tick: (seconds: number) => void
  draw: () => void
  pause: () => void
  key: (key: string, repeat: boolean) => boolean
  finish: (best: number) => void
}

export function useTerminalGame(session: Session) {
  const storageKey = 'blis.game.' + session.id + '.high-score'
  const best = ref(0)
  try {
    const value = Number(localStorage.getItem(storageKey))
    if (Number.isSafeInteger(value) && value >= 0) best.value = value
  } catch (error) {
    console.warn('Failed to read game high score', error)
  }
  let savedBest = best.value
  let nextSaveAt = 0
  let lastTick = 0
  let clock: number | undefined
  let unlisten: UnlistenFn | undefined
  let mounted = false

  function saveBest() {
    if (savedBest === best.value) return
    try {
      localStorage.setItem(storageKey, String(best.value))
      savedBest = best.value
    } catch (error) {
      console.warn('Failed to save game high score', error)
    }
  }

  function tick() {
    const now = performance.now()
    session.tick(Math.min((now - lastTick) / 1000, 0.05))
    lastTick = now
    best.value = Math.max(best.value, session.score())
    if (!session.running() || now >= nextSaveAt) {
      saveBest()
      nextSaveAt = now + 750
    }
    session.draw()
  }

  function updateClock() {
    window.clearInterval(clock)
    clock = undefined
    if (mounted && session.running()) {
      lastTick = performance.now()
      clock = window.setInterval(tick, session.interval)
    } else {
      saveBest()
    }
  }

  function pauseWhenHidden() {
    if (document.hidden) session.pause()
  }

  function handleKey(event: KeyboardEvent) {
    if (
      event.defaultPrevented ||
      event.altKey ||
      event.ctrlKey ||
      event.metaKey ||
      isCompositionKey(event)
    )
      return
    const key = event.key.toLowerCase()
    if (key === 'escape') {
      event.preventDefault()
      if (!event.repeat) {
        saveBest()
        session.finish(best.value)
      }
    } else if (session.key(key, event.repeat)) {
      event.preventDefault()
    }
  }

  function stopListening(stop: UnlistenFn) {
    void Promise.resolve(stop()).catch((error) =>
      console.warn('Failed to unlisten from terminal visibility', error),
    )
  }

  watch(session.running, updateClock, { flush: 'sync' })
  onMounted(() => {
    session.panel.value?.focus({ preventScroll: true })
    session.draw()
    if (document.hidden) session.pause()
    mounted = true
    updateClock()
    window.addEventListener('keydown', handleKey)
    window.addEventListener('blur', session.pause)
    window.addEventListener('pagehide', session.pause)
    document.addEventListener('visibilitychange', pauseWhenHidden)
    if (isTauri()) {
      void listen('blis-terminal-hidden', session.pause, { target: 'terminal' })
        .then((stop) => {
          if (mounted) unlisten = stop
          else stopListening(stop)
        })
        .catch((error) =>
          console.warn('Failed to listen for terminal visibility', error),
        )
    }
  })

  onUnmounted(() => {
    mounted = false
    updateClock()
    window.removeEventListener('keydown', handleKey)
    window.removeEventListener('blur', session.pause)
    window.removeEventListener('pagehide', session.pause)
    document.removeEventListener('visibilitychange', pauseWhenHidden)
    if (unlisten) stopListening(unlisten)
  })
  return best
}
