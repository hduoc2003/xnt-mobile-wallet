import type { TransactionDetail, TxStatus } from '@/types'
import { useTransactionStore } from '@/stores'

const TX_STORAGE_KEY = 'xnt_tx_history'
const NEPTUNE_SCAN_API = 'https://api.neptunescan.io/api/utxo'

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
    } catch {
      // Ignore storage errors
    }
  }

  const fetchTxStatus = async (txId: string): Promise<TxStatus> => {
    const digest = txId.startsWith('0x') ? txId.slice(2) : txId
    try {
      const response = await (window as any).walletApi.proxyHttpGet(`${NEPTUNE_SCAN_API}/${digest}`)
      if (!response?.success) return 'failed'
      const utxo = response.utxo
      if (!utxo) return 'failed'
      return utxo.in_mempool ? 'pending' : 'confirmed'
    } catch {
      return 'unknown'
    }
  }

  const refreshTransactionStatuses = async () => {
    const txs = transactionStore.getTransactions
    if (!txs.length) return

    const oneMinuteAgo = Date.now() - 60 * 1000

    await Promise.allSettled(
      txs.map(async (tx) => {
        // Leave recently sent transactions as "Unknown" — give the explorer
        // time to index the transaction before querying its status.
        if (tx.time && new Date(tx.time).getTime() > oneMinuteAgo) return
        const status = await fetchTxStatus(tx.id)
        transactionStore.updateTransactionStatus(tx.id, status)
      }),
    )

    // Persist updated statuses to localStorage
    try {
      window.localStorage.setItem(TX_STORAGE_KEY, JSON.stringify(transactionStore.getTransactions))
    } catch {
      // Ignore storage errors
    }
  }

  return {
    loadTransactionsFromStorage,
    addTransaction,
    clearTransactions,
    fetchTxStatus,
    refreshTransactionStatuses,
  }
}
