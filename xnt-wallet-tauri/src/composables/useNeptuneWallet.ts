import { toast } from 'vue-sonner'
import { useAccountsStore, useNeptuneStore, useSettingsStore } from '@/stores'
import * as API from '@/api/neptuneApi'
import { walletApi } from '@/api/tauri/walletApi'
import { currentEpoch, ensureOpen, useAccounts } from './useAccounts'
import type {
  BalanceResult,
  GenerateSeedResult,
  PayloadBuildTransaction,
  SpendingKeyResult,
  WalletState,
} from '@/types'
import {
  CHAIN_RESET_MESSAGE,
  DEFAULT_MIN_BLOCK_HEIGHT,
  normalizeUtxo,
  selectSpendableUtxos,
  toFiniteNumber,
} from '@/utils'

export function useNeptuneWallet() {
  const store = useNeptuneStore()
  const settingsStore = useSettingsStore()
  const accountsStore = useAccountsStore()
  const { unlockAll, lock } = useAccounts()

  const generateWallet = async (): Promise<GenerateSeedResult> => {
    try {
      const result = await (window as any).walletApi.generateSeed()
      if (!result.success) {
        throw new Error(result.error || 'Failed to generate seed')
      }

      store.setReceiverId(result.receiver_identifier)

      const spendingKeyResult = await loadBothAddresses(result.seed_phrase)
      store.setSpendingKey(spendingKeyResult.spending_key_hex)
      store.setReceiverId(spendingKeyResult.receiver_identifier)

      const addressResult =
        (spendingKeyResult as any).address || spendingKeyResult.receiver_identifier
      store.setAddress(addressResult)

      return result
    } catch (err: any) {
      await (window as any).logger?.error('Error generating wallet: ', err.message)
      throw err
    }
  }

  const getSpendingKeyFromSeed = async (
    seedPhrase: string[],
    keyType?: string,
  ): Promise<SpendingKeyResult> => {
    const network = store.getNetwork === 'mainnet' ? 'main' : 'testnet'
    const result = await (window as any).walletApi.generateKeysFromSeed(
      [...seedPhrase],
      network,
      keyType || 'generation',
    )
    if (!result.success) {
      throw new Error(result.error || 'Failed to get spending key from seed')
    }
    return result
  }

  // Derives both Generation (primary) and dCTIDH (display) addresses from the seed.
  // Stores Generation key as the active spending key and both addresses for UI display.
  const loadBothAddresses = async (seedPhrase: string[]): Promise<SpendingKeyResult> => {
    const gen = await getSpendingKeyFromSeed(seedPhrase, 'generation')
    store.setGeneration_address((gen as any).address || null)

    try {
      const dctidh = await getSpendingKeyFromSeed(seedPhrase, 'dctidh')
      store.setDctidh_address((dctidh as any).address || null)
      store.setGenerationSpendingKey((dctidh as any).spending_key_hex || null)
    } catch {
      // dCTIDH address is display-only; ignore errors
    }

    return gen
  }

  const recoverWalletFromSeed = async (seedPhrase: string[]): Promise<WalletState> => {
    try {
      store.clearWallet()
      await (window as any).walletApi.clearUtxoCache?.()

      const result = await loadBothAddresses(seedPhrase)
      store.setReceiverId(result.receiver_identifier)
      store.setSpendingKey(result.spending_key_hex)

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

  const decryptKeystore = async (password: string): Promise<{ historyError?: string }> => {
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
      store.setKeystoreFileName(keystoreFileName)

      const { historyError } = await unlockAll(password)
      // The registry names the main keystore; it may differ from the newest file.
      keystoreFileName = store.getKeystoreFileName ?? keystoreFileName

      try {
        const cachedResult = await walletApi.getCachedUtxosLocal(accountsStore.keyRefs)
        if (cachedResult?.success && Array.isArray(cachedResult.utxos)) {
          accountsStore.setAccountUtxos(
            accountsStore.keyRefs.map((r) => r.accountId),
            cachedResult.utxos.map(normalizeUtxo),
          )
          store.setUtxos(accountsStore.utxosOf(accountsStore.requireActiveId()))
          if (Number.isFinite(cachedResult.last_scanned_block)) {
            const h = toFiniteNumber(cachedResult.last_scanned_block)
            if (h > 0) store.setMinBlockHeight(h)
          }
        }
      } catch (e) {
        await (window as any).logger?.warn(
          'Load cached UTXOs at unlock failed (will scan later):',
          (e as any)?.message || e,
        )
      }

      try {
        const minBlockHeightResponse = await (window as any).walletApi.getMinBlockHeight(
          keystoreFileName,
        )
        const minBlockHeightData =
          typeof minBlockHeightResponse === 'string'
            ? JSON.parse(minBlockHeightResponse)
            : minBlockHeightResponse

        if (minBlockHeightData?.success && minBlockHeightData?.minBlockHeight !== undefined) {
          const savedHeight = toFiniteNumber(minBlockHeightData.minBlockHeight)
          store.setMinBlockHeight(savedHeight)
        } else {
          store.setMinBlockHeight(0)
        }
      } catch (err: any) {
        await (window as any).logger?.warn(
          'Error loading min block height from keystore, defaulting to 0:',
          err.message,
        )
        store.setMinBlockHeight(0)
      }

      return historyError === undefined ? {} : { historyError }
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

  const createKeystore = async (
    seed: string,
    password: string,
    isImportedWallet: boolean = false,
  ): Promise<string | null> => {
    try {
      store.clearWallet()
      store.setKeystoreFileName(null)
      await (window as any).walletApi.clearUtxoCache?.()

      let minBlockHeight = isImportedWallet ? 0 : DEFAULT_MIN_BLOCK_HEIGHT

      if (!isImportedWallet) {
        try {
          const bh = await API.getBlockHeight()
          minBlockHeight = toFiniteNumber(
            bh?.result?.height || bh?.height || DEFAULT_MIN_BLOCK_HEIGHT,
          )
        } catch (e: any) {
          await (window as any).logger?.warn('Failed to fetch block height:', e?.message || e)
        }
      }

      const resultRaw = await (window as any).walletApi.createKeystore(seed, password)
      const result = typeof resultRaw === 'string' ? JSON.parse(resultRaw) : resultRaw

      if (!result?.success || !result.fileName) {
        await (window as any).logger?.error(
          'Keystore creation failed:',
          result?.error || 'Unknown error',
        )
        return null
      }

      store.setKeystoreFileName(result.fileName)
      const finalMinBlockHeight = toFiniteNumber(minBlockHeight)
      store.setMinBlockHeight(finalMinBlockHeight)
      await (window as any).walletApi.updateMinBlockHeight(result.fileName, finalMinBlockHeight)

      await walletApi.accountsResetMain(result.fileName)
      await unlockAll(password)

      return result.fileName
    } catch (err: any) {
      await (window as any).logger?.error('Error creating keystore:', err.message)
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
        const h = +(utxo?.blockHeight ?? utxo?.block_height ?? utxo?.height ?? utxo?.block?.height)
        return Number.isFinite(h) && (min === null || h < min) ? h : min
      }, null)

      const response = await (window as any).walletApi.updateMinBlockHeight(
        keystoreFileName,
        minBlockHeight,
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
      await (window as any).logger?.warn(
        'Error loading min block height (will start from 0): ',
        err.message,
      )
      return 0
    }
  }

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
    const spendingKey = store.getSpendingKey
    if (!spendingKey) {
      const keystoreFileName = store.getKeystoreFileName
      if (keystoreFileName && store.getAddress) {
        throw new Error('No spending key available. Please unlock your wallet first.')
      }
      throw new Error('No spending key available. Please import or generate a wallet first.')
    }

    try {
      if (!store.getKeystoreFileName) {
        await checkKeystore()
      }

      // A lock meanwhile (and a new wallet after it) must not receive this scan's results.
      const mine = currentEpoch()
      const accountRefs = accountsStore.keyRefs
      const accountIds = accountRefs.map((r) => r.accountId)
      const showActive = () =>
        store.setUtxos(accountsStore.utxosOf(accountsStore.requireActiveId()))

      //Load cached UTXOs from disk
      try {
        const cachedResult = await walletApi.getCachedUtxosLocal(accountRefs)
        ensureOpen(mine)
        if (cachedResult?.success && Array.isArray(cachedResult.utxos)) {
          accountsStore.setAccountUtxos(accountIds, cachedResult.utxos.map(normalizeUtxo))
          showActive()
          if (Number.isFinite(cachedResult.last_scanned_block)) {
            const h = toFiniteNumber(cachedResult.last_scanned_block)
            if (h > 0) store.setMinBlockHeight(h)
          }
        }
      } catch (e) {
        await (window as any).logger?.warn(
          'Load cached UTXOs failed (continue to scan):',
          (e as any)?.message || e,
        )
      }

      if (!spendingKey && store.getAddress) {
        throw new Error('Wallet state not fully initialized. Please unlock your wallet.')
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

      const scanRequest = {
        accounts: accountRefs,
        startBlock: toFiniteNumber(startBlock),
        endBlock: currentHeight,
      }

      const scanResponse = await (window as any).walletApi.scanUtxosClientSide(scanRequest)
      ensureOpen(mine)
      const scanResult = typeof scanResponse === 'string' ? JSON.parse(scanResponse) : scanResponse

      if (!scanResult.success) {
        throw new Error(scanResult.error || 'Scan failed')
      }

      if (scanResult.chain_reset === true) toast.info(CHAIN_RESET_MESSAGE)

      accountsStore.setAccountUtxos(accountIds, (scanResult.utxos || []).map(normalizeUtxo))
      showActive()
      refreshAvailable()
      const freshUtxos = store.getUtxos || []

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
        // Persist the last scanned block height so future scans start from there,
        // not from the oldest UTXO block (which would cause repeated re-scanning
        // and could miss newly confirmed change UTXOs).
        store.setMinBlockHeight(lastScanned)

        const keystoreFileName = store.getKeystoreFileName
        if (keystoreFileName) {
          await (window as any).walletApi.updateMinBlockHeight(keystoreFileName, lastScanned)
          ensureOpen(mine)
        }
      }

      // Read after the last await, so the UTXOs and the account belong together.
      const accountId = accountsStore.requireActiveId()
      return {
        accountId,
        result: {
          utxos: accountsStore.utxosOf(accountId),
        },
      }
    } catch (err: any) {
      await (window as any).logger?.error('Error getting UTXOs:', err.message)
      throw err
    }
  }

  /** Spendable (not pending, timelock released) and total amount of `utxos`. */
  const sumUtxos = (utxos: any[], now: number) => {
    let spendable = 0
    let total = 0
    for (const utxo of utxos) {
      const amount = parseFloat(utxo.amount || '0')
      if (isNaN(amount)) continue
      total += amount
      const isPending = !!((utxo as any).isPending ?? (utxo as any).is_pending)
      if (isPending) continue
      const isTimelocked = !!((utxo as any).isTimelocked ?? (utxo as any).is_timelocked)
      if (!isTimelocked) {
        spendable += amount
      } else {
        const releaseDateMs = (utxo as any).releaseDateMs ?? (utxo as any).release_date_ms ?? null
        if (releaseDateMs !== null && releaseDateMs <= now) {
          spendable += amount
        }
      }
    }
    return { spendable, total }
  }

  /** Available balances from the UTXOs just scanned; pending stays until `getBalance` asks the mempool. */
  const refreshAvailable = () => {
    const now = Date.now()
    for (const { accountId } of accountsStore.keyRefs) {
      const available = sumUtxos(accountsStore.utxosOf(accountId), now).spendable.toFixed(8)
      accountsStore.setBalance(accountId, {
        ...accountsStore.balances.get(accountId),
        available,
      })
    }
    store.setBalance(accountsStore.balances.get(accountsStore.requireActiveId())?.available ?? null)
  }

  /** Balances of every opened account; returns the active one's. */
  const getBalance = async (): Promise<BalanceResult> => {
    try {
      const now = Date.now()
      const mine = currentEpoch()
      // The accounts asked about; one added meanwhile gets its pending on the next call.
      const refs = accountsStore.keyRefs
      // Without a mempool answer pending is unknown, not 0.
      let pendingByAccount: Map<string, string> | undefined
      const pendingData = await walletApi.getPendingBalance({ accounts: refs })
      ensureOpen(mine)
      if (pendingData.success) {
        pendingByAccount = new Map(
          pendingData.by_account.map((p: { account_id: string; pending: string }) => [
            p.account_id,
            parseFloat(p.pending).toFixed(8),
          ]),
        )
      } else {
        await (window as any).logger?.warn('Pending balance unavailable:', pendingData.error)
      }
      const pendingOf = (id: string) => {
        if (pendingByAccount === undefined || !refs.some((r) => r.accountId === id)) {
          return undefined
        }
        const pending = pendingByAccount.get(id)
        if (pending === undefined) throw new Error(`No pending balance for account ${id}`)
        return pending
      }
      for (const { accountId } of refs) {
        const { spendable } = sumUtxos(accountsStore.utxosOf(accountId), now)
        accountsStore.setBalance(accountId, {
          available: spendable.toFixed(8),
          pending: pendingOf(accountId),
        })
      }

      const activeId = accountsStore.requireActiveId()
      const { spendable, total } = sumUtxos(accountsStore.utxosOf(activeId), now)
      const balanceStr = spendable.toFixed(8)
      const pendingBalance = pendingOf(activeId)
      store.setBalance(balanceStr)
      store.setPendingBalance(pendingBalance ?? null)
      return {
        balance: balanceStr,
        pendingBalance: pendingBalance,
        totalBalance: total.toFixed(8),
      }
    } catch (err: any) {
      await (window as any).logger?.error('Error getting balance:', err.message)
      store.setBalance(null)
      store.setPendingBalance(null)
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

    const payload: any = {
      spendingKeyHex: args.spendingKeyHex || store.getSpendingKey,
      inputUtxoIds: args.inputUtxoIds,
      minBlockHeight: toFiniteNumber(minBlockHeight),
      outputAddresses: args.outputAddresses,
      outputAmounts: args.outputAmounts,
      fee: args.fee,
      lowMemoryProving: settingsStore.lowMemoryProving,
    }

    if (args.timelockReleaseDate !== undefined) {
      payload.timelockReleaseDate = args.timelockReleaseDate
    }

    // Forward per-input spending keys when provided (mixed-key transactions).
    if (args.spendingKeyHexes !== undefined && args.spendingKeyHexes.length > 0) {
      payload.spendingKeyHexes = args.spendingKeyHexes
    }
    try {
      const response = await (window as any).walletApi.buildTransaction(payload)
      if (!response?.success) {
        await (window as any).logger?.error(
          `[SendTx] walletApi.buildTransaction returned error: ${response?.error || 'unknown'}`,
        )
      }
      return response
    } catch (err: any) {
      const errorMessage = err?.message || err?.error || 'Unknown build transaction error'
      await (window as any).logger?.error(
        `[SendTx] buildTransaction invoke failed: ${errorMessage}`,
      )
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

  /** Spends `accountId`'s UTXOs with that account's keys, whichever account is shown now. */
  const selectUtxos = (
    utxos: any[],
    neededAmount: number,
    fee: number,
    maxInputs: number,
    accountId: string,
  ) => {
    const keys = accountsStore.keys.get(accountId)
    if (!keys) throw new Error(`Account ${accountId} is locked`)
    return selectSpendableUtxos(
      utxos,
      neededAmount,
      fee,
      { genKey: keys.generationKeyHex, dctidhKey: keys.dctidhKeyHex },
      maxInputs,
    )
  }

  const clearWallet = () => {
    store.clearWallet()
    lock()
  }

  return {
    generateWallet,
    recoverWalletFromSeed,
    getSpendingKeyFromSeed,
    loadBothAddresses,
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
    checkKeystore,

    clearWallet,
  }
}
