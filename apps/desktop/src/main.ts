import './styles.css'

async function mountApp() {
  const timer = new URLSearchParams(window.location.search).get('timer')
  const host = document.querySelector<HTMLElement>('#app')
  if (!host) throw new Error('Missing Blis application root')
  if (timer !== null) {
    const { mountTimerNotice } = await import('./features/tasks/timer-notice')
    mountTimerNotice(host, timer)
    return
  }
  const [{ createApp }, { default: TerminalView }] = await Promise.all([
    import('vue'),
    import('./terminal/TerminalView.vue'),
  ])
  createApp(TerminalView).mount(host)
}

void mountApp().catch((error) => console.error('Failed to mount Blis', error))
