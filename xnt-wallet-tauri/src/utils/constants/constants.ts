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
export const IS_MOBILE =
  import.meta.env.TAURI_ENV_PLATFORM === 'android' || import.meta.env.TAURI_ENV_PLATFORM === 'ios'
/** Upper bound for the Max inputs control; proving memory grows with every input. */
export const MAX_INPUTS_LIMIT = 5
export const DEFAULT_MAX_INPUTS = IS_MOBILE ? 2 : MAX_INPUTS_LIMIT
