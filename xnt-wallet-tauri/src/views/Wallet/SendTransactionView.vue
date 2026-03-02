<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { toast } from 'vue-sonner'
import { Send, ArrowLeft } from 'lucide-vue-next'
import { useNeptuneWallet, useFormat, useTransaction } from '@/composables'
import ConfirmTransactionDrawer from './components/ConfirmTransactionDrawer.vue'
import ProofGenerationModal from './components/ProofGenerationModal.vue'
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'

const router = useRouter()
const { getBalance, getUtxos, forceRescanUtxos, buildTransaction, broadcastSignedTransaction, selectUtxos } = useNeptuneWallet()
const { addTransaction } = useTransaction()
const { formatAmount } = useFormat()

const availableBalance = ref('0')
const maxSendable = ref('0')
const pendingBalance = ref('0')
const loading = ref(false)
const sendLoading = ref(false)
const showConfirmModal = ref(false)
const showProofModal = ref(false)
const showLimitInfo = ref(false)

const outputAddress = ref('')
const outputAmount = ref('')
const priorityFee = ref('')
const enableTimelock = ref(false)
const timelockDate = ref('')
const timelockTime = ref('12:00') // Default to noon

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

const loadMaxSendable = async () => {
  try {
    const utxosResult = await getUtxos()
    const utxos =
      utxosResult?.result?.utxos ||
      utxosResult?.utxos ||
      []

    if (!Array.isArray(utxos) || utxos.length === 0) {
      maxSendable.value = '0'
      return
    }

    const now = Date.now()
    const spendableUtxos = utxos.filter((u: any) => {
      const isTimelocked = !!(u.is_timelocked ?? u.isTimelocked)
      if (!isTimelocked) return true
      const releaseDateMs = u.release_date_ms ?? u.releaseDateMs ?? null
      return releaseDateMs !== null && releaseDateMs <= now
    })

    if (spendableUtxos.length === 0) {
      maxSendable.value = '0'
      return
    }

    const sorted = [...spendableUtxos].sort((a: any, b: any) => {
      const amountA = parseFloat(a.amount || '0')
      const amountB = parseFloat(b.amount || '0')
      return amountB - amountA
    })

    const top5 = sorted.slice(0, 5)
    const sum = top5.reduce((acc: number, u: any) => {
      const amt = parseFloat(u.amount || '0')
      return acc + (isNaN(amt) ? 0 : amt)
    }, 0)

    maxSendable.value = sum.toString()
  } catch (error) {
    maxSendable.value = '0'
  }
}

const isAddressValid = computed(() => outputAddress.value.trim().length > 0)
const isAmountValid = computed(() => {
  if (!outputAmount.value) return false
  const num = parseFloat(outputAmount.value)
  if (isNaN(num) || num <= 0) return false

  const balance = parseFloat(availableBalance.value)
  if (!isNaN(balance) && num > balance) return false

  const maxPerTx = parseFloat(maxSendable.value || '0')
  if (!isNaN(maxPerTx) && maxPerTx > 0 && num > maxPerTx) return false

  return true
})
const isFeeValid = computed(() => {
  if (!priorityFee.value) return false
  const num = parseFloat(priorityFee.value)
  return !isNaN(num) && num >= 0
})

