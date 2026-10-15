<script setup lang="ts">
import { CopyButton, ExplorerLink } from '@/components/commons'
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { toast } from 'vue-sonner'
import { ArrowLeft, GitMerge } from 'lucide-vue-next'
import { useNeptuneWallet, useFormat, useTransaction } from '@/composables'
import { useNeptuneStore } from '@/stores'
import { DEFAULT_FEE } from '@/utils'
import ConfirmTransactionDrawer from './components/ConfirmTransactionDrawer.vue'
import ProofGenerationModal from './components/ProofGenerationModal.vue'

import type { Utxo } from '@/types'

const router = useRouter()
const store = useNeptuneStore()
const { getUtxos, buildTransaction, broadcastSignedTransaction } = useNeptuneWallet()
const { addTransaction } = useTransaction()
const { formatXnt } = useFormat()

const MAX_SELECT = 5
const loading = ref(false)
const sendLoading = ref(false)
const showConfirmModal = ref(false)
const showProofModal = ref(false)
const fee = ref(DEFAULT_FEE)
const allUtxos = ref<Utxo[]>([])
const selectedUtxos = ref<Utxo[]>([])
const now = Date.now()

const isUtxoTimelocked = (utxo: Utxo) => {
  if (!utxo.isTimelocked) return false
  const releaseMs = utxo.releaseDateMs ?? null
  if (releaseMs === null) return true
  return releaseMs > now
}

// Use additionRecord as the unique key.
const utxoKey = (utxo: Utxo) => utxo.additionRecord

const isSelected = (utxo: Utxo) => selectedUtxos.value.some((u) => utxoKey(u) === utxoKey(utxo))

const isDisabled = (utxo: Utxo) => {
  if (isUtxoTimelocked(utxo)) return true
  if (utxo.isPending) return true
  if (isSelected(utxo)) return false
  return selectedUtxos.value.length >= MAX_SELECT
}

const toggleUtxo = (utxo: Utxo) => {
  if (isDisabled(utxo) && !isSelected(utxo)) return
  if (isSelected(utxo)) {
    selectedUtxos.value = selectedUtxos.value.filter((u) => utxoKey(u) !== utxoKey(utxo))
  } else {
    selectedUtxos.value = [...selectedUtxos.value, utxo]
  }
}

const totalSelected = computed(() =>
  selectedUtxos.value.reduce((s, u) => s + parseFloat(u.amount || '0'), 0),
)

const netAmount = computed(() => totalSelected.value - parseFloat(fee.value || '0'))

// Per-input spending key hex — one entry per selected UTXO, matched by key type.
// The backend accepts this array to unlock mixed-key inputs (Generation + dCTIDH).
// NOTE: store.getSpendingKey = Generation key; store.getGenerationSpendingKey = dCTIDH key.
const spendingKeyHexes = computed(() =>
  selectedUtxos.value.map((u) =>
    u.keyType === 'Generation'
      ? (store.getSpendingKey ?? store.getGenerationSpendingKey ?? '')
      : (store.getGenerationSpendingKey ?? store.getSpendingKey ?? ''),
  ),
)

// Primary key for the required spendingKeyHex field (used only for change address
// detection; change is always 0 on a merge so this key is never actually applied).
// Match the first selected UTXO's type so the backend log is meaningful.
const primarySpendingKeyHex = computed(() => {
  const firstKeyType = selectedUtxos.value[0]?.keyType || 'dCTIDH'
  return firstKeyType === 'Generation'
    ? (store.getSpendingKey ?? store.getGenerationSpendingKey)
    : (store.getGenerationSpendingKey ?? store.getSpendingKey)
})

// Merge output always goes to the dCTIDH address.
const ownAddress = computed(() => store.getDctidh_address ?? store.getAddress)

const isFeeValid = computed(() => {
  if (!fee.value) return false
  const num = parseFloat(fee.value)
  return !isNaN(num) && num >= 0
})

const validationError = computed(() => {
  if (selectedUtxos.value.length === 0) return 'Select at least 1 UTXO'
  if (!isFeeValid.value) return ''
  if (netAmount.value <= 0) return 'Net amount must be greater than 0'
  return ''
})

const isFormValid = computed(
  () =>
    selectedUtxos.value.length >= 1 &&
    isFeeValid.value &&
    netAmount.value > 0 &&
    !sendLoading.value,
)

