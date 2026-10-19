export const PAGE_FIRST = 1
export const PER_PAGE = 10
export const POLLING_INTERVAL = 1000 * 60 // 1 minute
export const DEFAULT_MIN_BLOCK_HEIGHT = 0
export const POLLING_INTERVAL_XNT_PRICE = 1000 * 60 * 5 // 5 minutes
export const BRIDGE_POLLING_INTERVAL = 1000 * 15
export const MIN_BRIDGE_AMOUNT = 1
/** Prefilled fee for Send, Merge and Bridge; the user can change it. */
export const DEFAULT_FEE = '0.1'
export const DESKTOP_IDLE_LOCK_MS = 1000 * 60 * 15
export const CHAIN_RESET_MESSAGE = 'Different chain than the cached data: wallet re-synced'
export const IS_MOBILE =
  import.meta.env.TAURI_ENV_PLATFORM === 'android' || import.meta.env.TAURI_ENV_PLATFORM === 'ios'
/** Upper bound for the max-inputs setting; proving memory grows with every input. */
export const MAX_INPUTS_LIMIT = 5
export const DEFAULT_MAX_INPUTS = IS_MOBILE ? 3 : MAX_INPUTS_LIMIT
export const DEFAULT_LOW_MEMORY_PROVING = IS_MOBILE