const amountErrorMessage = computed(() => {
  if (!outputAmount.value) return ''
  const num = parseFloat(outputAmount.value)
  if (isNaN(num) || num <= 0) return 'Invalid amount'

  const balance = parseFloat(availableBalance.value)
  if (!isNaN(balance) && num > balance) {
    return `Insufficient balance. Available: ${availableBalance.value} XNT`
  }

  const maxPerTx = parseFloat(maxSendable.value || '0')
  if (!isNaN(maxPerTx) && maxPerTx > 0 && num > maxPerTx) {
    return `Maximum per transaction is ${formatAmount(maxPerTx)} XNT (sum of your 5 largest UTXOs)`
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
  () => isAddressValid.value && isAmountValid.value && isFeeValid.value && isTimelockValid.value && !sendLoading.value
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
    const utxos =
      utxosResult?.result?.utxos ||
      utxosResult?.utxos ||
      []

    if (!utxos.length) {
      // Refresh balance to get pending; if pending > 0, user likely has a tx stuck in mempool
      const balanceResult = await getBalance()
      const pending = parseFloat(balanceResult?.pendingBalance || '0')
      if (!isNaN(pending) && pending > 0) {
        toast.error('Cannot send: balance is in a pending transaction', {
          description: 'Wait for that transaction to confirm or be dropped from the mempool, then try again.',
        })
      } else {
        toast.error('No UTXOs available')
      }
      return
    }

    // Select optimal UTXOs using coin selection algorithm
    const outputAmountNum = parseFloat(outputAmount.value)
    const feeAmount = parseFloat(priorityFee.value)
    
    if (isNaN(outputAmountNum) || isNaN(feeAmount)) {
      toast.error('Invalid amount or fee')
      return
    }

    const { selectedUtxos, totalAmount, spendingKeyHex } = selectUtxos(utxos, outputAmountNum, feeAmount)

    if (selectedUtxos.length === 0) {
      toast.error('No suitable UTXOs found')
      return
    }

    if (totalAmount < outputAmountNum + feeAmount) {
      toast.error(`Insufficient funds. Selected: ${totalAmount.toFixed(8)} XNT, Needed: ${(outputAmountNum + feeAmount).toFixed(8)} XNT`)
      return
    }

    // Build transaction with selected UTXOs only
    const inputAdditionRecords = selectedUtxos.map(
      (utxo: any) =>
        utxo.addition_record_hex ||
        utxo.additionRecord ||
        utxo.addition_record
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
      outputAddresses: [outputAddress.value.trim()],
      outputAmounts: [outputAmount.value],
      fee: priorityFee.value,
      timelockReleaseDate,
      spendingKeyHex,
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
      toAddress: outputAddress.value.trim(),
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
  await Promise.all([loadBalance(), loadMaxSendable()])
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
          Your {{ formatAmount(+pendingBalance) }} XNT is currently in a transaction waiting in the mempool.
          You cannot send again until that transaction confirms or is dropped. This applies on this device and if you switch device.
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
                <Label for="amount">
                  Amount <span class="text-destructive">*</span>
                </Label>
                <div class="flex items-center gap-2">
                  <Badge variant="outline" class="text-xs">
                    Max: {{ formatAmount(+maxSendable) }} XNT
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
              <p v-if="showLimitInfo" class="text-[11px] text-muted-foreground">
                You can spend at most {{ formatAmount(+maxSendable) }} XNT in a single transaction.
                This limit is the sum of your 5 largest UTXOs to prevent crash when generate proof.
              </p>
              <p v-if="amountErrorMessage" class="text-xs text-destructive">
                {{ amountErrorMessage }}
              </p>
            </div>

            <div class="space-y-2">
              <Label for="fee">
                Priority Fee <span class="text-destructive">*</span>
              </Label>
              <Input
                id="fee"
                v-model="priorityFee"
                type="text"
                placeholder="0.0"
                :disabled="sendLoading"
                @blur="handleFeeBlur"
              />
              <p v-if="priorityFee && !isFeeValid" class="text-xs text-destructive">
                Invalid fee
              </p>
            </div>
          </div>

          <!-- Timelock Section -->
          <div class="space-y-3 rounded-lg border border-border bg-muted/30 p-4">
            <div class="flex items-center justify-between">
              <div class="space-y-0.5">
                <Label for="timelock" class="text-base font-medium">
                  Time Lock
                </Label>
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
                  class="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors
                         placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring
                         disabled:cursor-not-allowed disabled:opacity-50"
                />
              </div>
              <div class="space-y-2">
                <Label for="timelock-time">
                  Release Time
                </Label>
                <input
                  id="timelock-time"
                  v-model="timelockTime"
                  type="time"
                  :disabled="sendLoading"
                  class="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors
                         placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring
                         disabled:cursor-not-allowed disabled:opacity-50"
                />
              </div>
            </div>
            <p v-if="timelockErrorMessage" class="text-xs text-destructive">
              {{ timelockErrorMessage }}
            </p>
            <p v-if="enableTimelock && !timelockErrorMessage && timelockDate" class="text-xs text-muted-foreground">
              Outputs will be locked until: {{ new Date(`${timelockDate}T${timelockTime || '00:00'}`).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' }) }}
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
      :recipient-address="outputAddress"
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

