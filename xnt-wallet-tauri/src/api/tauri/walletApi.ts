import type {
  BridgeDepositRecord,
  HistoryStatus,
  PayloadBuildBridgeDeposit,
  PayloadBuildTransaction,
} from '@/types'
import { safeInvoke } from './safeInvoke'

/** How the caught error is turned into the `error` field of a failure result. */
type ErrorFrom = 'raw' | 'message' | 'rich'

interface CallOptions {
  /** JSON.parse the string result before returning (default true). */
  parse?: boolean
  /** Strategy for deriving the returned error value (default 'raw'). */
  errorFrom?: ErrorFrom
  /** Extra fields merged into the `{ success: false }` failure object. */
  failureExtra?: Record<string, unknown>
  /** Also log the caught error to the console. */
  logToConsole?: boolean
}

const deriveError = (error: any, mode: ErrorFrom, fallback: string): unknown => {
  switch (mode) {
    case 'message':
      return error?.message || error || fallback
    case 'rich':
      return typeof error === 'string' ? error : error?.message || error?.toString?.() || fallback
    default:
      return error || fallback
  }
}

/**
 * Invokes a Tauri command and parses its JSON string result. On failure it
 * resolves to a `{ success: false, error }` object — the shape every caller
 * already checks — rather than throwing.
 */
async function call<T = any>(
  cmd: string,
  args: any,
  fallback: string,
  opts: CallOptions = {},
): Promise<T> {
  const { parse = true, errorFrom = 'raw', failureExtra, logToConsole } = opts
  try {
    const result = await safeInvoke<string>(cmd, args)
    return (parse ? JSON.parse(result) : result) as T
  } catch (error: any) {
    if (logToConsole) console.error(`[walletApi] ${cmd} failed:`, error)
    return { success: false, error: deriveError(error, errorFrom, fallback), ...failureExtra } as T
  }
}

/**
 * Typed façade over the Tauri wallet commands. Each method mirrors a
 * `#[tauri::command]` in the Rust backend and normalizes its JSON response.
 */