const formatDecimal = (value: string) => {
  if (!value) return ''
  let cleaned = value.replace(/[^\d.]/g, '')
  const parts = cleaned.split('.')
  if (parts.length > 2) cleaned = parts[0] + '.' + parts.slice(1).join('')
  if (cleaned.includes('.')) {
    const [integer, decimal = ''] = cleaned.split('.')
    cleaned = integer + '.' + decimal.slice(0, 8)
  }
  return cleaned
}

const handleFeeBlur = () => {
  if (fee.value) fee.value = formatDecimal(fee.value)
}

const handleShowConfirm = () => {
  if (!isFormValid.value) return
  showConfirmModal.value = true
}

const handleCancelConfirm = () => {
  showConfirmModal.value = false
}

const handleConfirmMerge = async () => {
  try {
    sendLoading.value = true
    showConfirmModal.value = false
    showProofModal.value = true

    const feeNum = parseFloat(fee.value)
    if (isNaN(feeNum)) {
      toast.error('Invalid fee')
      return
    }

    const address = ownAddress.value
    if (!address) {
      toast.error('Could not determine own address')
      return
    }

    const inputAdditionRecords = selectedUtxos.value.map(
      (utxo) =>
        (utxo as any).addition_record_hex || utxo.additionRecord || (utxo as any).addition_record,
    )

    const netStr = netAmount.value.toFixed(8)

    const txResult = await buildTransaction({
      inputAdditionRecords,
      outputAddresses: [address],
      outputAmounts: [netStr],
      fee: fee.value,
      // Primary key used for change address (change will be 0 for a merge, but required).
      spendingKeyHex: primarySpendingKeyHex.value ?? undefined,
      // Per-input keys let the backend unlock Generation and dCTIDH inputs in the same tx.
      spendingKeyHexes: spendingKeyHexes.value,
    })

    if (!txResult.success) {
      toast.error('Failed to build transaction', { description: txResult.error || '' })
      return
    }

    const outputUtxoDigests = txResult.outputUtxoDigests || []
    if (!outputUtxoDigests.length) {
      toast.error('Failed to get output UTXO digests from transaction')
      return
    }

    const submitResult = await broadcastSignedTransaction(txResult.transaction)
    if (!submitResult?.success) {
      toast.error(submitResult?.error || 'Failed to broadcast transaction')
      return
    }

    const firstDigest = outputUtxoDigests[0]
    if (!firstDigest || typeof firstDigest !== 'string') {
      toast.error('Transaction submitted but no UTXO digest available')
      return
    }

    addTransaction({
      id: firstDigest,
      txId: txResult.txId,
      outputCommitments: outputUtxoDigests,
      toAddress: address,
      amount: parseFloat(netStr),
      fee: feeNum,
      time: new Date().toISOString(),
    })

    toast.success('UTXOs merged successfully!')
    router.push('/')
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Failed to merge UTXOs'
    toast.error('Merge Failed', { description: message })
  } finally {
    sendLoading.value = false
    showProofModal.value = false
  }
}

onMounted(async () => {
  try {
    loading.value = true
    const result = await getUtxos()
    const utxos = result?.result?.utxos || result?.utxos || store.getUtxos || []
    allUtxos.value = (Array.isArray(utxos) ? utxos : []).filter(
      (u: any) => !(u.isPending ?? u.is_pending),
    )
  } catch (error) {
    toast.error('Failed to load UTXOs')
  } finally {
    loading.value = false
  }
})
</script>

