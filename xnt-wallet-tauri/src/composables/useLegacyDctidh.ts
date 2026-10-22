import { computed, ref } from 'vue'
import { walletApi } from '@/api/tauri/walletApi'
import { useAccountsStore, useSettingsStore } from '@/stores'
import type { LegacyDctidhMigration } from '@/types'
import { describeError, toAtomicUnits } from '@/utils'
import { useNeptuneWallet } from './useNeptuneWallet'
import { useTransaction } from './useTransaction'

/** Atomic units; a legacy UTXO at or below it is never worth moving. */
const dustThreshold = ref<number>()

export function useLegacyDctidh() {
  const accountsStore = useAccountsStore()
  const settingsStore = useSettingsStore()
  const { broadcastSignedTransaction } = useNeptuneWallet()
  const { addTransaction } = useTransaction()

  const legacyUtxos = computed(() =>
    accountsStore.activeId === undefined
      ? []
      : accountsStore.legacyDctidhUtxosOf(accountsStore.activeId),
  )
  const legacyAmount = computed(() =>
    legacyUtxos.value.reduce((sum, utxo) => sum + Number(utxo.amount), 0),
  )
  const pendingCount = computed(() => legacyUtxos.value.filter((u) => u.isPending).length)
  /** Unknown until the dust threshold is loaded. */
  const movableCount = computed(() => {
    const threshold = dustThreshold.value
    if (threshold === undefined) return undefined
    return legacyUtxos.value.filter((u) => !u.isPending && toAtomicUnits(u.amount) > threshold)
      .length
  })

  const loadDustThreshold = async () => {
    dustThreshold.value ??= toAtomicUnits(await walletApi.legacyDctidhDustThreshold())
  }

  const migrate = async (): Promise<LegacyDctidhMigration> => {
    const accountId = accountsStore.requireActiveId()
    const keys = accountsStore.keys.get(accountId)
    if (!keys) throw new Error(`Account ${accountId} is locked`)
    const built = await walletApi.buildLegacyDctidhMigration({
      dctidhKeyHex: keys.dctidhKeyHex,
      lowMemoryProving: settingsStore.lowMemoryProving,
    })
    if (!built.success) throw new Error(describeError(built.error))
    const migration: LegacyDctidhMigration = built
    const [firstDigest] = migration.outputUtxoDigests
    if (firstDigest === undefined) throw new Error('The migration has no output')
    if (migration.toAddress !== keys.dctidhAddress) {
      throw new Error("The migration doesn't pay this account's current dCTIDH address")
    }

    const submitted = await broadcastSignedTransaction(built.transaction)
    if (!submitted?.success) {
      throw new Error(
        `Node rejected the transaction: ${describeError(submitted?.error ?? submitted)}`,
      )
    }
    addTransaction(
      {
        id: firstDigest,
        txId: migration.txId,
        outputCommitments: migration.outputUtxoDigests,
        toAddress: migration.toAddress,
        amount: Number(migration.amount),
        fee: Number(migration.fee),
        time: new Date().toISOString(),
      },
      accountId,
    )
    return migration
  }

  return { legacyUtxos, legacyAmount, pendingCount, movableCount, loadDustThreshold, migrate }
}