export const walletApi = {
  // These two throw on failure (callers expect an exception, not a result object).
  generateSeed: async (): Promise<any> => {
    try {
      return JSON.parse(await safeInvoke<string>('generate_seed'))
    } catch (error: any) {
      throw new Error(error?.message || error || 'Failed to generate seed')
    }
  },

  generateKeysFromSeed: async (
    seedPhrase: string[],
    network?: string,
    keyType?: string,
  ): Promise<any> => {
    const networkStr = !network || network === 'mainnet' ? 'main' : network
    try {
      return JSON.parse(
        await safeInvoke<string>('generate_keys_from_seed', {
          seedPhrase,
          network: networkStr,
          keyType: keyType || 'dctidh',
        }),
      )
    } catch (error: any) {
      console.error('[walletApi] Error generating keys:', error)
      throw new Error(error?.message || error || 'Failed to generate keys from seed')
    }
  },

  decryptKeystore: (fileName: string, password: string): Promise<any> =>
    call('decrypt_keystore', { fileName, password }, 'Failed to decrypt keystore'),

  saveKeystoreAs: (fileName: string): Promise<any> =>
    call('save_keystore_as', { fileName }, 'Failed to save keystore'),

  updateMinBlockHeight: (keystoreFileName: string, blockHeight: number | null): Promise<any> => {
    const fileName = keystoreFileName || 'default'
    if (blockHeight === null || blockHeight === undefined) return Promise.resolve({ success: true })
    return call(
      'update_min_block_height',
      { fileName, blockHeight },
      'Failed to update min block height',
      {
        logToConsole: true,
      },
    )
  },

  buildTransaction: (payload: PayloadBuildTransaction): Promise<any> =>
    call('build_transaction', { payload }, 'Failed to build transaction', { errorFrom: 'rich' }),

  buildBridgeDeposit: (payload: PayloadBuildBridgeDeposit): Promise<any> =>
    call('build_bridge_deposit', { payload }, 'Failed to build bridge deposit', {
      errorFrom: 'rich',
    }),

  listBridgeDeposits: (): Promise<BridgeDepositRecord[]> =>
    safeInvoke<BridgeDepositRecord[]>('list_bridge_deposits'),

  discardBridgeDeposit: (commitment: string): Promise<BridgeDepositRecord[]> =>
    safeInvoke<BridgeDepositRecord[]>('discard_bridge_deposit', { commitment }),

  refreshBridgeDeposits: (): Promise<BridgeDepositRecord[]> =>
    safeInvoke<BridgeDepositRecord[]>('refresh_bridge_deposits'),

  txStatuses: (commitments: string[]): Promise<HistoryStatus[]> =>
    safeInvoke<HistoryStatus[]>('tx_statuses', { commitments }),

  /** Serial of the Leviathan burn that paid a cached UTXO, or null if no bridge settlement did. */
  bridgeBurnSerial: (commitment: string): Promise<string | null> =>
    safeInvoke<string | null>('bridge_burn_serial', { commitment }),

  getMinBlockHeight: (fileName: string): Promise<any> =>
    call('get_min_block_height', { fileName }, 'Failed to get min block height'),

  saveMinBlockHeight: (fileName: string, blockHeight: number): Promise<any> =>
    call('save_min_block_height', { fileName, blockHeight }, 'Failed to save min block height'),

  keystoreExists: (fileName: string): Promise<any> =>
    call('keystore_exists', { fileName }, 'Failed to check keystore', {
      failureExtra: { exists: false },
    }),

  checkKeystore: (fileName?: string): Promise<any> =>
    call('check_keystore', { fileName }, 'Failed to check keystore', {
      failureExtra: { exists: false },
    }),

  saveKeystoreDialog: (keystoreData: string, defaultFileName: string): Promise<any> =>
    call('save_keystore_dialog', { keystoreData, defaultFileName }, 'Failed to save keystore'),

  createKeystore: (seed: string, password: string): Promise<any> =>
    call('create_keystore', { seed, password }, 'Failed to create keystore'),

  getKeystorePath: (): Promise<any> =>
    call('get_keystore_path', undefined, 'Failed to get keystore path'),

  getCachedUtxosLocal: (): Promise<any> =>
    call('get_cached_utxos_local', undefined, 'Failed to get cached UTXOs local', {
      errorFrom: 'message',
    }),

  setRpcEndpoint: async (endpoint: string): Promise<any> => {
    try {
      await safeInvoke('set_rpc_endpoint', { endpoint })
      return { success: true }
    } catch (error: any) {
      return { success: false, error: error?.message || error || 'Failed to set RPC endpoint' }
    }
  },

  getCustomRpcEndpoint: async (): Promise<string | null> => {
    try {
      const result = await safeInvoke<string | null>('get_custom_rpc_endpoint')
      return result || null
    } catch {
      // Return null (not an error object) so callers' `endpoint || fallback` picks
      // the default endpoint instead of using a `{success:false}` object as a URL.
      return null
    }
  },

  scanUtxosClientSide: (request: any): Promise<any> =>
    call('scan_utxos_client_side', { request }, 'Failed to scan UTXOs', { errorFrom: 'rich' }),

  getCachedUtxos: (): Promise<any> =>
    call('get_cached_utxos', undefined, 'Failed to get cached UTXOs', { errorFrom: 'message' }),

  clearUtxoCache: (): Promise<any> =>
    call('clear_utxo_cache', undefined, 'Failed to clear UTXO cache', { errorFrom: 'message' }),

  getPendingBalance: (request: any): Promise<any> =>
    call('get_pending_balance', { request }, 'Failed to get pending balance', {
      errorFrom: 'message',
    }),

  proxyHttpGet: (url: string): Promise<any> =>
    call('proxy_http_get', { url }, 'Failed to fetch URL', { errorFrom: 'message' }),
}

export type WalletApi = typeof walletApi

/** Installs `window.walletApi` (idempotently) for legacy `(window as any).walletApi` callers. */
export function installWalletApi(): void {
  if (typeof window !== 'undefined' && !(window as any).walletApi) {
    ;(window as any).walletApi = walletApi
  }
}
