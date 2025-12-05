import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { WalletState } from '@/types'
import { DEFAULT_MIN_BLOCK_HEIGHT, toFiniteNumber } from '@/utils'

export const useNeptuneStore = defineStore('neptune', () => {
    const defaultNetwork = (import.meta.env.VITE_NODE_NETWORK || 'mainnet') as 'mainnet' | 'testnet'

    // ===== STATE =====
    const wallet = ref<WalletState>({
        receiverId: null,
        viewKey: null,
        spendingKey: null,
        address: null,
        network: defaultNetwork,
        balance: null,
        pendingBalance: null,
        utxos: [],
        minBlockHeight: null,
    })

    const keystoreFileName = ref<string | null>(null)

    // ===== SETTERS =====

    const setReceiverId = (receiverId: string | null) => {
        wallet.value.receiverId = receiverId
    }

    const setViewKey = (viewKey: string | null) => {
        wallet.value.viewKey = viewKey
    }

    const setSpendingKey = (spendingKey: string | null) => {
        wallet.value.spendingKey = spendingKey
    }

    const setAddress = (address: string | null) => {
        wallet.value.address = address
    }

    const setNetwork = (network: 'mainnet' | 'testnet') => {
        wallet.value.network = network
    }

    const setBalance = (balance: string | null) => {
        wallet.value.balance = balance
    }

    const setPendingBalance = (pendingBalance: string | null) => {
        wallet.value.pendingBalance = pendingBalance
    }

    const setUtxos = (utxos: any[]) => {
        wallet.value.utxos = utxos
    }

    const setMinBlockHeight = (minBlockHeight: number = DEFAULT_MIN_BLOCK_HEIGHT) => {
        wallet.value.minBlockHeight = toFiniteNumber(minBlockHeight)
    }

    const setWallet = (walletData: Partial<WalletState>) => {
        wallet.value = { ...wallet.value, ...walletData }
    }

    const setKeystoreFileName = (fileName: string | null) => {
        keystoreFileName.value = fileName
    }

    const clearWallet = () => {
        wallet.value = {
            receiverId: null,
            viewKey: null,
            spendingKey: null,
            address: null,
            network: defaultNetwork,
            balance: null,
            pendingBalance: null,
            utxos: [],
            minBlockHeight: null,
        }
    }

    // ===== GETTERS =====
    const getWallet = computed(() => wallet.value)
    const getReceiverId = computed(() => wallet.value.receiverId)
    const getViewKey = computed(() => wallet.value.viewKey)
    const getSpendingKey = computed(() => wallet.value.spendingKey)
    const getAddress = computed(() => wallet.value.address)
    const getNetwork = computed(() => wallet.value.network)
    const getBalance = computed(() => wallet.value.balance)
    const getPendingBalance = computed(() => wallet.value.pendingBalance)
    const getUtxos = computed(() => wallet.value.utxos)
    const getMinBlockHeight = computed(() => wallet.value.minBlockHeight ?? null)
    const hasWallet = computed(() => wallet.value.address !== null)
    const getKeystoreFileName = computed(() => keystoreFileName.value ?? null)
    return {
        getWallet,
        getReceiverId,
        getViewKey,
        getSpendingKey,
        getAddress,
        getNetwork,
        getBalance,
        getPendingBalance,
        getUtxos,
        getMinBlockHeight,
        hasWallet,
        getKeystoreFileName,
        setReceiverId,
        setViewKey,
        setSpendingKey,
        setAddress,
        setNetwork,
        setBalance,
        setPendingBalance,
        setUtxos,
        setMinBlockHeight,
        setWallet,
        setKeystoreFileName,
        clearWallet,
    }
})
