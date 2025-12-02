import { useNeptuneStore } from '@/stores/neptuneStore'
import * as API from '@/api/neptuneApi'
import type {
  BalanceResult,
  GenerateSeedResult,
  PayloadBuildTransaction,
  ViewKeyResult,
  WalletState,
} from '@/types/wallet'
import initWasm, { generate_seed, address_from_seed, validate_seed_phrase } from '@neptune/wasm'
import { DEFAULT_MIN_BLOCK_HEIGHT } from '@/utils/constants'

let wasmInitialized = false
let initPromise: Promise<void> | null = null

export function useNeptuneWallet() {
  const store = useNeptuneStore()

  // ===== WASM METHODS =====
  const ensureWasmInitialized = async (): Promise<void> => {
    if (wasmInitialized) {
      return
    }

    if (initPromise) {
      return initPromise
    }

    initPromise = (async () => {
      try {
        await initWasm()
        wasmInitialized = true
        console.log('✅ WASM initialized successfully')
      } catch (err: any) {
        wasmInitialized = false
        console.error('❌ WASM init error:', err)
        throw new Error('Failed to initialize Neptune WASM')
      }
    })()

    return initPromise
  }

  const generateWallet = async (): Promise<GenerateSeedResult> => {
    try {
      await ensureWasmInitialized()

      const resultJson = generate_seed()
      const result: GenerateSeedResult = JSON.parse(resultJson)

      store.setReceiverId(result.receiver_identifier)

      // Get view key and spending key
      const viewKeyResult = await getViewKeyFromSeed(result.seed_phrase)
      store.setViewKey(viewKeyResult.view_key_hex)
      store.setSpendingKey(viewKeyResult.spending_key_hex)

      const addressResult = await getAddressFromSeed(result.seed_phrase)
      store.setAddress(addressResult)

      return result
    } catch (err: any) {
      console.error('Error generating wallet:', err.message)
      throw err
    }
  }

  const getViewKeyFromSeed = async (_seedPhrase: string[]): Promise<ViewKeyResult> => {
    // TODO: Implement Tauri command
    // return await invoke('generate_keys_from_seed', { seedPhrase: _seedPhrase })
    
    // Mock data for testing
    return {
      receiver_identifier: 'mock_receiver_id_' + Date.now(),
      view_key_hex: '0x' + Array(64).fill(0).map(() => Math.floor(Math.random() * 16).toString(16)).join(''),
      spending_key_hex: '0x' + Array(64).fill(0).map(() => Math.floor(Math.random() * 16).toString(16)).join(''),
    }
  }

  const recoverWalletFromSeed = async (seedPhrase: string[]): Promise<WalletState> => {
    try {
      await ensureWasmInitialized()
      const isValid = validate_seed_phrase(JSON.stringify(seedPhrase))
      if (!isValid) throw new Error('Invalid seed phrase')

      // TODO: Implement Tauri command
      // const result = await getViewKeyFromSeed(seedPhrase)
      // store.setReceiverId(result.receiver_identifier)
      // store.setViewKey(result.view_key_hex)
      // store.setSpendingKey(result.spending_key_hex)

      const addressResult = await getAddressFromSeed(seedPhrase)
      store.setAddress(addressResult)

      return {
        network: store.getNetwork,
        receiverId: store.getReceiverId || '',
        viewKey: store.getViewKey || '',
        spendingKey: store.getSpendingKey || '',
        address: addressResult,
      }
    } catch (err: any) {
      console.error('Error recovering wallet from seed:', err.message)
      throw err
    }
  }

  const getAddressFromSeed = async (seedPhrase: string[]): Promise<string> => {
    await ensureWasmInitialized()
    const seedPhraseJson = JSON.stringify(seedPhrase)
    return address_from_seed(seedPhraseJson, store.getNetwork)
  }

  const decryptKeystore = async (_password: string): Promise<{ seedPhrase: string[] }> => {
    try {
      const keystoreFileName = store.getKeystoreFileName
      if (!keystoreFileName) await checkKeystore()

      // TODO: Implement Tauri command
      // const result = await invoke('decrypt_keystore', { fileName: keystoreFileName, password: _password })
      
      // Mock: return stored seed phrase
      throw new Error('Password verification not implemented yet')
    } catch (err: any) {
      if (
        err instanceof Error &&
        (err.message.includes('Unsupported state') || err.message.includes('unable to authenticate'))
      ) {
        console.error('Invalid password')
      } else console.error('Error decrypting keystore:', err.message)

      throw err
    }
  }

  const createKeystore = async (_seed: string, _password: string): Promise<string | null> => {
    try {
      // TODO: Implement Tauri command
      // const result = await invoke('create_keystore', { seed: _seed, password: _password })
      // if (!result.success) return null
      // store.setKeystoreFileName(result.fileName)
      // store.setMinBlockHeight(DEFAULT_MIN_BLOCK_HEIGHT)
      // return result.fileName

      console.log('Creating keystore with seed and password...')
      // Mock success for now
      const mockFileName = `neptune-wallet-${Date.now()}.json`
      store.setKeystoreFileName(mockFileName)
      store.setMinBlockHeight(DEFAULT_MIN_BLOCK_HEIGHT)
      return mockFileName
    } catch (err: any) {
      console.error('Error creating keystore:', err.message)
      throw err
    }
  }

  const saveKeystoreAs = async (): Promise<void> => {
    try {
      const keystoreFileName = store.getKeystoreFileName
      if (!keystoreFileName) throw new Error('No file to save')

      // TODO: Implement Tauri command
      // const result = await invoke('save_keystore_as', { fileName: keystoreFileName })
      // if (!result.filePath) throw new Error('User canceled')
      throw new Error('Tauri command not implemented yet: save_keystore_as')
    } catch (err: any) {
      console.error('Error saving keystore:', err.message)
      throw err
    }
  }

  const checkKeystore = async (): Promise<boolean> => {
    try {
      // TODO: Implement Tauri command
      // const keystoreFile = await invoke('check_keystore', { fileName: store.getKeystoreFileName })
      // if (!keystoreFile.exists) return false
      // store.setKeystoreFileName(keystoreFile.fileName)
      // if ('minBlockHeight' in keystoreFile) {
      //   store.setMinBlockHeight(keystoreFile.minBlockHeight)
      // }
      // return true
      
      // Mock: return true if keystoreFileName exists
      return !!store.getKeystoreFileName
    } catch (err: any) {
      console.error('Error checking keystore:', err.message)
      throw err
    }
  }

  const persistMinBlockHeight = async (utxos: any[]) => {
    const keystoreFileName = store.getKeystoreFileName
    if (!keystoreFileName) return

    try {
      const minBlockHeight = utxos.reduce((min, utxo) => {
        const h = +(utxo?.blockHeight ?? utxo?.block_height ?? utxo?.height ?? utxo?.block?.height)
        return Number.isFinite(h) && (min === null || h < min) ? h : min
      }, null)

      // TODO: Implement Tauri command
      // const response = await invoke('update_min_block_height', {
      //   fileName: keystoreFileName,
      //   minBlockHeight
      // })
      // if (!response.success) throw new Error('Failed to update min block height')
      store.setMinBlockHeight(minBlockHeight)
    } catch (err: any) {
      console.error('Error saving min block height:', err.message)
      throw err
    }
  }

  const loadMinBlockHeightFromKeystore = async (): Promise<number> => {
    const keystoreFileName = store.getKeystoreFileName
    if (!keystoreFileName) return DEFAULT_MIN_BLOCK_HEIGHT

    try {
      // TODO: Implement Tauri command
      // const response = await invoke('get_min_block_height', { fileName: keystoreFileName })
      // if (!response?.success) throw new Error(String(response.error))
      //
      // const minBlockHeight = response.minBlockHeight
      // store.setMinBlockHeight(minBlockHeight)
      // return minBlockHeight
      return DEFAULT_MIN_BLOCK_HEIGHT
    } catch (err: any) {
      console.error('Error loading min block height:', err.message)
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

      const response = await API.getUtxosFromViewKey(store.getViewKey || '', startBlock || 0)

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

      const response = await API.getBalance(store.getViewKey || '', startBlock || 0)
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

    // TODO: Implement Tauri command
    // const payload = {
    //   spendingKeyHex: store.getSpendingKey,
    //   inputAdditionRecords: args.inputAdditionRecords,
    //   minBlockHeight: minBlockHeight || 0,
    //   outputAddresses: args.outputAddresses,
    //   outputAmounts: args.outputAmounts,
    //   fee: args.fee,
    // }
    // return await invoke('build_transaction', payload)

    console.log('Build transaction called with:', args)
    throw new Error('Tauri command not implemented yet: build_transaction')
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
    initWasm: ensureWasmInitialized,
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
