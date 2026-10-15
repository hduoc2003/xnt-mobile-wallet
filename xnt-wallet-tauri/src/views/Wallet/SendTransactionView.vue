<script setup lang="ts">
import { CopyButton, ExplorerLink } from '@/components/commons'
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { toast } from 'vue-sonner'
import { Send, ArrowLeft } from 'lucide-vue-next'
import { useNeptuneWallet, useFormat, useTransaction } from '@/composables'
import { useNeptuneStore } from '@/stores'
import { DEFAULT_FEE, DEFAULT_MAX_INPUTS, toAtomicUnits } from '@/utils'
import { MaxInputsControl } from '@/components/commons'
import ConfirmTransactionDrawer from './components/ConfirmTransactionDrawer.vue'
import ProofGenerationModal from './components/ProofGenerationModal.vue'
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'

const router = useRouter()
const store = useNeptuneStore()
const {
  getBalance,
  getUtxos,
  forceRescanUtxos,
  buildTransaction,
  broadcastSignedTransaction,
  selectUtxos,
} = useNeptuneWallet()
const { addTransaction } = useTransaction()
const { formatXnt } = useFormat()

const availableBalance = ref('0')
const maxInputs = ref(DEFAULT_MAX_INPUTS)
const loadedUtxos = ref<any[]>([])
const maxSendable = computed(() => computeMaxSendable(loadedUtxos.value, maxInputs.value))
const pendingBalance = ref('0')
const loading = ref(false)
const sendLoading = ref(false)
const showConfirmModal = ref(false)
const showProofModal = ref(false)
const showLimitInfo = ref(false)
const showManualInfo = ref(false)

const outputAddress = ref('')
const outputAmount = ref('')
const priorityFee = ref(DEFAULT_FEE)
const enableTimelock = ref(false)
const timelockDate = ref('')
const timelockTime = ref('12:00') // Default to noon

// Manual UTXO selection (advanced): let the user pick exactly which UTXOs fund
// this send instead of the default auto-selection (largest UTXOs, capped by Max inputs).
const manualMode = ref(false)
const allUtxos = ref<any[]>([])
const manualSelected = ref<any[]>([])
const nowMs = Date.now()

const loadBalance = async () => {
  try {
    loading.value = true
    const result = await getBalance()
    availableBalance.value = result?.balance ?? '0'
    pendingBalance.value = result?.pendingBalance ?? '0'
  } catch (error) {
    toast.error('Failed to load balance')
  } finally {
    loading.value = false
  }
}

const computeMaxSendable = (utxos: any[], limit: number): string => {
  if (!Array.isArray(utxos) || utxos.length === 0) return '0'

  const now = Date.now()
  const spendableUtxos = utxos.filter((u: any) => {
    if (u.is_pending ?? u.isPending) return false
    const isTimelocked = !!(u.is_timelocked ?? u.isTimelocked)
    if (!isTimelocked) return true
    const releaseDateMs = u.release_date_ms ?? u.releaseDateMs ?? null
    return releaseDateMs !== null && Number(releaseDateMs) <= now
  })

  if (spendableUtxos.length === 0) return '0'

  const sorted = [...spendableUtxos].sort(
    (a: any, b: any) => parseFloat(b.amount || '0') - parseFloat(a.amount || '0'),
  )

  const sumAtomic = sorted.slice(0, limit).reduce((acc: number, u: any) => {
    const amt = toAtomicUnits(u.amount || '0')
    return acc + (isNaN(amt) ? 0 : amt)
  }, 0)
  return (sumAtomic / 1e8).toString()
}

const spendableOnly = (utxos: any[]): any[] =>
  Array.isArray(utxos) ? utxos.filter((u: any) => !(u.is_pending ?? u.isPending)) : []

const loadMaxSendable = async () => {
  try {
    const utxosResult = await getUtxos()
    const utxos = utxosResult?.result?.utxos || utxosResult?.utxos || []
    loadedUtxos.value = utxos
    allUtxos.value = spendableOnly(utxos)
  } catch (error) {
    const cachedUtxos = store.getUtxos || []
    loadedUtxos.value = cachedUtxos
    allUtxos.value = spendableOnly(cachedUtxos)
  }
}

