import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import type { AccountKeys, Registry } from '@/types'

vi.mock('@/api/tauri/walletApi', () => ({
  walletApi: {
    accountsLoad: vi.fn(),
    accountsSeedPhrase: vi.fn(),
    accountsSetActive: vi.fn(),
    deriveAccountKeys: vi.fn(),
    getPendingBalance: vi.fn(),
  },
}))

import { walletApi } from '@/api/tauri/walletApi'
import { useAccountsStore, useNeptuneStore } from '@/stores'
import { migrateLegacyStorage, useAccounts } from './useAccounts'
import { useNeptuneWallet } from './useNeptuneWallet'
import { useTransaction } from './useTransaction'
import { useTransactionStore } from '@/stores'
import type { NormalizedUtxo } from '@/utils'
import type { TransactionDetail } from '@/types'

const api = vi.mocked(walletApi)

const registry: Registry = {
  seeds: [
    { id: 'main', keystoreFile: 'k.json', source: 'created', backedUp: true },
    { id: 'imp-1', keystoreFile: 'imp-1.json', source: 'phrase', backedUp: false },
  ],
  accounts: [
    { id: 'main:0', seedId: 'main', index: 0, name: null, hidden: false },
    { id: 'main:1', seedId: 'main', index: 1, name: null, hidden: false },
    { id: 'imp-1:0', seedId: 'imp-1', index: 0, name: null, hidden: false },
  ],
  active: 'main:1',
}

const keysFor = (phrase: string[], index: number): AccountKeys => ({
  index,
  generationKeyHex: `gen-${phrase[0]}-${index}`,
  dctidhKeyHex: `dctidh-${phrase[0]}-${index}`,
  generationAddress: `xntgen-${phrase[0]}-${index}`,
  dctidhAddress: `xntdctidh-${phrase[0]}-${index}`,
})

const baseUtxo: NormalizedUtxo = {
  utxoDigest: 'a',
  amount: '0',
  blockHeight: 5,
  additionRecord: '0x0',
  isTimelocked: false,
  releaseDateMs: null,
  keyType: 'Generation',
  isPending: false,
  isLegacyDctidh: false,
}

const sentTx = (id: string): TransactionDetail => ({
  id,
  toAddress: 'xnt1',
  amount: 1,
  fee: 0.1,
  time: 't',
})

class MemoryStorage {
  items = new Map<string, string>()
  getItem = (key: string) => this.items.get(key) ?? null
  setItem = (key: string, value: string) => void this.items.set(key, value)
  removeItem = (key: string) => void this.items.delete(key)
  get length() {
    return this.items.size
  }
  key = (i: number) => [...this.items.keys()][i] ?? null
}

