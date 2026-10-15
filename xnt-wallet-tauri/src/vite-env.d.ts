/// <reference types="vite/client" />

declare module '*.vue' {
  import type { DefineComponent } from 'vue'
  const component: DefineComponent<{}, {}, any>
  export default component
}

interface ImportMetaEnv {
  /** Set by the Tauri CLI at build time: linux, darwin, windows, android or ios. */
  readonly TAURI_ENV_PLATFORM?: string
  readonly VITE_APP_API?: string
  readonly VITE_EXPLORER_URL?: string
  readonly VITE_LEVIATHAN_EXPLORER_URL?: string
  readonly VITE_PRICE_URL?: string
  readonly VITE_NODE_NETWORK?: string
}