// --- Manual UTXO selection helpers (used when manualMode is on) ---
const utxoKey = (u: any): string =>
  u.additionRecord || u.addition_record_hex || u.addition_record || u.utxoDigest || ''

// Minimal digest display: first 4 + last 4 hex chars (drops any 0x prefix).
const shortDigest = (d: string): string => {
  const s = (d || '').replace(/^0x/, '')
  return s.length <= 8 ? s : `${s.slice(0, 4)}...${s.slice(-4)}`
}

const isUtxoTimelocked = (u: any): boolean => {
  const timelocked = !!(u.is_timelocked ?? u.isTimelocked)
  if (!timelocked) return false
  const releaseMs = u.release_date_ms ?? u.releaseDateMs ?? null
  return releaseMs === null || Number(releaseMs) > nowMs
}

const isSelectedUtxo = (u: any): boolean =>
  manualSelected.value.some((s) => utxoKey(s) === utxoKey(u))

const toggleUtxo = (u: any) => {
  if (isUtxoTimelocked(u) && !isSelectedUtxo(u)) return
  if (isSelectedUtxo(u)) {
    manualSelected.value = manualSelected.value.filter((s) => utxoKey(s) !== utxoKey(u))
  } else if (manualSelected.value.length >= maxInputs.value) {
    toast.error(`At most ${maxInputs.value} inputs per transaction; raise Max inputs to pick more`)
  } else {
    manualSelected.value = [...manualSelected.value, u]
  }
}

// Exact sum of selected UTXO amounts (via integer atomic units).
const manualSelectedTotal = computed(
  () => manualSelected.value.reduce((s, u) => s + toAtomicUnits(u.amount || '0'), 0) / 1e8,
)

// Per-input spending key, one per selected UTXO, matched by key type. The backend
// accepts this to unlock mixed-key (Generation + dCTIDH) inputs in one transaction.
// NOTE: store.getSpendingKey = Generation key; store.getGenerationSpendingKey = dCTIDH key.
const keyForUtxo = (u: any): string =>
  u.keyType === 'Generation'
    ? (store.getSpendingKey ?? store.getGenerationSpendingKey ?? '')
    : (store.getGenerationSpendingKey ?? store.getSpendingKey ?? '')

const manualSpendingKeyHexes = computed(() => manualSelected.value.map(keyForUtxo))
const manualPrimaryKey = computed(
  () =>
    manualSpendingKeyHexes.value[0] || store.getSpendingKey || store.getGenerationSpendingKey || '',
)

// Change returned to the user in manual mode (selected − amount − fee).
const manualChange = computed(() => {
  const amt = parseFloat(outputAmount.value || '0')
  const fee = parseFloat(priorityFee.value || '0')
  const changeAtomic =
    toAtomicUnits(manualSelectedTotal.value) -
    toAtomicUnits(isNaN(amt) ? 0 : amt) -
    toAtomicUnits(isNaN(fee) ? 0 : fee)
  return changeAtomic / 1e8
})

// bech32m addresses are canonically lowercase; mobile keyboards can auto-capitalize
// the recipient field, yielding a mixed-case string the parser rejects. Normalize here.
const normalizedAddress = computed(() => outputAddress.value.trim().toLowerCase())
const isAddressValid = computed(() => normalizedAddress.value.length > 0)
const isAmountValid = computed(() => {
  if (!outputAmount.value) return false
  const num = parseFloat(outputAmount.value)
  if (isNaN(num) || num <= 0) return false

  const balance = parseFloat(availableBalance.value)
  if (!isNaN(balance) && num > balance) return false

  if (manualMode.value) {
    if (manualSelected.value.length === 0 || manualSelected.value.length > maxInputs.value)
      return false
    const fee = parseFloat(priorityFee.value || '0')
    const need = toAtomicUnits(num) + toAtomicUnits(isNaN(fee) ? 0 : fee)
    return toAtomicUnits(manualSelectedTotal.value) >= need
  }

  const maxPerTx = parseFloat(maxSendable.value || '0')
  if (!isNaN(maxPerTx) && maxPerTx > 0 && num > maxPerTx) return false

  return true
})
const isFeeValid = computed(() => {
  if (!priorityFee.value) return false
  const num = parseFloat(priorityFee.value)
  return !isNaN(num) && num >= 0
})

