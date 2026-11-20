import './timer-notice.css'

export function mountTimerNotice(host: HTMLElement, label: string) {
  const notice = document.createElement('main')
  notice.className = 'timer-notice'
  notice.setAttribute('role', 'status')

  const heading = document.createElement('strong')
  heading.textContent = 'Blis 倒计时结束'
  const message = document.createElement('p')
  message.textContent = label

  notice.append(heading, message)
  host.replaceChildren(notice)
}
