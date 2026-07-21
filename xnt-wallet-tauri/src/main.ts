import { createApp } from 'vue'
import { createPinia } from 'pinia'
import i18n from '@/i18n'
import { installNativeLogger, installWalletApi, sendLogToNative } from '@/api/tauri'
import App from './App.vue'
import router from './router'
import './style.css'

// Wire up native logging (+ global error hooks) and the Tauri command bridge
// before the app mounts so early failures are captured.
installNativeLogger()
installWalletApi()

function initializeApp() {
  const appElement = document.getElementById('app')
  if (!appElement) {
    sendLogToNative('error', '[Tauri] #app element not found!')
    return
  }

  const app = createApp(App)

  // Surface Vue render/lifecycle errors (with the failing component + phase) to
  // the native log — the default only reports uncaught window errors.
  app.config.errorHandler = (err: any, _instance, info) => {
    const stack = err?.stack ? `\n${err.stack}` : ''
    console.error('[Vue errorHandler]', info, err)
    sendLogToNative('error', `[Vue errorHandler] ${info}: ${err?.message || err}${stack}`)
  }

  app.use(i18n)
  app.use(createPinia())
  app.use(router)

  // Some dependencies still attach the deprecated DOMNodeInserted mutation
  // event, which throws in modern WebViews; swallow those registrations.
  const originalAddEventListener = Element.prototype.addEventListener
  Element.prototype.addEventListener = function (type: any, listener: any, options: any) {
    if (type === 'DOMNodeInserted') return
    return originalAddEventListener.call(this, type, listener, options)
  }

  try {
    app.mount('#app')
  } catch (err: any) {
    sendLogToNative('error', `[Tauri] Failed to mount Vue app: ${err?.message || err}`)
    throw err
  }
}

initializeApp()
