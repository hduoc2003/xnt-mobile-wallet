import type { TransactionDetail } from '@/types'
import { useTransactionStore } from '@/stores'

const TX_STORAGE_KEY = 'xnt_tx_history'

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

    return {
        loadTransactionsFromStorage,
        addTransaction,
        clearTransactions,
    }
}

