import { useNeptuneStore } from '@/stores/neptuneStore'
import * as API from '@/api/neptuneApi'
import type {
  BalanceResult,
  GenerateSeedResult,
  PayloadBuildTransaction,
  ViewKeyResult,
  WalletState,
} from '@/types/wallet'
import { DEFAULT_MIN_BLOCK_HEIGHT } from '@/utils/constants/constants'
import { toFiniteNumber } from '@/utils'

export function useNeptuneWallet() {
  const store = useNeptuneStore()

  const generateWallet = async (): Promise<GenerateSeedResult> => {
    try {
      const result = await (window as any).walletApi.generateSeed()
      if (!result.success) {
        throw new Error(result.error || 'Failed to generate seed')
      }

      store.setReceiverId(result.receiver_identifier)

      const viewKeyResult = await getViewKeyFromSeed(result.seed_phrase)
      store.setViewKey(viewKeyResult.view_key_hex)
      store.setSpendingKey(viewKeyResult.spending_key_hex)

      const addressResult = (viewKeyResult as any).address || viewKeyResult.receiver_identifier
      store.setAddress(addressResult)

      return result
    } catch (err: any) {
      await (window as any).logger?.error('Error generating wallet: ', err.message)
      throw err
    }
  }

  const getViewKeyFromSeed = async (seedPhrase: string[]): Promise<ViewKeyResult> => {
    const network = store.getNetwork === 'mainnet' ? 'main' : 'testnet'
    const result = await (window as any).walletApi.generateKeysFromSeed([...seedPhrase], network)
    if (!result.success) {
      throw new Error(result.error || 'Failed to get view key from seed')
    }
    return result
  }

  const recoverWalletFromSeed = async (seedPhrase: string[]): Promise<WalletState> => {
    try {
      const result = await getViewKeyFromSeed(seedPhrase)
      store.setReceiverId(result.receiver_identifier)
      store.setViewKey(result.view_key_hex)
      store.setSpendingKey(result.spending_key_hex)

      const addressResult = (result as any).address || result.receiver_identifier
      store.setAddress(addressResult)

      return {
        network: store.getNetwork,
        receiverId: result.receiver_identifier,
        viewKey: result.view_key_hex,
        spendingKey: result.spending_key_hex,
        address: addressResult,
      }
    } catch (err: any) {
      await (window as any).logger?.error('Error recovering wallet from seed: ', err.message)
      throw err
    }
  }

  const getAddressFromSeed = async (seedPhrase: string[]): Promise<string> => {
    const network = store.getNetwork === 'mainnet' ? 'main' : 'testnet'
    const result = await (window as any).walletApi.generateKeysFromSeed(seedPhrase, network)
    return result.address || result.receiver_identifier || ''
  }

  const decryptKeystore = async (password: string): Promise<{ seedPhrase: string[] }> => {
    try {
      const keystoreFileName = store.getKeystoreFileName
      if (!keystoreFileName) await checkKeystore()

      const result = await (window as any).walletApi.decryptKeystore(
        keystoreFileName,
        password
      )

      const seedPhrase = result.phrase.trim().split(/\s+/)
      const viewKeyResult = await getViewKeyFromSeed(seedPhrase)

      store.setViewKey(viewKeyResult.view_key_hex)
      store.setReceiverId(viewKeyResult.receiver_identifier)
      store.setSpendingKey(viewKeyResult.spending_key_hex)

      const addressResult = (viewKeyResult as any).address || viewKeyResult.receiver_identifier
      store.setAddress(addressResult)

      await loadMinBlockHeightFromKeystore()

      return { seedPhrase }
    } catch (err: any) {
      if (
        err instanceof Error &&
        (err.message.includes('Unsupported state') ||
          err.message.includes('unable to authenticate'))
      ) {
        await (window as any).logger?.error('Invalid password')
      } else await (window as any).logger?.error('Error decrypting keystore: ', err.message)

      throw err
    }
  }

  const createKeystore = async (seed: string, password: string): Promise<string | null> => {
    try {
      const result = await (window as any).walletApi.createKeystore(seed, password)
      if (!result.success) return null
      store.setKeystoreFileName(result.fileName)
      store.setMinBlockHeight(DEFAULT_MIN_BLOCK_HEIGHT)
      return result.fileName
    } catch (err: any) {
      await (window as any).logger?.error('Error creating keystore: ', err.message)
      throw err
    }
  }

  const saveKeystoreAs = async (): Promise<void> => {
    try {
      const keystoreFileName = store.getKeystoreFileName
      if (!keystoreFileName) throw new Error('No file to save')

      const result = await (window as any).walletApi.saveKeystoreAs(keystoreFileName)
      if (!result.filePath) throw new Error('User canceled')
    } catch (err: any) {
      await (window as any).logger?.error('Error saving keystore: ', err.message)
      throw err
    }
  }

  const checkKeystore = async (): Promise<boolean> => {
    try {
      const keystoreFile = await (window as any).walletApi.checkKeystore(store.getKeystoreFileName)
      if (!keystoreFile.exists) return false

      store.setKeystoreFileName(keystoreFile.fileName)
      if ('minBlockHeight' in keystoreFile) {
        const height = keystoreFile.minBlockHeight
        store.setMinBlockHeight(toFiniteNumber(height))
      }
      return true
    } catch (err: any) {
      await (window as any).logger?.error('Error checking keystore: ', err.message)
      throw err
    }
  }

  const persistMinBlockHeight = async (utxos: any[]) => {
    const keystoreFileName = store.getKeystoreFileName
    if (!keystoreFileName) return

    try {
      const minBlockHeight = utxos.reduce((min, utxo) => {
        const h = +(
          utxo?.blockHeight ??
          utxo?.block_height ??
          utxo?.height ??
          utxo?.block?.height
        )
        return Number.isFinite(h) && (min === null || h < min) ? h : min
      }, null)

      const response = await (window as any).walletApi.updateMinBlockHeight(
        keystoreFileName,
        minBlockHeight
      )
      if (!response.success) throw new Error('Failed to update min block height')
      store.setMinBlockHeight(minBlockHeight)
    } catch (err: any) {
      await (window as any).logger?.error('Error saving min block height: ', err.message)
      throw err
    }
  }

  const loadMinBlockHeightFromKeystore = async (): Promise<number> => {
    const keystoreFileName = store.getKeystoreFileName
    if (!keystoreFileName) return DEFAULT_MIN_BLOCK_HEIGHT

    try {
      const response = await (window as any).walletApi.getMinBlockHeight(keystoreFileName)
      if (!response?.success) throw new Error(String(response.error))

      const minBlockHeight = toFiniteNumber(response.minBlockHeight)

      store.setMinBlockHeight(minBlockHeight)
      return minBlockHeight
    } catch (err: any) {
      await (window as any).logger?.error('Error loading min block height: ', err.message)
      throw err
    }
  }

  // ===== API METHODS =====
  const getUtxos = async (): Promise<any> => {
    try {
      if (!store.getViewKey) {
        throw new Error('No view key available. Please import or generate a wallet first.')
      }

      let startBlock: number | null = store.getMinBlockHeight
      if (startBlock == null) startBlock = await loadMinBlockHeightFromKeystore()

      const response = await API.getUtxosFromViewKey(
        store.getViewKey || '',
        toFiniteNumber(startBlock)
      )

      const result = response?.result || response
      const utxos = result?.utxos ?? result
      const utxoList = Array.isArray(utxos) ? utxos : []

      store.setUtxos(utxoList)

      await persistMinBlockHeight(utxoList)
      return result
    } catch (err: any) {
      console.error('Error getting UTXOs:', err.message)
      throw err
    }
  }

  const getBalance = async (): Promise<BalanceResult> => {
    try {
      let startBlock: number | null | undefined = store.getMinBlockHeight

      if (startBlock == null) {
        startBlock = await loadMinBlockHeightFromKeystore()
      }

      const response = await API.getBalance(
        store.getViewKey || '',
        toFiniteNumber(startBlock)
      )
      const result = response?.result || response

      store.setBalance(result?.balance || result)
      store.setPendingBalance(result?.pendingBalance || result)
      return {
        balance: result?.balance || result,
        pendingBalance: result?.pendingBalance || result,
      }
    } catch (err: any) {
      console.error('Error getting balance:', err.message)
      throw err
    }
  }

  const getBlockHeight = async (): Promise<any> => {
    try {
      const response = await API.getBlockHeight()
      const result = response?.result || response
      return result?.height || result
    } catch (err: any) {
      console.error('Error getting block height:', err.message)
      throw err
    }
  }

  const getNetworkInfo = async (): Promise<any> => {
    try {
      const response = await API.getNetworkInfo()
      const result = response?.result || response
      store.setNetwork((result.network + 'net') as 'mainnet' | 'testnet')

      return result
    } catch (err: any) {
      console.error('Error getting network info:', err.message)
      throw err
    }
  }

  const buildTransaction = async (args: PayloadBuildTransaction): Promise<any> => {
    let minBlockHeight: number | null | undefined = store.getMinBlockHeight
    if (minBlockHeight === null || minBlockHeight === undefined) {
      minBlockHeight = await loadMinBlockHeightFromKeystore()
    }

    const payload = {
      spendingKeyHex: store.getSpendingKey,
      inputAdditionRecords: args.inputAdditionRecords,
      minBlockHeight: toFiniteNumber(minBlockHeight),
      outputAddresses: args.outputAddresses,
      outputAmounts: args.outputAmounts,
      fee: args.fee,
    }
    try {
      const response = await (window as any).walletApi.buildTransaction(payload)
      if (!response?.success) {
        await (window as any).logger?.error(
          `[SendTx] walletApi.buildTransaction returned error: ${response?.error || 'unknown'}`
        )
      }
      return response
    } catch (err: any) {
      const errorMessage = err?.message || err?.error || 'Unknown build transaction error'
      await (window as any).logger?.error(`[SendTx] buildTransaction invoke failed: ${errorMessage}`)
      return { success: false, error: errorMessage }
    }
  }

  const broadcastSignedTransaction = async (transactionHex: string): Promise<any> => {
    try {
      const response = await API.broadcastSignedTransaction(transactionHex)
      const result = response?.result || response
      return result
    } catch (err: any) {
      console.error('Error sending transaction:', err.message)
      throw err
    }
  }

  // ===== UTILITY METHODS =====
  const clearWallet = () => {
    store.clearWallet()
  }

  return {
    generateWallet,
    recoverWalletFromSeed,
    getViewKeyFromSeed,
    getAddressFromSeed,

    getUtxos,
    getBalance,
    getBlockHeight,
    getNetworkInfo,
    buildTransaction,
    broadcastSignedTransaction,
    decryptKeystore,
    createKeystore,
    saveKeystoreAs,
    checkKeystore,

    clearWallet,
  }
}
