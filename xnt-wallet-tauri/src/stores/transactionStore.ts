import type { TransactionDetail, TxStatus } from '@/types'
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export const useTransactionStore = defineStore('transaction', () => {
    // ===== STATE =====
    const transactions = ref<TransactionDetail[]>([])

    // ===== SETTERS =====
    const setTransactions = (items: TransactionDetail[]) => {
        transactions.value = items
    }

    const addTransaction = (item: TransactionDetail) => {
        transactions.value = [item, ...transactions.value]
    }

    const updateTransactionStatus = (id: string, status: TxStatus) => {
        const idx = transactions.value.findIndex(t => t.id === id)
        if (idx !== -1) {
            transactions.value[idx] = { ...transactions.value[idx], status }
        }
    }

    const clearTransactions = () => {
        transactions.value = []
    }

    // ===== GETTERS =====
    const getTransactions = computed(() => transactions.value)

    return {
        getTransactions,
        setTransactions,
        addTransaction,
        updateTransactionStatus,
        clearTransactions,
    }
})
