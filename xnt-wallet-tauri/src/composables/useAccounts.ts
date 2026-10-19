import { walletApi } from '@/api/tauri/walletApi'
import * as API from '@/api/neptuneApi'
import { MAIN_SEED, useAccountsStore, useNeptuneStore } from '@/stores'
import type { AccountEntry, ImportSource, Registry, SeedEntry } from '@/types'
import { normalizeUtxo } from '@/utils'

/** Opened seeds' words, in memory only, for deriving new accounts without asking again. */
const seedPhrases = new Map<string, string[]>()
/** Bumped by every lock and unlock, so work that a lock overtook stops instead of reopening. */
let epoch = 0

export const currentEpoch = () => epoch

/** Throws when a lock happened since `mine`, so work started before it writes nothing. */
export const ensureOpen = (mine: number) => {
  if (epoch !== mine) throw new Error('The wallet was locked meanwhile')
}

export const historyKey = (base: string, accountId: string) => `${base}:${accountId}`

type KeyValueStorage = Pick<Storage, 'getItem' | 'setItem' | 'removeItem'>

const LEGACY_HISTORY_KEYS = [
  { key: 'xnt_tx_history', id: 'id' },
  { key: 'xnt_received_history', id: 'commitment' },
] as const

const isRow = (row: unknown): row is Record<string, unknown> =>
  typeof row === 'object' && row !== null

/** Rows of a stored list; `undefined` when the value is not a JSON list. */
const parseList = (raw: string | null): Record<string, unknown>[] | undefined => {
  if (raw === null) return []
  try {
    const parsed: unknown = JSON.parse(raw)
    return Array.isArray(parsed) ? parsed.filter(isRow) : undefined
  } catch {
    return undefined
  }
}

/** Moves pre-accounts History to Account 1 by id; a corrupt value is set aside and reported. */
export function migrateLegacyStorage(storage: KeyValueStorage): string[] {
  const problems: string[] = []
  for (const { key, id } of LEGACY_HISTORY_KEYS) {
    const legacy = storage.getItem(key)
    if (legacy === null) continue
    const target = historyKey(key, `${MAIN_SEED}:0`)
    const legacyRows = parseList(legacy)
    const targetRows = parseList(storage.getItem(target))
    if (legacyRows === undefined || targetRows === undefined) {
      const aside = `${key}:corrupt-${Date.now()}`
      storage.setItem(aside, legacy)
      storage.removeItem(key)
      problems.push(`History in ${key} could not be read; it was kept as ${aside}`)
      continue
    }
    const known = new Set(targetRows.map((row) => row[id]))
    targetRows.push(...legacyRows.filter((row) => !known.has(row[id])))
    storage.setItem(target, JSON.stringify(targetRows))
    storage.removeItem(key)
  }
  return problems
}

const errorText = (error: unknown) => (error instanceof Error ? error.message : String(error))

