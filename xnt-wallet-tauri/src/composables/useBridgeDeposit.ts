import { computed, ref, onActivated, onDeactivated, onUnmounted, watch } from 'vue'
import type { BridgeDepositRecord, PayloadBuildBridgeDeposit } from '@/types'
import { useAccountsStore, useSettingsStore } from '@/stores'
import { BRIDGE_POLLING_INTERVAL, toAtomicUnits } from '@/utils'
import { useNeptuneWallet } from './useNeptuneWallet'

export interface BridgeDepositForm {
  midenAddress: string
  amount: string
  fee: string
}

/** Text of a JSON-RPC error object (`{ code, message, data? }`) or any other thrown value. */
const describeError = (error: unknown): string => {
  if (typeof error !== 'object' || error === null) return String(error)
  const message = 'message' in error ? String(error.message) : JSON.stringify(error)
  return 'data' in error && error.data !== undefined
    ? `${message}: ${JSON.stringify(error.data)}`
    : message
}

const isPending = (record: BridgeDepositRecord) =>
  record.status.state === 'submitted' || record.status.state === 'confirmed'

export const useBridgeDeposit = () => {
  const { getUtxos, selectUtxos, broadcastSignedTransaction } = useNeptuneWallet()
  const settingsStore = useSettingsStore()
  const accountsStore = useAccountsStore()
  const allDeposits = ref<BridgeDepositRecord[]>([])
  const deposits = computed(() =>
    allDeposits.value.filter((d) => d.walletAccount === accountsStore.activeId),
  )
  const refreshError = ref<string>()
  let timer: ReturnType<typeof setTimeout> | undefined
  let disposed = false
  // Kept-alive views pause polling while their tab is hidden.
  let active = true
  // Responses older than the newest applied one are dropped (a slow poll must not undo a submit).
  let requestSeq = 0
  let appliedSeq = 0

  const walletApi = () => (window as any).walletApi

  const schedule = () => {
    clearTimeout(timer)
    timer = undefined
    if (!disposed && active && deposits.value.some(isPending)) {
      timer = setTimeout(refresh, BRIDGE_POLLING_INTERVAL)
    }
  }

  const apply = (seq: number, records: BridgeDepositRecord[]) => {
    if (seq < appliedSeq) return
    appliedSeq = seq
    allDeposits.value = [...records].sort((a, b) => b.createdMs - a.createdMs)
  }

  /** Saved records with their last known status; no network. */
  const load = async () => {
    const seq = ++requestSeq
    apply(seq, await walletApi().listBridgeDeposits())
  }

  async function refresh() {
    const seq = ++requestSeq
    try {
      apply(seq, await walletApi().refreshBridgeDeposits())
      refreshError.value = undefined
    } catch (error: unknown) {
      refreshError.value = error instanceof Error ? error.message : String(error)
    } finally {
      schedule()
    }
  }

  /** Builds, proves and submits a deposit; resolves once the node accepted it. */
  const submitDeposit = async (form: BridgeDepositForm) => {
    const utxosResult = await getUtxos()
    const utxos = utxosResult?.result?.utxos || utxosResult?.utxos || []
    const walletAccount: string = utxosResult.accountId
    const selection = selectUtxos(
      utxos,
      Number(form.amount),
      Number(form.fee),
      settingsStore.maxInputsPerTx,
      walletAccount,
    )
    if (selection.selectedUtxos.length === 0) {
      throw new Error(
        'No spendable UTXOs: wait for your pending transaction to confirm or drop, then try again',
      )
    }
    const needed = toAtomicUnits(form.amount) + toAtomicUnits(form.fee)
    if (toAtomicUnits(selection.totalAmount) < needed) {
      throw new Error(
        `Your ${settingsStore.maxInputsPerTx} largest spendable UTXOs hold ${selection.totalAmount} XNT; amount + fee is ${needed / 1e8} XNT`,
      )
    }

    const payload: PayloadBuildBridgeDeposit = {
      inputAdditionRecords: selection.selectedUtxos.map(
        (utxo: any) => utxo.addition_record_hex || utxo.additionRecord || utxo.addition_record,
      ),
      midenAddress: form.midenAddress.trim(),
      amount: form.amount,
      fee: form.fee,
      spendingKeyHex: selection.spendingKeyHex,
      spendingKeyHexes: selection.spendingKeyHexes,
      lowMemoryProving: settingsStore.lowMemoryProving,
      walletAccount,
    }
    const built = await walletApi().buildBridgeDeposit(payload)
    if (!built.success) throw new Error(describeError(built.error))

    // The backend recorded the deposit before returning; forget it if the node rejects it.
    const submitted = await broadcastSignedTransaction(built.transaction)
    const seq = ++requestSeq
    if (!submitted?.success) {
      apply(seq, await walletApi().discardBridgeDeposit(built.utxoCommitment))
      const reason = describeError(submitted?.error ?? submitted)
      await (window as any).logger?.error(`[Bridge] node rejected deposit: ${reason}`)
      throw new Error(`Node rejected the deposit: ${reason}`)
    }
    apply(seq, await walletApi().listBridgeDeposits())
    schedule()
  }

  onActivated(() => {
    active = true
  })
  // Another account may have deposits that still need polling.
  watch(() => accountsStore.activeId, schedule)
  onDeactivated(() => {
    active = false
    clearTimeout(timer)
  })
  onUnmounted(() => {
    disposed = true
    clearTimeout(timer)
  })

  return { deposits, refreshError, load, refresh, submitDeposit }
}