const isHighFee = computed(() => {
  const num = parseFloat(priorityFee.value)
  return !isNaN(num) && num > 2
})

const amountErrorMessage = computed(() => {
  if (!outputAmount.value) return ''
  const num = parseFloat(outputAmount.value)
  if (isNaN(num) || num <= 0) return 'Invalid amount'

  const balance = parseFloat(availableBalance.value)
  if (!isNaN(balance) && num > balance) {
    return `Insufficient balance. Available: ${availableBalance.value} XNT`
  }

  if (manualMode.value) {
    if (manualSelected.value.length === 0) return 'Select at least one UTXO below'
    if (manualSelected.value.length > maxInputs.value) {
      return `${manualSelected.value.length} UTXOs selected; Max inputs is ${maxInputs.value}`
    }
    const fee = parseFloat(priorityFee.value || '0')
    const need = toAtomicUnits(num) + toAtomicUnits(isNaN(fee) ? 0 : fee)
    if (toAtomicUnits(manualSelectedTotal.value) < need) {
      return `Selected UTXOs (${formatXnt(manualSelectedTotal.value)} XNT) don't cover amount + fee`
    }
    return ''
  }

  const maxPerTx = parseFloat(maxSendable.value || '0')
  if (!isNaN(maxPerTx) && maxPerTx > 0 && num > maxPerTx) {
    return `Maximum per transaction is ${formatXnt(maxPerTx)} XNT (sum of your ${maxInputs.value} largest UTXOs)`
  }

  return ''
})

/** True when user has no spendable balance but has pending (e.g. tx stuck in mempool) */
const balanceStuckInPending = computed(() => {
  const available = parseFloat(availableBalance.value)
  const pending = parseFloat(pendingBalance.value)
  return !isNaN(available) && !isNaN(pending) && available === 0 && pending > 0
})

const isTimelockValid = computed(() => {
  if (!enableTimelock.value) return true
  if (!timelockDate.value) return false

  const timeValue = timelockTime.value || '00:00'
  const dateTimeString = `${timelockDate.value}T${timeValue}`
  const selectedDate = new Date(dateTimeString)
  const now = new Date()

  if (isNaN(selectedDate.getTime())) {
    return false
  }

  return selectedDate.getTime() > now.getTime()
})

const timelockErrorMessage = computed(() => {
  if (!enableTimelock.value) return ''
  if (!timelockDate.value) return 'Release date is required when timelock is enabled'

  const timeValue = timelockTime.value || '00:00'
  const dateTimeString = `${timelockDate.value}T${timeValue}`
  const selectedDate = new Date(dateTimeString)
  const now = new Date()

  if (isNaN(selectedDate.getTime())) {
    return 'Invalid date or time'
  }

  if (selectedDate.getTime() <= now.getTime()) {
    return 'Release date must be in the future'
  }
  return ''
})

const isFormValid = computed(
  () =>
    isAddressValid.value &&
    isAmountValid.value &&
    isFeeValid.value &&
    isTimelockValid.value &&
    !sendLoading.value,
)

const formatDecimal = (value: string) => {
  if (!value) return ''

  let cleaned = value.replace(/[^\d.]/g, '')

  const parts = cleaned.split('.')
  if (parts.length > 2) {
    cleaned = parts[0] + '.' + parts.slice(1).join('')
  }

  if (cleaned && !cleaned.includes('.') && /^\d+$/.test(cleaned)) {
    cleaned = cleaned + '.0'
  }

  if (cleaned.includes('.')) {
    const [integer, decimal = ''] = cleaned.split('.')
    cleaned = integer + '.' + decimal.slice(0, 8)
  }

  return cleaned
}

const handleAmountBlur = () => {
  if (outputAmount.value) {
    outputAmount.value = formatDecimal(outputAmount.value)
  }
}

const handleFeeBlur = () => {
  if (priorityFee.value) {
    priorityFee.value = formatDecimal(priorityFee.value)
  }
}