export function useAccounts() {
  const accounts = useAccountsStore()
  const neptune = useNeptuneStore()

  const network = () => (neptune.getNetwork === 'mainnet' ? 'main' : 'testnet')

  const deriveSeedAccounts = async (registry: Registry, seedId: string, phrase: string[]) => {
    const mine = epoch
    const seedAccounts = registry.accounts.filter((a) => a.seedId === seedId)
    const derived = await Promise.all(
      seedAccounts.map((a) => walletApi.deriveAccountKeys(phrase, a.index, network())),
    )
    ensureOpen(mine)
    seedAccounts.forEach((a, i) => accounts.setKeys(a.id, derived[i]))
  }

  /** Mirrors the active account into the wallet store that the existing views read. */
  const applyActive = () => {
    const active = accounts.activeKeys
    if (!active) throw new Error('The active account has no keys')
    const id = accounts.requireActiveId()
    neptune.setSpendingKey(active.generationKeyHex)
    neptune.setGenerationSpendingKey(active.dctidhKeyHex)
    neptune.setGeneration_address(active.generationAddress)
    neptune.setDctidh_address(active.dctidhAddress)
    neptune.setAddress(active.generationAddress)
    neptune.setUtxos(accounts.utxosOf(id))
    const balance = accounts.balances.get(id)
    neptune.setBalance(balance?.available ?? null)
    neptune.setPendingBalance(balance?.pending ?? null)
  }

  /** Scans the whole chain for accounts the regular scans have not covered yet. */
  const scanNewAccounts = async (accountIds: string[]) => {
    const mine = epoch
    const refs = accounts.keyRefs.filter((r) => accountIds.includes(r.accountId))
    if (refs.length !== accountIds.length) throw new Error('Some new accounts have no keys')
    const tip = (await API.getBlockHeight())?.result?.height
    if (!tip) throw new Error('Failed to get block height')
    const result = await walletApi.scanUtxosClientSide({
      accounts: refs,
      isolated: true,
      startBlock: 0,
      endBlock: tip,
    })
    if (!result?.success) throw new Error(result?.error || 'Scan failed')
    ensureOpen(mine)
    accounts.setAccountUtxos(
      refs.map((r) => r.accountId),
      result.utxos.map(normalizeUtxo),
    )
  }

  const lock = () => {
    epoch++
    accounts.clear()
    seedPhrases.clear()
  }

  /** Only the main seed failing fails the unlock, which then keeps nothing. */
  const unlockAll = async (password: string): Promise<{ historyError?: string }> => {
    lock()
    const mine = epoch
    const step = async <T>(pending: Promise<T>): Promise<T> => {
      const value = await pending
      ensureOpen(mine)
      return value
    }
    try {
      let registry = await step(walletApi.accountsLoad())
      accounts.setRegistry(registry)
      const main = registry.seeds.find((s) => s.id === MAIN_SEED)
      if (!main) throw new Error('The registry has no main seed')
      neptune.setKeystoreFileName(main.keystoreFile)
      for (const seed of registry.seeds) {
        try {
          const phrase = await step(walletApi.accountsSeedPhrase(seed.id, password))
          seedPhrases.set(seed.id, phrase)
          await step(deriveSeedAccounts(registry, seed.id, phrase))
        } catch (error) {
          if (seed.id === MAIN_SEED || epoch !== mine) throw error
          seedPhrases.delete(seed.id)
          accounts.setLocked(seed.id, errorText(error))
        }
      }
      if (!accounts.activeKeys) {
        console.warn(`[accounts] Active account ${registry.active} is locked; using Account 1`)
        registry = await step(walletApi.accountsSetActive(`${MAIN_SEED}:0`))
        accounts.setRegistry(registry)
      }
      const problems = migrateLegacyStorage(window.localStorage)
      applyActive()
      return problems.length === 0 ? {} : { historyError: problems.join('; ') }
    } catch (error) {
      if (epoch === mine) lock()
      throw error
    }
  }

  const seedPhrase = (seedId: string): string[] => {
    const phrase = seedPhrases.get(seedId)
    if (!phrase) throw new Error(`Seed ${seedId} is not open`)
    return phrase
  }

  /** The first scan of new accounts; its failure is returned, since the accounts stay. */
  const firstScan = async (accountIds: string[]): Promise<string | undefined> => {
    try {
      await scanNewAccounts(accountIds)
      return undefined
    } catch (error) {
      return `${errorText(error)}. A Rescan on the UTXO tab finds their coins.`
    }
  }

  const createAccount = async (
    seedId: string,
  ): Promise<{ entry: AccountEntry; scanError?: string }> => {
    const mine = epoch
    const phrase = seedPhrase(seedId)
    const indices = accounts.accounts.filter((a) => a.seedId === seedId).map((a) => a.index)
    if (indices.length === 0) throw new Error(`Seed ${seedId} has no account`)
    const entry = await walletApi.accountsAdd(seedId, Math.max(...indices) + 1)
    const registry = await walletApi.accountsLoad()
    ensureOpen(mine)
    accounts.setRegistry(registry)
    const keys = await walletApi.deriveAccountKeys(phrase, entry.index, network())
    ensureOpen(mine)
    accounts.setKeys(entry.id, keys)
    const scanError = await firstScan([entry.id])
    return scanError === undefined ? { entry } : { entry, scanError }
  }

  /** A failed discovery or first scan is returned: the seed stays imported. */
  const importSeed = async (
    source: ImportSource,
    walletPassword: string,
  ): Promise<{ seed: SeedEntry; discoveryError?: string; scanError?: string }> => {
    const mine = epoch
    const seed = await walletApi.accountsImport(source, walletPassword)
    const registry = await walletApi.accountsLoad()
    ensureOpen(mine)
    accounts.setRegistry(registry)
    const phrase = await walletApi.accountsSeedPhrase(seed.id, walletPassword)
    ensureOpen(mine)
    seedPhrases.set(seed.id, phrase)
    await deriveSeedAccounts(registry, seed.id, phrase)
    let discoveryError: string | undefined
    try {
      await discoverAccounts(seed.id, walletPassword)
    } catch (error) {
      discoveryError = errorText(error)
    }
    const scanError = await firstScan(
      accounts.accounts.filter((a) => a.seedId === seed.id).map((a) => a.id),
    )
    return {
      seed,
      ...(discoveryError === undefined ? {} : { discoveryError }),
      ...(scanError === undefined ? {} : { scanError }),
    }
  }

  const discoverAccounts = async (seedId: string, walletPassword: string) => {
    const mine = epoch
    const added = await walletApi.accountsDiscover(seedId, walletPassword)
    const registry = await walletApi.accountsLoad()
    ensureOpen(mine)
    accounts.setRegistry(registry)
    await deriveSeedAccounts(registry, seedId, seedPhrase(seedId))
    return added
  }

  const switchTo = async (id: string) => {
    const mine = epoch
    accounts.findAccount(id)
    if (!accounts.keys.has(id)) throw new Error('This account is locked')
    const registry = await walletApi.accountsSetActive(id)
    ensureOpen(mine)
    accounts.setRegistry(registry)
    applyActive()
  }

  const removeSeed = async (seedId: string) => {
    const mine = epoch
    const removed = accounts.accounts.filter((a) => a.seedId === seedId).map((a) => a.id)
    // A locked group has no keys: its cached UTXOs can't be matched, but its records still go.
    const refs = removed.map(
      (id) =>
        accounts.keyRefs.find((r) => r.accountId === id) ?? { accountId: id, spendingKeysHex: [] },
    )
    let failure: unknown
    let registry: Registry
    try {
      registry = await walletApi.accountsRemoveSeed(seedId)
    } catch (error) {
      // The registry may already be saved without the seed: then clean up anyway.
      failure = error
      registry = await walletApi.accountsLoad()
    }
    ensureOpen(mine)
    accounts.setRegistry(registry)
    if (registry.seeds.some((s) => s.id === seedId)) throw failure
    await walletApi.forgetAccounts(refs)
    ensureOpen(mine)
    seedPhrases.delete(seedId)
    accounts.lockedSeeds.delete(seedId)
    accounts.forgetAccounts(removed)
    for (const id of removed) {
      for (const { key } of LEGACY_HISTORY_KEYS) window.localStorage.removeItem(historyKey(key, id))
    }
    applyActive()
    if (failure !== undefined) throw failure
  }

  const rename = async (id: string, name: string | null) => {
    const mine = epoch
    const registry = await walletApi.accountsRename(id, name)
    ensureOpen(mine)
    accounts.setRegistry(registry)
  }

  const setHidden = async (id: string, hidden: boolean) => {
    const mine = epoch
    const registry = await walletApi.accountsSetHidden(id, hidden)
    ensureOpen(mine)
    accounts.setRegistry(registry)
  }

  return {
    unlockAll,
    lock,
    seedPhrase,
    createAccount,
    importSeed,
    discoverAccounts,
    scanNewAccounts,
    removeSeed,
    switchTo,
    rename,
    setHidden,
  }
}
