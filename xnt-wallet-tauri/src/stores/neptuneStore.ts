import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { WalletState } from '@/types'
import { DEFAULT_MIN_BLOCK_HEIGHT, toFiniteNumber } from '@/utils'

export const useNeptuneStore = defineStore('neptune', () => {
    const defaultNetwork = (import.meta.env.VITE_NODE_NETWORK || 'mainnet') as 'mainnet' | 'testnet'

    // ===== STATE =====
    const wallet = ref<WalletState>({
      receiverId: null,
      spendingKey: null,
      generationSpendingKey: null,
      address: null,
      dctidh_address: null,
      generation_address: null,
      network: defaultNetwork,
      balance: null,
      pendingBalance: null,
      utxos: [],
      minBlockHeight: null,
    })

    const keystoreFileName = ref<string | null>(null)
    const rpcUrl = ref<string | null>(null)
    const addressType = ref<'dctidh' | 'generation'>('generation')

    // ===== SETTERS =====

    const setReceiverId = (receiverId: string | null) => {
        wallet.value.receiverId = receiverId
    }

    const setSpendingKey = (spendingKey: string | null) => {
        wallet.value.spendingKey = spendingKey
    }

    const setGenerationSpendingKey = (key: string | null) => {
        wallet.value.generationSpendingKey = key
    }

    const setAddress = (address: string | null) => {
        wallet.value.address = address
    }

    const setDctidh_address = (address: string | null) => {
        wallet.value.dctidh_address = address
    }

    const setGeneration_address = (address: string | null) => {
        wallet.value.generation_address = address
    }

    const setAddressType = (type: 'dctidh' | 'generation') => {
        addressType.value = type
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

    const setRpcUrl = (url: string | null) => {
        rpcUrl.value = url
    }

    const clearWallet = () => {
      wallet.value = {
        receiverId: null,
        spendingKey: null,
        generationSpendingKey: null,
        address: null,
        dctidh_address: null,
        generation_address: null,
        network: defaultNetwork,
        balance: null,
        pendingBalance: null,
        utxos: [],
        minBlockHeight: null,
      }
      addressType.value = 'generation'
    }

    // ===== GETTERS =====
    const getWallet = computed(() => wallet.value)
    const getReceiverId = computed(() => wallet.value.receiverId)
    const getSpendingKey = computed(() => wallet.value.spendingKey)
    const getGenerationSpendingKey = computed(() => wallet.value.generationSpendingKey ?? null)
    const getAddress = computed(() => wallet.value.address)
    const getDctidh_address = computed(() => wallet.value.dctidh_address ?? null)
    const getGeneration_address = computed(() => wallet.value.generation_address ?? null)
    const getAddressType = computed(() => addressType.value)
    const getNetwork = computed(() => wallet.value.network)
    const getBalance = computed(() => wallet.value.balance)
    const getPendingBalance = computed(() => wallet.value.pendingBalance)
    const getUtxos = computed(() => wallet.value.utxos)
    const getMinBlockHeight = computed(() => wallet.value.minBlockHeight ?? null)
    const hasWallet = computed(() => wallet.value.address !== null)
    const getKeystoreFileName = computed(() => keystoreFileName.value ?? null)
    const getRpcUrl = computed(() => rpcUrl.value)
    return {
        getWallet,
        getReceiverId,
        getSpendingKey,
        getGenerationSpendingKey,
        getAddress,
        getDctidh_address,
        getGeneration_address,
        getAddressType,
        getNetwork,
        getBalance,
        getPendingBalance,
        getUtxos,
        getMinBlockHeight,
        hasWallet,
        getKeystoreFileName,
        getRpcUrl,
        setReceiverId,
        setSpendingKey,
        setGenerationSpendingKey,
        setAddress,
        setDctidh_address,
        setGeneration_address,
        setAddressType,
        setNetwork,
        setBalance,
        setPendingBalance,
        setUtxos,
        setMinBlockHeight,
        setWallet,
        setKeystoreFileName,
        setRpcUrl,
        clearWallet,
    }
})