<template>
  <div class="p-4">
    <div class="mx-auto max-w-2xl space-y-6">
      <!-- Header -->
      <div class="flex items-center gap-3">
        <Button variant="ghost" size="icon" @click="router.push('/utxo')">
          <ArrowLeft class="size-5" />
        </Button>
        <div>
          <h1 class="text-2xl font-bold text-foreground">Merge UTXOs</h1>
          <p class="text-sm text-muted-foreground">Consolidate selected UTXOs into one</p>
        </div>
      </div>

      <!-- Fee Input -->
      <Card>
        <CardHeader>
          <CardTitle class="flex items-center gap-2">
            <GitMerge class="size-5" />
            Merge Settings
          </CardTitle>
        </CardHeader>
        <CardContent class="space-y-4">
          <div class="space-y-2">
            <Label for="fee"> Priority Fee <span class="text-destructive">*</span> </Label>
            <Input
              id="fee"
              v-model="fee"
              type="text"
              placeholder="0.0"
              :disabled="sendLoading"
              @blur="handleFeeBlur"
            />
            <p v-if="fee && !isFeeValid" class="text-xs text-destructive">Invalid fee</p>
          </div>
        </CardContent>
      </Card>

      <!-- Selection Summary -->
      <Card>
        <CardContent class="pt-4 space-y-2">
          <div class="flex flex-wrap gap-3 text-sm">
            <Badge variant="outline">{{ selectedUtxos.length }} / {{ MAX_SELECT }} selected</Badge>
            <Badge variant="outline">Total: {{ formatXnt(totalSelected) }} XNT</Badge>
            <Badge
              :variant="
                !selectedUtxos.length || (isFeeValid && netAmount > 0) ? 'outline' : 'default'
              "
            >
              After fee: {{ selectedUtxos.length && isFeeValid ? formatXnt(netAmount) : '-' }} XNT
            </Badge>
            <Badge v-if="selectedUtxos.length > 1" variant="secondary" class="text-xs">
              Mixed keys supported
            </Badge>
          </div>
          <p v-if="validationError" class="text-xs text-primary">{{ validationError }}</p>
        </CardContent>
      </Card>

      <!-- UTXO Selection List -->
      <Card class="border border-border">
        <CardHeader>
          <CardTitle class="text-base">Select UTXOs</CardTitle>
        </CardHeader>
        <CardContent>
          <!-- Loading -->
          <div v-if="loading" class="flex items-center justify-center py-8">
            <div
              class="size-8 animate-spin rounded-full border-4 border-primary border-t-transparent"
            />
          </div>

          <!-- Empty -->
          <div
            v-else-if="allUtxos.length === 0"
            class="flex flex-col items-center justify-center py-12 text-center"
          >
            <GitMerge class="mb-4 size-12 text-muted-foreground opacity-50" />
            <p class="text-sm text-muted-foreground">No UTXOs found</p>
          </div>

          <!-- List — rendered flat so the page (not an inner scroll box) scrolls -->
          <div v-else class="space-y-3">
            <div
              v-for="utxo in allUtxos"
              :key="utxoKey(utxo)"
              class="flex items-start gap-3 rounded-lg border border-border p-3 transition-colors"
              :class="{
                'border-primary bg-primary/5': isSelected(utxo),
                'opacity-50 cursor-not-allowed': isDisabled(utxo),
                'cursor-pointer hover:bg-muted/50': !isDisabled(utxo),
              }"
              @click="!isDisabled(utxo) && toggleUtxo(utxo)"
            >
              <input
                type="checkbox"
                class="mt-1 size-4 accent-primary flex-shrink-0"
                :checked="isSelected(utxo)"
                :disabled="isDisabled(utxo)"
                @change.stop="toggleUtxo(utxo)"
                @click.stop
              />
              <div class="min-w-0 flex-1 space-y-1">
                <div class="flex items-center gap-2 flex-wrap">
                  <code class="break-all font-mono text-xs text-foreground">
                    {{ (utxo as any).utxoDigest }}
                    <template v-if="utxo.utxoDigest">
                      <CopyButton :value="utxo.utxoDigest" label="UTXO digest" />
                      <ExplorerLink :path="`/utxo/${utxo.utxoDigest}`" />
                    </template>
                  </code>
                  <Badge variant="secondary" class="text-xs">{{ utxo.keyType || 'dCTIDH' }}</Badge>
                  <Badge v-if="isUtxoTimelocked(utxo)" variant="destructive" class="text-xs">
                    Locked
                  </Badge>
                </div>
                <div class="flex gap-4 text-xs text-muted-foreground">
                  <span class="font-mono font-semibold text-primary">
                    {{ formatXnt(utxo.amount) }} XNT
                  </span>
                  <span>Block {{ utxo.blockHeight }}</span>
                  <span v-if="isUtxoTimelocked(utxo) && utxo.releaseDateMs">
                    Unlocks: {{ new Date(utxo.releaseDateMs).toLocaleDateString() }}
                  </span>
                </div>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      <!-- Confirm Button -->
      <Button
        class="w-full gap-2"
        :disabled="!isFormValid || sendLoading"
        @click="handleShowConfirm"
      >
        <GitMerge class="size-4" />
        {{ sendLoading ? 'Merging...' : 'Confirm Merge' }}
      </Button>
    </div>

    <!-- Confirm Drawer -->
    <ConfirmTransactionDrawer
      v-model:open="showConfirmModal"
      :recipient-address="ownAddress || ''"
      :amount="isFeeValid && netAmount > 0 ? netAmount.toFixed(8) : '0'"
      :fee="fee"
      :loading="sendLoading"
      @confirm="handleConfirmMerge"
      @cancel="handleCancelConfirm"
    />

    <!-- Proof Modal -->
    <ProofGenerationModal v-model:open="showProofModal" />
  </div>
</template>
