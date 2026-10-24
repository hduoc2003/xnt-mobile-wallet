import type { HistoryStatus, ReceivedEntry, TransactionDetail } from '@/types'
import { useAccountsStore, useTransactionStore } from '@/stores'
import { callJsonRpc } from '@/api/neptuneApi'
import type { NormalizedUtxo } from '@/utils'
import { historyKey } from './useAccounts'

const TX_STORAGE_KEY = 'xnt_tx_history'
const RECEIVED_STORAGE_KEY = 'xnt_received_history'

/** Mined is final; pending and dropped are re-asked, since a dropped tx can still be mined. */
const isResolved = (status: TransactionDetail['status']) =>
  typeof status === 'object' && status.state === 'confirmed'

/** An account's sends; corrupt storage is an error, never an empty list to write over. */
export const readSent = (accountId: string): TransactionDetail[] => {
  const key = historyKey(TX_STORAGE_KEY, accountId)
  const raw = window.localStorage.getItem(key)
  if (!raw) return []
  const parsed: unknown = JSON.parse(raw)
  if (!Array.isArray(parsed)) throw new Error(`History in ${key} is corrupt`)
  return parsed
}

/** Each account keeps its own History; the store shows the active account's. */
export const useTransaction = () => {
  const transactionStore = useTransactionStore()
  const accountsStore = useAccountsStore()

  const receivedKey = () => historyKey(RECEIVED_STORAGE_KEY, accountsStore.requireActiveId())

  /** Shows the active account's sends; the list is replaced even when it has none. */
  const loadTransactionsFromStorage = (): TransactionDetail[] => {
    if (typeof window === 'undefined') return []
    transactionStore.setTransactions([])
    const sent = readSent(accountsStore.requireActiveId())
    transactionStore.setTransactions(sent)
    return sent
  }

  /** `accountId` is the account that sent, captured with its UTXOs. Throws if storage fails. */
  const addTransaction = (transaction: TransactionDetail, accountId: string) => {
    if (typeof window === 'undefined') return
    const sent = [transaction, ...readSent(accountId)]
    window.localStorage.setItem(historyKey(TX_STORAGE_KEY, accountId), JSON.stringify(sent))
    if (accountId === accountsStore.activeId) transactionStore.setTransactions(sent)
  }

  /** Forgets every account's History (a new wallet replaced the old one). */
  const clearTransactions = () => {
    if (typeof window === 'undefined') return
    transactionStore.clearTransactions()
    const storage = window.localStorage
    const keys = Array.from({ length: storage.length }, (_, i) => storage.key(i))
    for (const key of keys) {
      if (
        key !== null &&
        [TX_STORAGE_KEY, RECEIVED_STORAGE_KEY].some(
          (base) => key === base || key.startsWith(`${base}:`),
        )
      ) {
        storage.removeItem(key)
      }
    }
  }

  const refreshTransactionStatuses = async () => {
    const accountId = accountsStore.requireActiveId()
    const unresolved = readSent(accountId).filter((tx) => !isResolved(tx.status))
    if (!unresolved.length) return
    const statuses: HistoryStatus[] = await (window as any).walletApi.txStatuses(
      unresolved.map((tx) => tx.id),
    )
    if (statuses.length !== unresolved.length) {
      throw new Error(`Node returned ${statuses.length} statuses for ${unresolved.length} sends`)
    }
    const byId = new Map(unresolved.map((tx, i) => [tx.id, statuses[i]]))
    const sent = readSent(accountId).map((tx) => {
      const status = byId.get(tx.id)
      return status === undefined ? tx : { ...tx, status }
    })
    window.localStorage.setItem(historyKey(TX_STORAGE_KEY, accountId), JSON.stringify(sent))
    if (accountId === accountsStore.activeId) transactionStore.setTransactions(sent)
  }

  const loadReceived = (key = receivedKey()): ReceivedEntry[] => {
    const raw = window.localStorage.getItem(key)
    if (!raw) return []
    const parsed: unknown = JSON.parse(raw)
    if (!Array.isArray(parsed)) throw new Error('Received history in local storage is corrupt')
    return parsed
  }

  const blockTime = async (height: number): Promise<string> => {
    const response = await callJsonRpc('archival_getBlockHeader', [height])
    const timestamp = response?.result?.header?.timestamp
    if (timestamp === undefined) {
      throw new Error(`No header for block ${height}: ${JSON.stringify(response?.error)}`)
    }
    return new Date(Number(timestamp)).toISOString()
  }

  /**
   * Appends newly scanned UTXOs to the received log; spent ones stay listed. Outputs of our
   * own sends and deposits (change) are skipped. Bridge payouts get their Leviathan burn serial.
   * A failed block-time or burn lookup skips that UTXO until the next sync and is reported in
   * `errors`.
   */
  const syncReceived = async (
    utxos: NormalizedUtxo[],
    ownOutputs: Set<string>,
    accountId: string,
  ) => {
    const key = historyKey(RECEIVED_STORAGE_KEY, accountId)
    const received = loadReceived(key)
    const known = new Set(received.map((r) => r.utxoId ?? r.commitment))
    const times = new Map<number, string>()
    const errors: string[] = []
    for (const utxo of utxos) {
      const commitment = utxo.utxoDigest
      if (
        !commitment ||
        utxo.blockHeight <= 0 ||
        known.has(utxo.utxoId) ||
        known.has(commitment) ||
        ownOutputs.has(commitment)
      ) {
        continue
      }
      try {
        const time = times.get(utxo.blockHeight) ?? (await blockTime(utxo.blockHeight))
        times.set(utxo.blockHeight, time)
        const bridgeSerial: string | null = await (window as any).walletApi.bridgeBurnSerial(
          commitment,
        )
        received.push({
          utxoId: utxo.utxoId,
          commitment,
          amount: utxo.amount,
          blockHeight: utxo.blockHeight,
          time,
          ...(bridgeSerial === null ? {} : { bridgeSerial }),
        })
        known.add(utxo.utxoId)
      } catch (error: unknown) {
        errors.push(error instanceof Error ? error.message : String(error))
      }
    }
    window.localStorage.setItem(key, JSON.stringify(received))
    return { received, errors }
  }

  return {
    loadTransactionsFromStorage,
    addTransaction,
    clearTransactions,
    refreshTransactionStatuses,
    loadReceived,
    syncReceived,
  }
}
