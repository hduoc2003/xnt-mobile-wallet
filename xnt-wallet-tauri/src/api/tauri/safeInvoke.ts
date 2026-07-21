import { invoke } from '@tauri-apps/api/core'

/**
 * Wrapper around Tauri's `invoke` that retries briefly while the runtime is
 * still initializing. The WebView can mount and start issuing commands before
 * `__TAURI_INTERNALS__` has been injected, which surfaces as a handful of
 * recognizable "not ready" errors; for those we back off and retry until
 * `maxWait` elapses. Any other error is rethrown immediately.
 */
export async function safeInvoke<T = any>(cmd: string, args?: any): Promise<T> {
  const maxWait = 5000
  const startTime = Date.now()

  while (true) {
    try {
      return await invoke<T>(cmd, args)
    } catch (e: any) {
      const msg = String(e?.message || e || '')
      const looksLikeNotReady =
        msg.includes("Cannot read properties of undefined (reading 'invoke')") ||
        msg.includes('__TAURI_INTERNALS__') ||
        msg.includes('Tauri API not available') ||
        msg.includes('Tauri is not ready')

      if (!looksLikeNotReady || Date.now() - startTime >= maxWait) {
        throw new Error(msg || 'Tauri API not available')
      }

      await new Promise((resolve) => setTimeout(resolve, 50))
    }
  }
}
