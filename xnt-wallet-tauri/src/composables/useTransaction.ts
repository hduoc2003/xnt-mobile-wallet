import type { HistoryStatus, ReceivedEntry, TransactionDetail } from '@/types'
import { useTransactionStore } from '@/stores'
import { callJsonRpc } from '@/api/neptuneApi'
import type { NormalizedUtxo } from '@/utils'

const TX_STORAGE_KEY = 'xnt_tx_history'
const RECEIVED_STORAGE_KEY = 'xnt_received_history'

/** Mined is final; pending and dropped are re-asked, since a dropped tx can still be mined. */
const isResolved = (status: TransactionDetail['status']) =>
  typeof status === 'object' && status.state === 'confirmed'

export const useTransaction = () => {
  const transactionStore = useTransactionStore()

  const loadTransactionsFromStorage = (): TransactionDetail[] => {
    if (typeof window === 'undefined') return []
    try {
      const raw = window.localStorage.getItem(TX_STORAGE_KEY)
      if (!raw) return []
      const parsed = JSON.parse(raw)
      if (Array.isArray(parsed)) {
        transactionStore.setTransactions(parsed)
        return parsed
      }
      return []
    } catch {
      return []
    }
  }

  const addTransaction = (transactions: TransactionDetail) => {
    if (typeof window === 'undefined') return
    try {
      transactionStore.addTransaction(transactions)
      window.localStorage.setItem(TX_STORAGE_KEY, JSON.stringify(transactionStore.getTransactions))
    } catch {
      // Ignore storage errors
    }
  }

  const clearTransactions = () => {
    if (typeof window === 'undefined') return
    try {
      transactionStore.clearTransactions()
      window.localStorage.removeItem(TX_STORAGE_KEY)
      window.localStorage.removeItem(RECEIVED_STORAGE_KEY)
    } catch {
      // Ignore storage errors
    }
  }

  const persistTransactions = () => {
    try {
      window.localStorage.setItem(TX_STORAGE_KEY, JSON.stringify(transactionStore.getTransactions))
    } catch {
      // Ignore storage errors
    }
  }

  /** Asks our node for every sent tx whose status isn't final yet. */
  const refreshTransactionStatuses = async () => {
    const unresolved = transactionStore.getTransactions.filter((tx) => !isResolved(tx.status))
    if (!unresolved.length) return
    const statuses: HistoryStatus[] = await (window as any).walletApi.txStatuses(
      unresolved.map((tx) => tx.id),
    )
    if (statuses.length !== unresolved.length) {
      throw new Error(`Node returned ${statuses.length} statuses for ${unresolved.length} sends`)
    }
    unresolved.forEach((tx, i) => transactionStore.updateTransactionStatus(tx.id, statuses[i]))
    persistTransactions()
  }

  const loadReceived = (): ReceivedEntry[] => {
    const raw = window.localStorage.getItem(RECEIVED_STORAGE_KEY)
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
   * own sends and deposits (change) are skipped. A failed block-time lookup skips that UTXO
   * until the next sync and is reported in `errors`.
   */
  const syncReceived = async (utxos: NormalizedUtxo[], ownOutputs: Set<string>) => {
    const received = loadReceived()
    const known = new Set(received.map((r) => r.commitment))
    const times = new Map<number, string>()
    const errors: string[] = []
    for (const utxo of utxos) {
      const commitment = utxo.utxoDigest
      if (
        !commitment ||
        utxo.blockHeight <= 0 ||
        known.has(commitment) ||
        ownOutputs.has(commitment)
      ) {
        continue
      }
      try {
        const time = times.get(utxo.blockHeight) ?? (await blockTime(utxo.blockHeight))
        times.set(utxo.blockHeight, time)
        received.push({ commitment, amount: utxo.amount, blockHeight: utxo.blockHeight, time })
        known.add(commitment)
      } catch (error: unknown) {
        errors.push(error instanceof Error ? error.message : String(error))
      }
    }
    window.localStorage.setItem(RECEIVED_STORAGE_KEY, JSON.stringify(received))
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