const handleShowConfirm = () => {
  if (!isFormValid.value) return
  showConfirmModal.value = true
}

const handleCancelConfirm = () => {
  showConfirmModal.value = false
}

const handleConfirmSend = async () => {
  try {
    sendLoading.value = true
    showConfirmModal.value = false
    showProofModal.value = true

    // Get UTXOs (rescan so cache is fresh)
    const utxosResult = await getUtxos()
    const utxos = utxosResult?.result?.utxos || utxosResult?.utxos || []

    if (!utxos.length) {
      // Refresh balance to get pending; if pending > 0, user likely has a tx stuck in mempool
      const balanceResult = await getBalance()
      const pending = parseFloat(balanceResult?.pendingBalance || '0')
      if (!isNaN(pending) && pending > 0) {
        toast.error('Cannot send: balance is in a pending transaction', {
          description:
            'Wait for that transaction to confirm or be dropped from the mempool, then try again.',
        })
      } else {
        toast.error('No UTXOs available')
      }
      return
    }

    const outputAmountNum = parseFloat(outputAmount.value)
    const feeAmount = parseFloat(priorityFee.value)

    if (isNaN(outputAmountNum) || isNaN(feeAmount)) {
      toast.error('Invalid amount or fee')
      return
    }

    let chosenUtxos: any[]
    let spendingKeyHex: string | undefined
    let spendingKeyHexes: string[]
    let selectedTotal: number

    if (manualMode.value) {
      chosenUtxos = manualSelected.value
      spendingKeyHex = manualPrimaryKey.value || undefined
      spendingKeyHexes = manualSpendingKeyHexes.value
      selectedTotal = manualSelectedTotal.value
    } else {
      const sel = selectUtxos(utxos, outputAmountNum, feeAmount, maxInputs.value)
      chosenUtxos = sel.selectedUtxos
      spendingKeyHex = sel.spendingKeyHex
      spendingKeyHexes = sel.spendingKeyHexes
      selectedTotal = sel.totalAmount
    }

    if (chosenUtxos.length === 0) {
      toast.error(manualMode.value ? 'Select at least one UTXO' : 'No suitable UTXOs found')
      return
    }

    if (toAtomicUnits(selectedTotal) < toAtomicUnits(outputAmountNum) + toAtomicUnits(feeAmount)) {
      toast.error(
        `Insufficient funds. Selected: ${selectedTotal.toFixed(8)} XNT, Needed: ${(outputAmountNum + feeAmount).toFixed(8)} XNT`,
      )
      return
    }

    // Build transaction with the chosen UTXOs only
    const inputAdditionRecords = chosenUtxos.map(
      (utxo: any) => utxo.addition_record_hex || utxo.additionRecord || utxo.addition_record,
    )

    let timelockReleaseDate: number | undefined = undefined
    if (enableTimelock.value && timelockDate.value) {
      const timeValue = timelockTime.value || '00:00'
      const dateTimeString = `${timelockDate.value}T${timeValue}`
      const selectedDate = new Date(dateTimeString)
      const now = Date.now()

      if (isNaN(selectedDate.getTime())) {
        toast.error('Invalid date or time')
        return
      }

      if (selectedDate.getTime() <= now) {
        toast.error('Release date must be in the future')
        return
      }

      timelockReleaseDate = selectedDate.getTime()
    }

    const txResult = await buildTransaction({
      inputAdditionRecords,
      outputAddresses: [normalizedAddress.value],
      outputAmounts: [outputAmount.value],
      fee: priorityFee.value,
      timelockReleaseDate,
      spendingKeyHex,
      spendingKeyHexes,
    })

    if (!txResult.success) {
      toast.error('Failed to build transaction', { description: txResult.error || '' })
      return
    }

    // Get output UTXO digests from buildTransaction response (client-side computed)
    const outputUtxoDigests = txResult.outputUtxoDigests || []

    if (!outputUtxoDigests || outputUtxoDigests.length === 0) {
      toast.error('Failed to get output UTXO digests from transaction')
      return
    }

    // Broadcast transaction
    const submitResult = await broadcastSignedTransaction(txResult.transaction)

    if (!submitResult?.success) {
      toast.error(submitResult?.error || 'Failed to broadcast transaction')
      return
    }

    // Use first output UTXO digest (from buildTransaction, not server response)
    const firstDigest = outputUtxoDigests[0]

    if (!firstDigest || typeof firstDigest !== 'string') {
      toast.error('Transaction submitted but no UTXO digest available')
      return
    }

    // Store transaction locally for history view
    const amountNum = parseFloat(outputAmount.value)
    const feeNum = parseFloat(priorityFee.value)

    addTransaction({
      id: firstDigest,
      txId: txResult.txId,
      outputCommitments: outputUtxoDigests,
      toAddress: normalizedAddress.value,
      amount: Number.isFinite(amountNum) ? amountNum : 0,
      fee: Number.isFinite(feeNum) ? feeNum : 0,
      time: new Date().toISOString(),
    })

    toast.success('Transaction sent successfully!')

    // Wait a bit for transaction to be added to mempool, then refresh balance
    setTimeout(async () => {
      try {
        await getBalance()
      } catch (err) {
        // Log error but don't block navigation
        console.error('Failed to refresh balance after transaction:', err)
      }
    }, 1000) // 1 second delay to allow mempool to update

    // Navigate back to wallet
    router.push('/')
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Failed to send transaction'
    toast.error('Transaction Failed', { description: message })
  } finally {
    sendLoading.value = false
    showProofModal.value = false
  }
}

