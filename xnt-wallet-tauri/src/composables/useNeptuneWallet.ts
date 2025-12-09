import { useNeptuneStore } from '@/stores/neptuneStore'
import * as API from '@/api/neptuneApi'
import type {
  BalanceResult,
  GenerateSeedResult,
  PayloadBuildTransaction,
  ViewKeyResult,
  WalletState,
} from '@/types'
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

      // Load min block height from keystore (but don't overwrite if it's 0)
      const loadedHeight = await loadMinBlockHeightFromKeystore()
      if (loadedHeight > 0) {
        store.setMinBlockHeight(loadedHeight)
      }

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
    if (!keystoreFileName) return 0 // Return 0 to indicate not set

    try {
      const response = await (window as any).walletApi.getMinBlockHeight(keystoreFileName)
      if (!response?.success) {
        // If keystore doesn't have minBlockHeight set, return 0 (not set yet)
        return 0
      }

      const minBlockHeight = toFiniteNumber(response.minBlockHeight || 0)
      
      // Return the value (0 means not set yet, >0 means it's set)
      return minBlockHeight
    } catch (err: any) {
      await (window as any).logger?.warn('Error loading min block height (will start from 0): ', err.message)
      return 0 // Return 0 on error, meaning not set yet
    }
  }

  // ===== API METHODS =====
  const getUtxos = async (): Promise<any> => {
    try {
      if (!store.getViewKey) {
        throw new Error('No view key available. Please import or generate a wallet first.')
      }

      // Get current block height
      const blockHeightResponse = await API.getBlockHeight()
      const currentHeight = blockHeightResponse?.result?.height || blockHeightResponse?.height || 0

      if (currentHeight === 0) {
        throw new Error('Failed to get block height')
      }

      // Calculate start block from existing UTXOs (if any)
      const existingUtxos = store.getUtxos || []
      let minBlockFromUtxos: number | null = null
      if (existingUtxos.length > 0) {
        minBlockFromUtxos = existingUtxos.reduce((min: number | null, utxo: any) => {
          const h = +(
            utxo?.blockHeight ??
            utxo?.block_height ??
            utxo?.height ??
            utxo?.block?.height
          )
          return Number.isFinite(h) && (min === null || h < min) ? h : min
        }, null)
      }

      // Get last scanned block (from store or keystore)
      let lastScannedBlock: number | null = store.getMinBlockHeight
      // If store doesn't have it, or it's 0 (meaning not set), try loading from keystore
      if (lastScannedBlock == null || lastScannedBlock === 0) {
        const loaded = await loadMinBlockHeightFromKeystore()
        // Only use loaded value if it's greater than 0 (0 means not set yet)
        if (loaded > 0) {
          lastScannedBlock = loaded
          store.setMinBlockHeight(loaded) // Update store with loaded value
        } else {
          lastScannedBlock = null
        }
      }

      // Start from the last scanned block (where we stopped scanning)
      // This is more efficient than rescanning from the oldest UTXO block
      let startBlock: number
      if (lastScannedBlock !== null && lastScannedBlock > 0) {
        // Resume from last scanned block (most efficient)
        startBlock = lastScannedBlock
      } else if (minBlockFromUtxos !== null) {
        // No last scanned block, but we have UTXOs - start from oldest UTXO
        startBlock = minBlockFromUtxos
      } else {
        // First scan - start from beginning
        startBlock = DEFAULT_MIN_BLOCK_HEIGHT
      }
      

      // Get RPC URL from environment or use default
      const rpcUrl = import.meta.env.VITE_APP_API || 'https://66b4b9bd1e4c.ngrok-free.app'

      // Use client-side scanning (NO VIEW KEY TO SERVER!)
      const scanRequest = {
        viewKeyHex: store.getViewKey,
        startBlock: toFiniteNumber(startBlock),
        endBlock: currentHeight,
        rpcUrl: rpcUrl,
      }

      const scanResponse = await (window as any).walletApi.scanUtxosClientSide(scanRequest)
      const scanResult = typeof scanResponse === 'string' ? JSON.parse(scanResponse) : scanResponse

      if (!scanResult.success) {
        throw new Error(scanResult.error || 'Scan failed')
      }

      // Merge new UTXOs with existing ones (deduplicate by utxo_hash)
      const existingUtxoHashes = new Set(
        existingUtxos.map((u: any) => u.utxoHash || u.utxo_hash || u.hash)
      )
      const newUtxos = (scanResult.utxos || []).filter(
        (u: any) => !existingUtxoHashes.has(u.utxoHash || u.utxo_hash || u.hash)
      )
      const allUtxos = [...existingUtxos, ...newUtxos]

      // Store all UTXOs
      store.setUtxos(allUtxos)

      // Update last scanned block height - save the last scanned block, not the minimum UTXO block
      // This allows us to resume scanning from where we left off
      if (scanResult.lastScannedBlock !== null && scanResult.lastScannedBlock !== undefined) {
        const lastScanned = toFiniteNumber(scanResult.lastScannedBlock)
        store.setMinBlockHeight(lastScanned)
        
        // Persist the last scanned block to keystore (not the minimum UTXO block)
        const keystoreFileName = store.getKeystoreFileName
        if (keystoreFileName) {
          try {
            const response = await (window as any).walletApi.updateMinBlockHeight(
              keystoreFileName,
              lastScanned
            )
            if (!response.success) {
              await (window as any).logger?.warn('Failed to save last scanned block height')
            }
          } catch (err: any) {
            await (window as any).logger?.warn('Error saving last scanned block height: ', err.message)
          }
        }
      }

      return {
        result: {
          utxos: allUtxos,
        },
      }
    } catch (err: any) {
      console.error('Error getting UTXOs:', err.message)
      throw err
    }
  }

  const getBalance = async (): Promise<BalanceResult> => {
    try {
      // Calculate balance from unspent UTXOs (client-side, no view_key sent to server)
      const utxos = store.getUtxos || []
      const balance = utxos.reduce((sum: number, utxo: any) => {
        const amount = parseFloat(utxo.amount || '0')
        return sum + (isNaN(amount) ? 0 : amount)
      }, 0)

      const balanceStr = balance.toFixed(8)
      store.setBalance(balanceStr)

      // Get pending balance from mempool (client-side scanning)
      let pendingBalance = '0'
      if (store.getViewKey) {
        try {
          const rpcUrl = import.meta.env.VITE_APP_API || 'https://66b4b9bd1e4c.ngrok-free.app'
          const pendingResult = await (window as any).walletApi.getPendingBalance({
            viewKeyHex: store.getViewKey,
            rpcUrl: rpcUrl,
          })
          const pendingData = typeof pendingResult === 'string' ? JSON.parse(pendingResult) : pendingResult
          if (pendingData.success) {
            pendingBalance = pendingData.pendingBalance || '0'
          }
        } catch (pendingErr: any) {
          console.warn('Error getting pending balance:', pendingErr.message)
          // Continue with pending balance as 0 if mempool check fails
        }
      }

      store.setPendingBalance(pendingBalance)
      return {
        balance: balanceStr,
        pendingBalance: pendingBalance,
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
