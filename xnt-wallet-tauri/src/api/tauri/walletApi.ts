import type {
  AccountEntry,
  AccountKeys,
  BridgeDepositRecord,
  ChainCheck,
  ImportSource,
  Registry,
  SeedEntry,
  HistoryStatus,
  PayloadBuildBridgeDeposit,
  PayloadBuildLegacyDctidhMigration,
  PayloadBuildTransaction,
} from '@/types'
import type { AccountKeysRef } from '@/stores/accountsStore'
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

  buildLegacyDctidhMigration: (payload: PayloadBuildLegacyDctidhMigration): Promise<any> =>
    call('build_legacy_dctidh_migration', { payload }, 'Failed to move legacy funds', {
      errorFrom: 'rich',
    }),

  legacyDctidhDustThreshold: (): Promise<string> =>
    safeInvoke<string>('legacy_dctidh_dust_threshold'),

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

  checkChain: (): Promise<ChainCheck> => safeInvoke<ChainCheck>('check_chain'),

  deriveAccountKeys: (seedPhrase: string[], index: number, network: string): Promise<AccountKeys> =>
    safeInvoke<AccountKeys>('derive_account_keys', { seedPhrase, index, network }),

  accountsLoad: (): Promise<Registry> => safeInvoke<Registry>('accounts_load'),

  accountsAdd: (seedId: string, index: number): Promise<AccountEntry> =>
    safeInvoke<AccountEntry>('accounts_add', { seedId, index }),

  accountsRename: (id: string, name: string | null): Promise<Registry> =>
    safeInvoke<Registry>('accounts_rename', { id, name }),

  accountsSetHidden: (id: string, hidden: boolean): Promise<Registry> =>
    safeInvoke<Registry>('accounts_set_hidden', { id, hidden }),

  accountsSetActive: (id: string): Promise<Registry> =>
    safeInvoke<Registry>('accounts_set_active', { id }),

  accountsMarkBackedUp: (seedId: string): Promise<Registry> =>
    safeInvoke<Registry>('accounts_mark_backed_up', { seedId }),

  /** On an error the registry may already be saved without the seed: reload it. */
  accountsRemoveSeed: (seedId: string): Promise<Registry> =>
    safeInvoke<Registry>('accounts_remove_seed', { seedId }),

  /** Opens a save dialog for that seed's keystore file; resolves to `{ success, filePath }`. */
  accountsSaveKeystoreAs: (seedId: string): Promise<any> =>
    call('accounts_save_keystore_as', { seedId }, 'Failed to save keystore', {
      errorFrom: 'message',
    }),

  forgetAccounts: (accounts: AccountKeysRef[]): Promise<void> =>
    safeInvoke<void>('forget_accounts', { accounts }),

  accountsResetMain: (keystoreFile: string): Promise<Registry> =>
    safeInvoke<Registry>('accounts_reset_main', { keystoreFile }),

  accountsImport: (source: ImportSource, walletPassword: string): Promise<SeedEntry> =>
    safeInvoke<SeedEntry>('accounts_import', { source, walletPassword }),

  accountsDiscover: (seedId: string, walletPassword: string): Promise<AccountEntry[]> =>
    safeInvoke<AccountEntry[]>('accounts_discover', { seedId, walletPassword }),

  /** Reads a seed's words without touching wallet state. */
  accountsSeedPhrase: (seedId: string, walletPassword: string): Promise<string[]> =>
    safeInvoke<string[]>('accounts_seed_phrase', { seedId, walletPassword }),

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

  /** With `accounts`, only their UTXOs, tagged; without, every cached UTXO untagged. */
  getCachedUtxosLocal: (accounts?: AccountKeysRef[]): Promise<any> =>
    call('get_cached_utxos_local', { accounts }, 'Failed to get cached UTXOs local', {
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