onMounted(async () => {
  // Scan UTXOs first so the store has fresh data, then compute balance from
  // that fresh store. Running them in parallel caused a race where loadBalance()
  // could read the store while getUtxos() was mid-way through overwriting it
  // with the disk cache (before the RPC scan completed), resulting in expired
  // timelocked UTXOs being excluded from the spendable balance.
  await loadMaxSendable()
  await loadBalance()
})
</script>

<template>
  <div class="p-4">
    <div class="mx-auto max-w-2xl space-y-6">
      <!-- Header -->
      <div class="flex items-center gap-3">
        <Button variant="ghost" size="icon" @click="router.push('/')">
          <ArrowLeft class="size-5" />
        </Button>
        <div>
          <h1 class="text-2xl font-bold text-foreground">Send Transaction</h1>
          <p class="text-sm text-muted-foreground">Send XNT to another address</p>
        </div>
      </div>

      <!-- Mempool-stuck notice: balance is in a pending tx so user cannot send until it confirms or drops -->
      <Alert v-if="balanceStuckInPending" variant="destructive" class="mb-2">
        <AlertTitle>Balance in pending transaction</AlertTitle>
        <AlertDescription>
          Your {{ formatXnt(pendingBalance) }} XNT is currently in a transaction waiting in the
          mempool. You cannot send again until that transaction confirms or is dropped. This applies
          on this device and if you switch device.
        </AlertDescription>
      </Alert>

      <!-- Send Form -->
      <Card>
        <CardHeader>
          <CardTitle class="flex items-center gap-2">
            <Send class="size-5" />
            Transaction Details
          </CardTitle>
        </CardHeader>
        <CardContent class="space-y-4">
          <!-- Recipient Address -->
          <div class="space-y-2">
            <Label for="recipient">
              Recipient Address <span class="text-destructive">*</span>
            </Label>
            <Textarea
              id="recipient"
              v-model="outputAddress"
              placeholder="Enter recipient address"
              rows="3"
              class="font-mono text-sm"
              autocapitalize="none"
              autocorrect="off"
              spellcheck="false"
              :disabled="sendLoading"
            />
            <p v-if="outputAddress && !isAddressValid" class="text-xs text-destructive">
              Address is required
            </p>
          </div>

          <!-- Amount and Fee Row -->
          <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
            <div class="space-y-2">
              <div class="flex items-center justify-between">
                <Label for="amount"> Amount <span class="text-destructive">*</span> </Label>
                <div v-if="!manualMode" class="flex items-center gap-2">
                  <Badge variant="outline" class="text-xs">
                    Max: {{ formatXnt(maxSendable) }} XNT
                  </Badge>
                  <button
                    type="button"
                    class="flex h-5 w-5 items-center justify-center rounded-full border text-[10px] leading-none"
                    @click="showLimitInfo = !showLimitInfo"
                  >
                    ?
                  </button>
                </div>
              </div>
              <Input
                id="amount"
                v-model="outputAmount"
                type="text"
                placeholder="0.0"
                :disabled="sendLoading"
                @blur="handleAmountBlur"
              />
              <p v-if="showLimitInfo && !manualMode" class="text-[11px] text-muted-foreground">
                You can spend at most {{ formatXnt(maxSendable) }} XNT in a single transaction. This
                limit is the sum of your {{ maxInputs }} largest UTXOs (Max inputs below).
              </p>
              <p v-if="amountErrorMessage" class="text-xs text-destructive">
                {{ amountErrorMessage }}
              </p>
            </div>

            <div class="space-y-2">
              <Label for="fee"> Priority Fee <span class="text-destructive">*</span> </Label>
              <Input
                id="fee"
                v-model="priorityFee"
                type="text"
                placeholder="0.0"
                :disabled="sendLoading"
                @blur="handleFeeBlur"
              />
              <p v-if="priorityFee && !isFeeValid" class="text-xs text-destructive">Invalid fee</p>
              <p v-if="isHighFee" class="text-xs text-yellow-500 dark:text-yellow-400">
                ⚠️ Fee is unusually high ({{ priorityFee }} XNT). Please double-check you haven't
                confused the fee with the amount.
              </p>
            </div>
          </div>

          <MaxInputsControl v-model="maxInputs" :disabled="sendLoading" />

          <!-- Advanced: manual UTXO selection -->
          <div class="space-y-3 rounded-lg border border-border bg-muted/30 p-4">
            <div class="flex items-center justify-between">
              <div class="space-y-0.5">
                <div class="flex items-center gap-2">
                  <Label for="manual-utxo" class="text-base font-medium">
                    Choose UTXOs manually
                  </Label>
                  <button
                    type="button"
                    class="flex h-5 w-5 items-center justify-center rounded-full border text-[10px] leading-none"
                    @click="showManualInfo = !showManualInfo"
                  >
                    ?
                  </button>
                </div>
                <p class="text-xs text-muted-foreground">
                  Pick exactly which UTXOs fund this transaction (up to Max inputs).
                </p>
              </div>
              <Switch
                id="manual-utxo"
                :checked="manualMode"
                :disabled="sendLoading"
                @click.stop="manualMode = !manualMode"
              />
            </div>

            <p v-if="showManualInfo" class="text-xs text-muted-foreground">
              💡 By default the wallet auto-selects up to {{ maxInputs }} of your largest UTXOs. If
              sending crashes during proof generation, lower Max inputs or pick fewer UTXOs here to
              build a lighter transaction.
            </p>

            <template v-if="manualMode">
              <!-- Selection summary -->
              <div class="flex flex-wrap items-center gap-2 text-xs">
                <Badge variant="outline">{{ manualSelected.length }} selected</Badge>
                <Badge variant="outline">Total: {{ formatXnt(manualSelectedTotal) }} XNT</Badge>
                <Badge
                  v-if="
                    manualSelected.length > 0 && outputAmount && isFeeValid && manualChange >= 0
                  "
                  variant="secondary"
                >
                  Change: {{ formatXnt(manualChange) }} XNT
                </Badge>
              </div>

              <!-- UTXO list (page scrolls; no inner scroll box) -->
              <div
                v-if="allUtxos.length === 0"
                class="py-4 text-center text-xs text-muted-foreground"
              >
                No UTXOs available
              </div>
              <div v-else class="space-y-2">
                <div
                  v-for="utxo in allUtxos"
                  :key="utxoKey(utxo)"
                  class="flex items-start gap-3 rounded-lg border border-border p-3 transition-colors"
                  :class="{
                    'border-primary bg-primary/5': isSelectedUtxo(utxo),
                    'cursor-not-allowed opacity-50': isUtxoTimelocked(utxo),
                    'cursor-pointer hover:bg-muted/50': !isUtxoTimelocked(utxo),
                  }"
                  @click="!isUtxoTimelocked(utxo) && !sendLoading && toggleUtxo(utxo)"
                >
                  <input
                    type="checkbox"
                    class="mt-1 size-4 flex-shrink-0 accent-primary"
                    :checked="isSelectedUtxo(utxo)"
                    :disabled="isUtxoTimelocked(utxo) || sendLoading"
                    @change.stop="toggleUtxo(utxo)"
                    @click.stop
                  />
                  <div class="min-w-0 flex-1 space-y-1">
                    <div class="flex flex-wrap items-center gap-2">
                      <span class="font-mono font-semibold text-primary">
                        {{ formatXnt(utxo.amount) }} XNT
                      </span>
                      <Badge variant="secondary" class="text-xs">
                        {{ utxo.keyType || 'dCTIDH' }}
                      </Badge>
                      <Badge v-if="isUtxoTimelocked(utxo)" variant="destructive" class="text-xs">
                        Locked
                      </Badge>
                    </div>
                    <div class="flex gap-3 text-xs text-muted-foreground">
                      <code class="font-mono">
                        {{ shortDigest(utxo.utxoDigest) }}
                        <CopyButton :value="utxo.utxoDigest" label="UTXO digest" />
                        <ExplorerLink :path="`/utxo/${utxo.utxoDigest}`" />
                      </code>
                    </div>
                  </div>
                </div>
              </div>
            </template>
          </div>

          <!-- Timelock Section -->
          <div class="space-y-3 rounded-lg border border-border bg-muted/30 p-4">
            <div class="flex items-center justify-between">
              <div class="space-y-0.5">
                <Label for="timelock" class="text-base font-medium"> Time Lock </Label>
                <p class="text-xs text-muted-foreground">
                  Lock the transaction outputs until a specific date
                </p>
              </div>
              <Switch
                id="timelock"
                :checked="enableTimelock"
                :disabled="sendLoading"
                @click.stop="enableTimelock = !enableTimelock"
              />
            </div>

            <!-- Date/time picker appears only when timelock is enabled -->
            <div v-if="enableTimelock" class="grid grid-cols-1 gap-4 sm:grid-cols-2">
              <div class="space-y-2">
                <Label for="timelock-date">
                  Release Date <span class="text-destructive">*</span>
                </Label>
                <input
                  id="timelock-date"
                  v-model="timelockDate"
                  type="date"
                  :min="new Date().toISOString().split('T')[0]"
                  :disabled="sendLoading"
                  class="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                />
              </div>
              <div class="space-y-2">
                <Label for="timelock-time"> Release Time </Label>
                <input
                  id="timelock-time"
                  v-model="timelockTime"
                  type="time"
                  :disabled="sendLoading"
                  class="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                />
              </div>
            </div>
            <p v-if="timelockErrorMessage" class="text-xs text-destructive">
              {{ timelockErrorMessage }}
            </p>
            <p
              v-if="enableTimelock && !timelockErrorMessage && timelockDate"
              class="text-xs text-muted-foreground"
            >
              Outputs will be locked until:
              {{
                new Date(`${timelockDate}T${timelockTime || '00:00'}`).toLocaleString(undefined, {
                  dateStyle: 'medium',
                  timeStyle: 'short',
                })
              }}
            </p>
          </div>

          <!-- Send Button -->
          <Button
            class="w-full gap-2"
            :disabled="!isFormValid || sendLoading"
            @click="handleShowConfirm"
          >
            <Send class="size-4" />
            {{ sendLoading ? 'Sending...' : 'Review & Send' }}
          </Button>
        </CardContent>
      </Card>
    </div>

    <!-- Confirm Transaction Drawer -->
    <ConfirmTransactionDrawer
      v-model:open="showConfirmModal"
      :recipient-address="normalizedAddress"
      :amount="outputAmount"
      :fee="priorityFee"
      :timelock-enabled="enableTimelock"
      :timelock-date="timelockDate"
      :timelock-time="timelockTime"
      :loading="sendLoading"
      @confirm="handleConfirmSend"
      @cancel="handleCancelConfirm"
    />

    <!-- Proof Generation Modal -->
    <ProofGenerationModal v-model:open="showProofModal" />
  </div>
</template>
