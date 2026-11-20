import { createApp } from 'vue'
import TerminalView from './terminal/TerminalView.vue'
import TimerNotice from './features/tasks/TimerNotice.vue'
import './styles.css'

const timer = new URLSearchParams(window.location.search).get('timer')
const app =
  timer === null
    ? createApp(TerminalView)
    : createApp(TimerNotice, { label: timer })
app.mount('#app')
