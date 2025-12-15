import { useNeptuneStore } from '@/stores/neptuneStore'
import * as API from '@/api/neptuneApi'
import type {
  BalanceResult,
  GenerateSeedResult,
  PayloadBuildTransaction,
  ViewKeyResult,
  WalletState,
} from '@/types'
import { DEFAULT_MIN_BLOCK_HEIGHT } from '@/utils'
import { toFiniteNumber } from '@/utils'

let cachedViewKey: string | null = null


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
      cachedViewKey = viewKeyResult.view_key_hex
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
      store.clearWallet()
      cachedViewKey = null
      await (window as any).walletApi.clearUtxoCache?.()

      const result = await getViewKeyFromSeed(seedPhrase)
      store.setReceiverId(result.receiver_identifier)
      store.setSpendingKey(result.spending_key_hex)
      cachedViewKey = result.view_key_hex

      const addressResult = (result as any).address || result.receiver_identifier
      store.setAddress(addressResult)

      return {
        network: store.getNetwork,
        receiverId: result.receiver_identifier,
        spendingKey: result.spending_key_hex,
        address: addressResult,
      }
    } catch (err: any) {
      await (window as any).logger?.error('Error recovering wallet from seed:', err.message)
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
      let keystoreFileName = store.getKeystoreFileName
      if (!keystoreFileName) {
        await checkKeystore()
        keystoreFileName = store.getKeystoreFileName
      }

      if (!keystoreFileName) {
        throw new Error('No keystore file found')
      }

      store.clearWallet()
      cachedViewKey = null
      await (window as any).walletApi.clearUtxoCache?.()
      store.setKeystoreFileName(keystoreFileName)

      const result = await (window as any).walletApi.decryptKeystore(
        keystoreFileName,
        password
      )

      const seedPhrase = result.phrase.trim().split(/\s+/)
      const viewKeyResult = await getViewKeyFromSeed(seedPhrase)

      store.setReceiverId(viewKeyResult.receiver_identifier)
      store.setSpendingKey(viewKeyResult.spending_key_hex)
      cachedViewKey = viewKeyResult.view_key_hex

      const addressResult = (viewKeyResult as any).address || viewKeyResult.receiver_identifier
      store.setAddress(addressResult)

      try {
        const minBlockHeightResponse = await (window as any).walletApi.getMinBlockHeight(keystoreFileName)
        const minBlockHeightData = typeof minBlockHeightResponse === 'string' 
          ? JSON.parse(minBlockHeightResponse) 
          : minBlockHeightResponse
        
        if (minBlockHeightData?.success && minBlockHeightData?.minBlockHeight !== undefined) {
          const savedHeight = toFiniteNumber(minBlockHeightData.minBlockHeight)
          store.setMinBlockHeight(savedHeight)
        } else {
          store.setMinBlockHeight(0)
        }
      } catch (err: any) {
        await (window as any).logger?.warn('Error loading min block height from keystore, defaulting to 0:', err.message)
        store.setMinBlockHeight(0)
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

  const createKeystore = async (seed: string, password: string, isImportedWallet: boolean = false): Promise<string | null> => {
    try {
      const savedAddress = store.getAddress
      const savedReceiverId = store.getReceiverId
      const savedSpendingKey = store.getSpendingKey
      const savedViewKey = cachedViewKey
      
      store.clearWallet()
      cachedViewKey = null
      store.setKeystoreFileName(null)
      await (window as any).walletApi.clearUtxoCache?.()

      let minBlockHeight = isImportedWallet ? 0 : DEFAULT_MIN_BLOCK_HEIGHT
      
      if (!isImportedWallet) {
        try {
          const bh = await API.getBlockHeight()
          minBlockHeight = toFiniteNumber(bh?.result?.height || bh?.height || DEFAULT_MIN_BLOCK_HEIGHT)
        } catch (e: any) {
          await (window as any).logger?.warn('Failed to fetch block height:', e?.message || e)
        }
      }

      const resultRaw = await (window as any).walletApi.createKeystore(seed, password)
      const result = typeof resultRaw === 'string' ? JSON.parse(resultRaw) : resultRaw
      
      if (!result?.success || !result.fileName) {
        await (window as any).logger?.error('Keystore creation failed:', result?.error || 'Unknown error')
        return null
      }
      
      store.setKeystoreFileName(result.fileName)
      const finalMinBlockHeight = toFiniteNumber(minBlockHeight)
      store.setMinBlockHeight(finalMinBlockHeight)
      await (window as any).walletApi.updateMinBlockHeight(result.fileName, finalMinBlockHeight)
      
      if (savedAddress) store.setAddress(savedAddress)
      if (savedReceiverId) store.setReceiverId(savedReceiverId)
      if (savedSpendingKey) store.setSpendingKey(savedSpendingKey)
      if (savedViewKey) cachedViewKey = savedViewKey
      
      return result.fileName
    } catch (err: any) {
      await (window as any).logger?.error('Error creating keystore:', err.message)
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
    if (!keystoreFileName) return 0

    try {
      const response = await (window as any).walletApi.getMinBlockHeight(keystoreFileName)
      if (!response?.success) {
        return 0
      }

      const minBlockHeight = toFiniteNumber(response.minBlockHeight || 0)
      
      return minBlockHeight
    } catch (err: any) {
      await (window as any).logger?.warn('Error loading min block height (will start from 0): ', err.message)
      return 0
    }
  }

  // ===== API METHODS =====
  const forceRescanUtxos = async (): Promise<any> => {
    const keystoreFileName = store.getKeystoreFileName
    if (keystoreFileName) {
      store.setMinBlockHeight(0)
      await (window as any).walletApi.updateMinBlockHeight(keystoreFileName, 0)
    } else {
      store.setMinBlockHeight(0)
    }
    
    return await getUtxos()
  }

  const getUtxos = async (): Promise<any> => {
    if (!cachedViewKey) {
      throw new Error('No view key available. Please import or generate a wallet first.')
    }

    try {
      if (!store.getKeystoreFileName) {
        await checkKeystore()
      }

      const blockHeightResponse = await API.getBlockHeight()
      const currentHeight = blockHeightResponse?.result?.height || 0

      if (currentHeight === 0) {
        throw new Error('Failed to get block height')
      }

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

      let lastScannedBlock: number | null = store.getMinBlockHeight
      let startBlock: number
      
      // If store has 0, it's an imported wallet - force full scan from block 0
      if (lastScannedBlock === 0) {
        startBlock = DEFAULT_MIN_BLOCK_HEIGHT
      } else if (lastScannedBlock == null) {
        const loaded = await loadMinBlockHeightFromKeystore()
        if (loaded > 0) {
          lastScannedBlock = loaded
          store.setMinBlockHeight(loaded)
          startBlock = Math.max(DEFAULT_MIN_BLOCK_HEIGHT, lastScannedBlock - 10)
        } else {
          lastScannedBlock = 0
          store.setMinBlockHeight(0)
          startBlock = DEFAULT_MIN_BLOCK_HEIGHT
        }
      } else if (lastScannedBlock > 0) {
        startBlock = Math.max(DEFAULT_MIN_BLOCK_HEIGHT, lastScannedBlock - 10)
      } else if (minBlockFromUtxos !== null) {
        startBlock = minBlockFromUtxos
      } else {
        startBlock = DEFAULT_MIN_BLOCK_HEIGHT
      }

      const customRpc = await (window as any).walletApi.getCustomRpcEndpoint?.()
      const rpcUrl = customRpc || import.meta.env.VITE_APP_API
      if (!rpcUrl) {
        throw new Error('RPC endpoint is not configured')
      }

      const scanRequest = {
        viewKeyHex: cachedViewKey,
        startBlock: toFiniteNumber(startBlock),
        endBlock: currentHeight,
        rpcUrl: rpcUrl,
      }

      const scanResponse = await (window as any).walletApi.scanUtxosClientSide(scanRequest)
      const scanResult = typeof scanResponse === 'string' ? JSON.parse(scanResponse) : scanResponse

      if (!scanResult.success) {
        throw new Error(scanResult.error || 'Scan failed')
      }

      const normalizeUtxo = (u: any) => ({
        utxoHash: u.utxo_hash,
        amount: u.amount?.toString?.() ?? '0',
        blockHeight: toFiniteNumber(u.block_height),
        additionRecord: u.addition_record_hex,
      })

      const freshUtxos = (scanResult.utxos || []).map(normalizeUtxo)
      store.setUtxos(freshUtxos)

      const lastScannedFromResponse = scanResult.last_scanned_block ?? null

      const minHeightFromUtxos = freshUtxos.length
        ? freshUtxos.reduce((min: number, u: any) => {
            const h = toFiniteNumber(u.blockHeight)
            if (!Number.isFinite(h) || h <= 0) return min
            return min === 0 || h < min ? h : min
          }, 0)
        : null

      if (lastScannedFromResponse !== null && lastScannedFromResponse !== undefined) {
        const lastScanned = toFiniteNumber(lastScannedFromResponse)
        const heightToPersist = minHeightFromUtxos && minHeightFromUtxos > 0 ? minHeightFromUtxos : lastScanned

        store.setMinBlockHeight(heightToPersist)

        const keystoreFileName = store.getKeystoreFileName
        if (keystoreFileName) {
          await (window as any).walletApi.updateMinBlockHeight(keystoreFileName, heightToPersist)
        }
      }

      return {
        result: {
          utxos: freshUtxos,
        },
      }
    } catch (err: any) {
      await (window as any).logger?.error('Error getting UTXOs:', err.message)
      store.setUtxos([])
      throw err
    }
  }

  const getBalance = async (): Promise<BalanceResult> => {
    try {
      const utxos = store.getUtxos || []
      const balance = utxos.reduce((sum: number, utxo: any) => {
        const amount = parseFloat(utxo.amount || '0')
        return sum + (isNaN(amount) ? 0 : amount)
      }, 0)

      const balanceStr = balance.toFixed(8)
      store.setBalance(balanceStr)

      let pendingBalance = '0'
      if (cachedViewKey) {
        const customRpc = await (window as any).walletApi.getCustomRpcEndpoint?.()
        const rpcUrl = customRpc || import.meta.env.VITE_APP_API
        if (rpcUrl) {
          const pendingResult = await (window as any).walletApi.getPendingBalance({
            viewKeyHex: cachedViewKey,
            rpcUrl: rpcUrl,
          })
          const pendingData = typeof pendingResult === 'string' ? JSON.parse(pendingResult) : pendingResult
          if (pendingData.success) {
            pendingBalance = pendingData.pending_balance || '0'
          }
        }
      }

      store.setPendingBalance(pendingBalance)
      return {
        balance: balanceStr,
        pendingBalance: pendingBalance,
      }
    } catch (err: any) {
      await (window as any).logger?.error('Error getting balance:', err.message)
      store.setBalance('0')
      store.setPendingBalance('0')
      throw err
    }
  }

  const getBlockHeight = async (): Promise<any> => {
    try {
      const response = await API.getBlockHeight()
      const result = response?.result || response
      return result?.height || result
    } catch (err: any) {
      await (window as any).logger?.error('Error getting block height:', err.message)
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
      await (window as any).logger?.error('Error getting network info:', err.message)
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
      await (window as any).logger?.error('Error sending transaction:', err.message)
      throw err
    }
  }

  // ===== UTILITY METHODS =====
  const selectUtxos = (
    utxos: any[],
    neededAmount: number,
    fee: number
  ): { selectedUtxos: any[]; totalAmount: number } => {
    const totalNeeded = neededAmount + fee

    const sortedUtxos = [...utxos].sort((a, b) => {
      const amountA = parseFloat(a.amount || '0')
      const amountB = parseFloat(b.amount || '0')
      return amountB - amountA 
    })

    let totalSelected = 0
    const selectedUtxos: any[] = []

    for (const utxo of sortedUtxos) {
      const amount = parseFloat(utxo.amount || '0')
      if (isNaN(amount) || amount <= 0) continue

      selectedUtxos.push(utxo)
      totalSelected += amount

      if (totalSelected >= totalNeeded) {
        break
      }
    }

    return {
      selectedUtxos,
      totalAmount: totalSelected,
    }
  }

  const clearWallet = () => {
    store.clearWallet()
    cachedViewKey = null
  }

  return {
    generateWallet,
    recoverWalletFromSeed,
    getViewKeyFromSeed,
    getAddressFromSeed,

    getUtxos,
    forceRescanUtxos,
    getBalance,
    getBlockHeight,
    getNetworkInfo,
    buildTransaction,
    broadcastSignedTransaction,
    selectUtxos,
    decryptKeystore,
    createKeystore,
    saveKeystoreAs,
    checkKeystore,

    clearWallet,
  }
}
