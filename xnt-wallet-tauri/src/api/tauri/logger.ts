import { safeInvoke } from './safeInvoke'

export type LogLevel = 'trace' | 'debug' | 'info' | 'warn' | 'error'

/** Forwards a single log line to the native backend (best-effort, never throws). */
export function sendLogToNative(level: LogLevel, message: string): void {
  try {
    safeInvoke('log_message', { level, message }).catch(() => {})
  } catch {
    // Native logging is best-effort; it must never break the app.
  }
}

const toMessage = (args: unknown[]): string =>
  args.map((a) => (typeof a === 'string' ? a : JSON.stringify(a))).join(' ')

export interface NativeLogger {
  info(...args: unknown[]): void
  warn(...args: unknown[]): void
  error(...args: unknown[]): void
  debug(...args: unknown[]): void
}

/** Console-and-native logger, also exposed on `window.logger` for legacy callers. */
export const logger: NativeLogger = {
  info: (...args) => sendLogToNative('info', toMessage(args)),
  warn: (...args) => sendLogToNative('warn', toMessage(args)),
  error: (...args) => {
    console.error('[ERROR]', ...args)
    sendLogToNative('error', toMessage(args))
  },
  debug: (...args) => sendLogToNative('debug', toMessage(args)),
}

/**
 * Installs `window.logger` (idempotently) and global handlers that forward
 * uncaught errors and unhandled rejections to the native log.
 */
export function installNativeLogger(): void {
  if (typeof window === 'undefined') return

  if (!(window as any).logger) {
    ;(window as any).logger = logger
  }

  window.addEventListener('error', (event) => {
    const stack = event.error?.stack ? `\n${event.error.stack}` : ''
    const msg = `[JS error] ${event.message || ''} at ${event.filename}:${event.lineno}:${event.colno}${stack}`
    console.error(msg, event.error)
    sendLogToNative('error', msg)
  })

  window.addEventListener('unhandledrejection', (event) => {
    const reason: any = event.reason
    const detail = (reason && (reason.message || reason.toString())) || 'unknown'
    // Include the stack so async component/render failures can be traced to a
    // specific component (the message alone is not enough to locate the source).
    const stack = reason?.stack ? `\n${reason.stack}` : ''
    const msg = `[JS unhandledrejection] ${detail}${stack}`
    console.error(msg, reason)
    sendLogToNative('error', msg)
  })
}
