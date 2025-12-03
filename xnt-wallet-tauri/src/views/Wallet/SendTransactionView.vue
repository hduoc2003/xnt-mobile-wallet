<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { toast } from 'vue-sonner'
import { Send, ArrowLeft } from 'lucide-vue-next'
import { useNeptuneWallet } from '@/composables'

const router = useRouter()
const { getBalance, getUtxos, buildTransaction, broadcastSignedTransaction } = useNeptuneWallet()

const availableBalance = ref('0')
const loading = ref(false)
const sendLoading = ref(false)
const showConfirmModal = ref(false)

const outputAddress = ref('')
const outputAmount = ref('')
const priorityFee = ref('')

const loadBalance = async () => {
  try {
    loading.value = true
    const result = await getBalance()
    availableBalance.value = result?.balance ?? '0'
  } catch (error) {
    toast.error('Failed to load balance')
  } finally {
    loading.value = false
  }
}

const isAddressValid = computed(() => outputAddress.value.trim().length > 0)
const isAmountValid = computed(() => {
  if (!outputAmount.value) return false
  const num = parseFloat(outputAmount.value)
  if (isNaN(num) || num <= 0) return false

  const balance = parseFloat(availableBalance.value)
  if (!isNaN(balance) && num > balance) return false

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

  return ''
})

const isFormValid = computed(
  () => isAddressValid.value && isAmountValid.value && isFeeValid.value && !sendLoading.value
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

    // Get UTXOs
    const utxosResult = await getUtxos()
    const utxos = utxosResult?.utxos || []

    if (!utxos.length) {
      toast.error('No UTXOs available')
      return
    }

    // Build transaction
    const inputAdditionRecords = utxos.map((utxo: any) => utxo.additionRecord || utxo.addition_record)

    const txResult = await buildTransaction({
      inputAdditionRecords,
      outputAddresses: [outputAddress.value.trim()],
      outputAmounts: [outputAmount.value],
      fee: priorityFee.value,
    })

    if (!txResult.success) {
      toast.error('Failed to build transaction')
      return
    }

    // Broadcast transaction
    await broadcastSignedTransaction(txResult.transaction)

    toast.success('Transaction sent successfully!')

    // Navigate back to wallet
    router.push('/')
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Failed to send transaction'
    toast.error('Transaction Failed', { description: message })
  } finally {
    sendLoading.value = false
  }
}

onMounted(() => {
  loadBalance()
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
                <Badge variant="secondary" class="text-xs">
                  Available: {{ availableBalance }} XNT
                </Badge>
              </div>
              <Input
                id="amount"
                v-model="outputAmount"
                type="text"
                placeholder="0.0"
                :disabled="sendLoading"
                @blur="handleAmountBlur"
              />
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

    <!-- Confirm Transaction Modal -->
    <Dialog v-model:open="showConfirmModal">
      <DialogContent class="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle>Confirm Transaction</DialogTitle>
          <DialogDescription>Please review the transaction details before sending</DialogDescription>
        </DialogHeader>

        <div class="space-y-4 py-4">
          <!-- Recipient -->
          <div class="space-y-2">
            <Label class="text-sm text-muted-foreground">Recipient Address</Label>
            <div class="break-all rounded-lg border border-border bg-muted/50 p-3 font-mono text-sm">
              {{ outputAddress }}
            </div>
          </div>

          <!-- Amount and Fee -->
          <div class="grid grid-cols-2 gap-4">
            <div class="space-y-2">
              <Label class="text-sm text-muted-foreground">Amount</Label>
              <div class="rounded-lg border border-border bg-muted/50 p-3 font-semibold">
                {{ outputAmount }} XNT
              </div>
            </div>
            <div class="space-y-2">
              <Label class="text-sm text-muted-foreground">Priority Fee</Label>
              <div class="rounded-lg border border-border bg-muted/50 p-3 font-semibold">
                {{ priorityFee }} XNT
              </div>
            </div>
          </div>

          <Alert>
            <AlertDescription>
              This action cannot be undone. Make sure all details are correct before proceeding.
            </AlertDescription>
          </Alert>
        </div>

        <DialogFooter class="gap-2">
          <Button variant="outline" @click="handleCancelConfirm" :disabled="sendLoading">
            Cancel
          </Button>
          <Button @click="handleConfirmSend" :disabled="sendLoading">
            <span
              v-if="sendLoading"
              class="mr-2 size-4 animate-spin rounded-full border-2 border-white border-t-transparent"
            />
            {{ sendLoading ? 'Sending...' : 'Confirm & Send' }}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  </div>
</template>