describe('useAccounts', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.resetAllMocks()
    const storage = new MemoryStorage()
    vi.stubGlobal('window', { localStorage: storage })
    vi.stubGlobal('localStorage', storage)
    api.accountsLoad.mockResolvedValue(registry)
    api.deriveAccountKeys.mockImplementation(async (phrase, index) => keysFor(phrase, index))
    api.accountsSeedPhrase.mockImplementation(async (seedId) => [seedId])
  })

  it('unlockAll derives keys for every account', async () => {
    await useAccounts().unlockAll('pw')
    const store = useAccountsStore()
    expect([...store.keys.keys()]).toEqual(['main:0', 'main:1', 'imp-1:0'])
    expect(store.keys.get('imp-1:0')?.generationKeyHex).toBe('gen-imp-1-0')
    expect(useNeptuneStore().getSpendingKey).toBe('gen-main-1')
    expect(useNeptuneStore().getGenerationSpendingKey).toBe('dctidh-main-1')
  })

  it('a locked imported seed does not block unlock', async () => {
    api.accountsSeedPhrase.mockImplementation(async (seedId) => {
      if (seedId === 'imp-1') throw new Error('Invalid password')
      return [seedId]
    })
    await useAccounts().unlockAll('pw')
    const store = useAccountsStore()
    expect(store.lockedSeeds.get('imp-1')).toBe('Invalid password')
    expect(store.keys.has('imp-1:0')).toBe(false)
    expect(store.activeId).toBe('main:1')
  })

  it('the main seed failing fails the unlock', async () => {
    api.accountsSeedPhrase.mockRejectedValue(new Error('Invalid password'))
    await expect(useAccounts().unlockAll('nope')).rejects.toThrow('Invalid password')
  })

  it('a locked active account falls back to Account 1', async () => {
    const onImported: Registry = { ...registry, active: 'imp-1:0' }
    api.accountsLoad.mockResolvedValue(onImported)
    api.accountsSetActive.mockResolvedValue({ ...registry, active: 'main:0' })
    api.accountsSeedPhrase.mockImplementation(async (seedId) => {
      if (seedId === 'imp-1') throw new Error('Invalid password')
      return [seedId]
    })
    await useAccounts().unlockAll('pw')
    expect(api.accountsSetActive).toHaveBeenCalledWith('main:0')
    expect(useNeptuneStore().getSpendingKey).toBe('gen-main-0')
  })

  it('switchTo rejects unknown id', async () => {
    await useAccounts().unlockAll('pw')
    await expect(useAccounts().switchTo('main:9')).rejects.toThrow('Unknown account main:9')
    expect(api.accountsSetActive).not.toHaveBeenCalled()
  })

  it('lock clears keys and seed words', async () => {
    const { unlockAll, lock, seedPhrase } = useAccounts()
    await unlockAll('pw')
    lock()
    expect(useAccountsStore().keys.size).toBe(0)
    expect(() => seedPhrase('main')).toThrow('Seed main is not open')
  })

  it('a failed unlock keeps nothing', async () => {
    api.deriveAccountKeys.mockRejectedValue(new Error('derive failed'))
    const { unlockAll, seedPhrase } = useAccounts()
    await expect(unlockAll('pw')).rejects.toThrow('derive failed')
    expect(useAccountsStore().registry).toBeUndefined()
    expect(() => seedPhrase('main')).toThrow()
  })

  it('a lock during unlock stops it', async () => {
    let release: (phrase: string[]) => void = () => {}
    api.accountsSeedPhrase.mockReturnValueOnce(new Promise((resolve) => (release = resolve)))
    const { unlockAll, lock } = useAccounts()
    const unlocking = unlockAll('pw')
    await Promise.resolve()
    await Promise.resolve()
    lock()
    release(['main'])
    await expect(unlocking).rejects.toThrow('locked meanwhile')
    expect(useAccountsStore().keys.size).toBe(0)
    expect(useNeptuneStore().getSpendingKey).toBeNull()
  })

  it('switchTo refuses an account of a locked seed', async () => {
    api.accountsSeedPhrase.mockImplementation(async (seedId) => {
      if (seedId === 'imp-1') throw new Error('Invalid password')
      return [seedId]
    })
    await useAccounts().unlockAll('pw')
    await expect(useAccounts().switchTo('imp-1:0')).rejects.toThrow('This account is locked')
  })

  it('the wallet store shows only the active account', async () => {
    const { unlockAll, switchTo } = useAccounts()
    await unlockAll('pw')
    api.accountsSetActive.mockResolvedValue({ ...registry, active: 'main:0' })
    const utxo = (accountId: string, amount: string): NormalizedUtxo => ({
      utxoDigest: `${accountId}-${amount}`,
      amount,
      blockHeight: 5,
      additionRecord: `0x${amount}`,
      isTimelocked: false,
      releaseDateMs: null,
      keyType: 'Generation',
      isPending: false,
      isLegacyDctidh: false,
      accountId,
    })
    useAccountsStore().setAccountUtxos(
      ['main:0', 'main:1'],
      [utxo('main:0', '1'), utxo('main:1', '2'), utxo('main:1', '3')],
    )
    await switchTo('main:0')
    expect(useNeptuneStore().getUtxos?.map((u) => u.amount)).toEqual(['1'])
    const { selectUtxos } = useNeptuneWallet()
    const selection = selectUtxos(useNeptuneStore().getUtxos ?? [], 0.5, 0.1, 5, 'main:0')
    expect(selection.selectedUtxos.map((u) => u.amount)).toEqual(['1'])
    expect(selection.spendingKeyHex).toBe('gen-main-0')
    // A send captured for main:1 keeps main:1's keys after the view switched away.
    const captured = selectUtxos(useAccountsStore().utxosOf('main:1'), 2.5, 0.1, 5, 'main:1')
    expect(captured.spendingKeyHex).toBe('gen-main-1')
  })

  it('balances are split by account', async () => {
    await useAccounts().unlockAll('pw')
    const store = useAccountsStore()
    store.setAccountUtxos(
      ['main:0', 'main:1', 'imp-1:0'],
      [
        { ...baseUtxo, accountId: 'main:0', amount: '1' },
        { ...baseUtxo, accountId: 'main:1', amount: '2', utxoDigest: 'b' },
      ],
    )
    api.getPendingBalance.mockResolvedValue({
      success: true,
      pending_balance: '5',
      by_account: [
        { account_id: 'main:0', pending: '0' },
        { account_id: 'main:1', pending: '4' },
        { account_id: 'imp-1:0', pending: '1' },
      ],
    })
    const active = await useNeptuneWallet().getBalance()
    expect(active.balance).toBe('2.00000000')
    expect(active.pendingBalance).toBe('4.00000000')
    expect(store.balances.get('main:0')).toEqual({ available: '1.00000000', pending: '0.00000000' })
    expect(store.balances.get('imp-1:0')).toEqual({
      available: '0.00000000',
      pending: '1.00000000',
    })
  })

  it('legacy dCTIDH UTXOs stay out of the spendable list and the balance', async () => {
    await useAccounts().unlockAll('pw')
    const store = useAccountsStore()
    store.setAccountUtxos(
      ['main:0', 'main:1'],
      [
        { ...baseUtxo, accountId: 'main:1', amount: '2' },
        { ...baseUtxo, accountId: 'main:1', amount: '7', utxoDigest: 'b', isLegacyDctidh: true },
      ],
    )
    expect(store.utxosOf('main:1').map((u) => u.amount)).toEqual(['2'])
    expect(store.legacyDctidhUtxosOf('main:1').map((u) => u.amount)).toEqual(['7'])
    expect(store.legacyDctidhUtxosOf('main:0')).toEqual([])
    api.getPendingBalance.mockResolvedValue({ success: false, error: 'offline' })
    expect((await useNeptuneWallet().getBalance()).balance).toBe('2.00000000')
    store.forgetAccounts(['main:1'])
    expect(store.legacyDctidhUtxosOf('main:1')).toEqual([])
  })

  it('History records a send under the account that made it, not the shown one', async () => {
    await useAccounts().unlockAll('pw')
    const { addTransaction } = useTransaction()
    addTransaction(sentTx('a'), 'main:0')
    expect(useTransactionStore().getTransactions).toEqual([])
    expect(JSON.parse(window.localStorage.getItem('xnt_tx_history:main:0') ?? '[]')).toEqual([
      sentTx('a'),
    ])
    addTransaction(sentTx('b'), 'main:1')
    expect(useTransactionStore().getTransactions).toEqual([sentTx('b')])
  })

  it('history is namespaced by account', async () => {
    window.localStorage.setItem('xnt_tx_history:main:0', JSON.stringify([sentTx('a')]))
    window.localStorage.setItem('xnt_tx_history:main:1', JSON.stringify([sentTx('b')]))
    const { unlockAll, switchTo } = useAccounts()
    await unlockAll('pw')
    const { loadTransactionsFromStorage } = useTransaction()
    expect(loadTransactionsFromStorage()).toEqual([sentTx('b')])
    api.accountsSetActive.mockResolvedValue({ ...registry, active: 'main:0' })
    await switchTo('main:0')
    expect(loadTransactionsFromStorage()).toEqual([sentTx('a')])
  })

  it('switching to an account without History clears the list', async () => {
    window.localStorage.setItem('xnt_tx_history:main:1', JSON.stringify([sentTx('b')]))
    const { unlockAll, switchTo } = useAccounts()
    await unlockAll('pw')
    const { loadTransactionsFromStorage } = useTransaction()
    loadTransactionsFromStorage()
    api.accountsSetActive.mockResolvedValue({ ...registry, active: 'main:0' })
    await switchTo('main:0')
    expect(loadTransactionsFromStorage()).toEqual([])
    expect(useTransactionStore().getTransactions).toEqual([])
  })

  it('corrupt History is an error, not overwritten', async () => {
    window.localStorage.setItem('xnt_tx_history:main:1', '{"not":"a list"}')
    await useAccounts().unlockAll('pw')
    const { addTransaction, loadTransactionsFromStorage } = useTransaction()
    expect(() => addTransaction(sentTx('a'), 'main:1')).toThrow('corrupt')
    expect(() => loadTransactionsFromStorage()).toThrow('corrupt')
    expect(window.localStorage.getItem('xnt_tx_history:main:1')).toBe('{"not":"a list"}')
  })

  it('an unreadable mempool leaves pending unknown', async () => {
    await useAccounts().unlockAll('pw')
    api.getPendingBalance.mockResolvedValue({ success: false, error: 'node down' })
    const active = await useNeptuneWallet().getBalance()
    expect(active.pendingBalance).toBeUndefined()
    expect(useAccountsStore().balances.get('main:1')?.pending).toBeUndefined()
  })

  it('migrating keeps rows from both legacy and namespaced history', () => {
    const storage = new MemoryStorage()
    storage.setItem('xnt_tx_history:main:0', JSON.stringify([{ id: 'a' }]))
    storage.setItem('xnt_tx_history', JSON.stringify([{ id: 'a' }, { id: 'b' }]))
    migrateLegacyStorage(storage)
    expect(JSON.parse(storage.getItem('xnt_tx_history:main:0') ?? '')).toEqual([
      { id: 'a' },
      { id: 'b' },
    ])
    expect(storage.getItem('xnt_tx_history')).toBeNull()
  })

  it('migration sets a corrupt key aside and still moves the other', () => {
    const storage = new MemoryStorage()
    storage.setItem('xnt_tx_history', 'not json')
    storage.setItem('xnt_received_history', JSON.stringify([{ commitment: 'c' }, null]))
    const problems = migrateLegacyStorage(storage)
    expect(problems).toHaveLength(1)
    expect(storage.getItem('xnt_tx_history')).toBeNull()
    const aside = [...storage.items.keys()].find((k) => k.startsWith('xnt_tx_history:corrupt-'))
    expect(aside && storage.getItem(aside)).toBe('not json')
    expect(JSON.parse(storage.getItem('xnt_received_history:main:0') ?? '')).toEqual([
      { commitment: 'c' },
    ])
    expect(migrateLegacyStorage(storage)).toEqual([])
  })

  it('migrates legacy history keys', () => {
    const storage = new MemoryStorage()
    storage.setItem('xnt_tx_history', '[{"id":"a"}]')
    storage.setItem('xnt_received_history', '[{"commitment":"c"}]')
    expect(migrateLegacyStorage(storage)).toEqual([])
    expect([...storage.items]).toEqual([
      ['xnt_tx_history:main:0', '[{"id":"a"}]'],
      ['xnt_received_history:main:0', '[{"commitment":"c"}]'],
    ])
    migrateLegacyStorage(storage)
    expect(storage.items.size).toBe(2)
    expect(storage.getItem('xnt_tx_history:main:0')).toBe('[{"id":"a"}]')
  })
})
