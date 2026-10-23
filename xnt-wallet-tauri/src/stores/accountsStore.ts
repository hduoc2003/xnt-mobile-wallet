import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import type { AccountEntry, AccountKeys, Registry } from '@/types'
import type { NormalizedUtxo } from '@/utils'

export const MAIN_SEED = 'main'

export interface AccountKeysRef {
  accountId: string
  spendingKeysHex: string[]
}

export interface AccountBalance {
  available: string
  /** Absent when the mempool could not be read. */
  pending?: string
}

export const useAccountsStore = defineStore('accounts', () => {
  const registry = ref<Registry | undefined>()
  const keys = ref(new Map<string, AccountKeys>())
  /** Seeds that did not open at unlock, with the reason. */
  const lockedSeeds = ref(new Map<string, string>())
  const utxosByAccount = ref(new Map<string, NormalizedUtxo[]>())
  /** Kept apart so no send, merge or bridge can select them. */
  const legacyDctidhUtxosByAccount = ref(new Map<string, NormalizedUtxo[]>())
  const balances = ref(new Map<string, AccountBalance>())

  const activeId = computed(() => registry.value?.active)
  const accounts = computed<AccountEntry[]>(() => registry.value?.accounts ?? [])
  const visibleAccounts = computed(() => accounts.value.filter((a) => !a.hidden))
  const activeKeys = computed(() =>
    activeId.value === undefined ? undefined : keys.value.get(activeId.value),
  )
  const keyRefs = computed<AccountKeysRef[]>(() =>
    [...keys.value].map(([accountId, k]) => ({
      accountId,
      spendingKeysHex: [k.generationKeyHex, k.dctidhKeyHex],
    })),
  )

  const requireActiveId = (): string => {
    if (activeId.value === undefined) throw new Error('No account is open')
    return activeId.value
  }

  const utxosOf = (id: string): NormalizedUtxo[] => utxosByAccount.value.get(id) ?? []
  const legacyDctidhUtxosOf = (id: string): NormalizedUtxo[] =>
    legacyDctidhUtxosByAccount.value.get(id) ?? []

  const setAccountUtxos = (accountIds: string[], utxos: NormalizedUtxo[]) => {
    const grouped = new Map<string, NormalizedUtxo[]>(accountIds.map((id) => [id, []]))
    const legacy = new Map<string, NormalizedUtxo[]>(accountIds.map((id) => [id, []]))
    for (const utxo of utxos) {
      const list =
        utxo.accountId === undefined
          ? undefined
          : (utxo.isLegacyDctidh ? legacy : grouped).get(utxo.accountId)
      if (!list) throw new Error(`UTXO ${utxo.utxoDigest} has no account from this request`)
      list.push(utxo)
    }
    grouped.forEach((list, id) => utxosByAccount.value.set(id, list))
    legacy.forEach((list, id) => legacyDctidhUtxosByAccount.value.set(id, list))
  }

  const forgetAccounts = (ids: string[]) => {
    for (const id of ids) {
      keys.value.delete(id)
      utxosByAccount.value.delete(id)
      legacyDctidhUtxosByAccount.value.delete(id)
      balances.value.delete(id)
    }
  }

  const setBalance = (id: string, balance: AccountBalance) => {
    balances.value.set(id, balance)
  }

  const findAccount = (id: string): AccountEntry => {
    const account = accounts.value.find((a) => a.id === id)
    if (!account) throw new Error(`Unknown account ${id}`)
    return account
  }

  /** The account among `ids` with `address` as its dCTIDH or Generation address, any case. */
  const accountWithAddress = (address: string, ids: string[]): string | undefined => {
    const wanted = address.trim().toLowerCase()
    return ids.find((id) => {
      const accountKeys = keys.value.get(id)
      return (
        accountKeys !== undefined &&
        [accountKeys.dctidhAddress, accountKeys.generationAddress].some(
          (own) => own.toLowerCase() === wanted,
        )
      )
    })
  }

  /** "Main seed" or "Imported N" (N = position among imported seeds). */
  const seedTitle = (seedId: string): string => {
    if (seedId === MAIN_SEED) return 'Main seed'
    const imported = (registry.value?.seeds ?? []).filter((s) => s.id !== MAIN_SEED)
    const position = imported.findIndex((s) => s.id === seedId) + 1
    if (position === 0) throw new Error(`Unknown seed ${seedId}`)
    return `Imported ${position}`
  }

  /** "Account N" (N = position in its seed group, hidden ones included, so names stay put). */
  const defaultName = (id: string): string => {
    const account = findAccount(id)
    const position =
      accounts.value
        .filter((a) => a.seedId === account.seedId)
        .sort((a, b) => a.index - b.index)
        .findIndex((a) => a.id === id) + 1
    return `Account ${position}`
  }

  /** The name within its seed group. */
  const shortName = (id: string): string => findAccount(id).name ?? defaultName(id)

  /** shortName with its seed title, for accounts listed outside their seed group. */
  const qualifiedName = (id: string): string =>
    `${seedTitle(findAccount(id).seedId)} · ${shortName(id)}`

  const displayName = (id: string): string => {
    const account = findAccount(id)
    if (account.name !== null || account.seedId === MAIN_SEED) return shortName(id)
    return `${seedTitle(account.seedId)} · ${shortName(id)}`
  }

  const setRegistry = (next: Registry) => {
    registry.value = next
  }

  const setKeys = (id: string, accountKeys: AccountKeys) => {
    keys.value.set(id, accountKeys)
  }

  const setLocked = (seedId: string, reason: string) => {
    lockedSeeds.value.set(seedId, reason)
  }

  const clear = () => {
    registry.value = undefined
    keys.value = new Map()
    lockedSeeds.value = new Map()
    utxosByAccount.value = new Map()
    legacyDctidhUtxosByAccount.value = new Map()
    balances.value = new Map()
  }

  return {
    registry,
    keys,
    lockedSeeds,
    activeId,
    accounts,
    visibleAccounts,
    activeKeys,
    keyRefs,
    utxosByAccount,
    balances,
    requireActiveId,
    utxosOf,
    legacyDctidhUtxosOf,
    setAccountUtxos,
    setBalance,
    forgetAccounts,
    findAccount,
    accountWithAddress,
    displayName,
    defaultName,
    shortName,
    qualifiedName,
    seedTitle,
    setRegistry,
    setKeys,
    setLocked,
    clear,
  }
})
