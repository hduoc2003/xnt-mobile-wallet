<script setup lang="ts">
import { ref, computed, onActivated } from 'vue'
import { useRouter } from 'vue-router'
import { toast } from 'vue-sonner'
import { ArrowLeft, ArrowRightLeft } from 'lucide-vue-next'
import { useBridgeDeposit, useBridgeFees, useFormat } from '@/composables'
import { useSettingsStore } from '@/stores'
import {
  DEFAULT_FEE,
  MIN_BRIDGE_AMOUNT,
  fromBaseUnits,
  isBridgeDepositInProgress,
  toBaseUnits,
} from '@/utils'
import ProofGenerationModal from './components/ProofGenerationModal.vue'
import BridgeDepositList from './components/BridgeDepositList.vue'

const router = useRouter()
const settingsStore = useSettingsStore()
const { deposits, refreshError, load, refresh, submitDeposit } = useBridgeDeposit()
const { fees, feesError, loadFees } = useBridgeFees()
const { formatXnt } = useFormat()
// Finished deposits are listed in History.
const inProgress = computed(() => deposits.value.filter(isBridgeDepositInProgress))

const leviathanAddress = ref('')
const amount = ref('')
const fee = ref(DEFAULT_FEE)
const submitting = ref(false)

// XNT has 8 decimals; anything else would be rounded or rejected by the backend.
const XNT_AMOUNT = /^\d+(\.\d{1,8})?$/

const amountError = computed(() => {
  if (!amount.value) return undefined
  if (!XNT_AMOUNT.test(amount.value)) return 'Enter an amount with at most 8 decimals'
  // Digits past WXNT precision would be locked but never minted.
  const decimals = fees.value?.wxntDecimals
  if (decimals !== undefined && (amount.value.split('.')[1]?.length ?? 0) > decimals) {
    return `Enter at most ${decimals} decimals (WXNT precision)`
  }
  return Number(amount.value) < MIN_BRIDGE_AMOUNT
    ? `Minimum deposit is ${MIN_BRIDGE_AMOUNT} XNT`
    : undefined
})
const feeError = computed(() =>
  XNT_AMOUNT.test(fee.value) ? undefined : 'Enter a fee with at most 8 decimals',
)
/** WXNT base units the depositor ends up with, once the amount is valid and the fee is known. */
const receivedUnits = computed(() =>
  fees.value && amount.value && !amountError.value
    ? toBaseUnits(amount.value, fees.value.wxntDecimals) - fees.value.mintFeeWxnt
    : undefined,
)
// The relayer refuses deposits that don't cover the mint fee (below_mint_fee).
const belowMintFee = computed(() => receivedUnits.value !== undefined && receivedUnits.value <= 0n)

const isFormValid = computed(
  () =>
    leviathanAddress.value.trim() !== '' &&
    amount.value !== '' &&
    fees.value !== undefined &&
    !amountError.value &&
    !feeError.value &&
    !belowMintFee.value,
)

const handleSubmit = async () => {
  if (!isFormValid.value) return
  submitting.value = true
  try {
    await submitDeposit({
      leviathanAddress: leviathanAddress.value,
      amount: amount.value,
      fee: fee.value,
    })
    toast.success('Deposit submitted', { description: 'WXNT is minted after the deposit is mined' })
    amount.value = ''
  } catch (error: unknown) {
    toast.error('Deposit failed', {
      description: error instanceof Error ? error.message : String(error),
    })
  } finally {
    submitting.value = false
  }
}

const loadError = ref<string>()

onActivated(async () => {
  void loadFees()
  try {
    await load()
    loadError.value = undefined
  } catch (error: unknown) {
    loadError.value = error instanceof Error ? error.message : String(error)
  }
  void refresh()
})
</script>

<template>
  <div class="p-4">
    <div class="mx-auto max-w-2xl space-y-6">
      <div class="flex items-center gap-3">
        <Button variant="ghost" size="icon" @click="router.push('/')">
          <ArrowLeft class="size-5" />
        </Button>
        <div>
          <h1 class="text-2xl font-bold text-foreground">Bridge to Leviathan</h1>
          <p class="text-sm text-muted-foreground">Lock XNT and receive WXNT on Leviathan</p>
        </div>
      </div>

      <Card>
        <CardHeader>
          <CardTitle class="flex items-center gap-2">
            <ArrowRightLeft class="size-5" />
            Deposit
          </CardTitle>
        </CardHeader>
        <CardContent class="space-y-4">
          <div class="space-y-2">
            <Label for="leviathan-address">
              Leviathan address <span class="text-destructive">*</span>
            </Label>
            <Textarea
              id="leviathan-address"
              v-model="leviathanAddress"
              placeholder="mtst1... or 0x account id"
              rows="2"
              class="font-mono text-sm"
              autocapitalize="none"
              autocorrect="off"
              spellcheck="false"
              :disabled="submitting"
            />
          </div>

          <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
            <div class="space-y-2">
              <Label for="bridge-amount"
                >Amount (XNT) <span class="text-destructive">*</span></Label
              >
              <Input
                id="bridge-amount"
                v-model="amount"
                type="text"
                inputmode="decimal"
                :placeholder="`${MIN_BRIDGE_AMOUNT} or more`"
                :disabled="submitting"
              />
              <p v-if="amountError" class="text-xs text-destructive">{{ amountError }}</p>
            </div>
            <div class="space-y-2">
              <Label for="bridge-fee">Fee (XNT)</Label>
              <Input
                id="bridge-fee"
                v-model="fee"
                type="text"
                inputmode="decimal"
                :disabled="submitting"
              />
              <p v-if="feeError" class="text-xs text-destructive">{{ feeError }}</p>
            </div>
          </div>

          <div class="space-y-1 rounded-md border p-3 text-xs">
            <p v-if="fees && receivedUnits !== undefined && !belowMintFee">
              You receive ~{{ fromBaseUnits(receivedUnits, fees.wxntDecimals) }} WXNT (mint fee
              {{ fromBaseUnits(fees.mintFeeWxnt, fees.wxntDecimals) }} WXNT)
            </p>
            <p v-else-if="fees && !belowMintFee" class="text-muted-foreground">
              Mint fee {{ fromBaseUnits(fees.mintFeeWxnt, fees.wxntDecimals) }} WXNT, taken from the
              minted amount
            </p>
            <p v-if="fees && belowMintFee" class="text-destructive">
              The amount must be more than the mint fee ({{
                fromBaseUnits(fees.mintFeeWxnt, fees.wxntDecimals)
              }}
              WXNT); the relayer refuses smaller deposits
            </p>
            <p v-if="feesError" class="text-destructive">
              Mint fee unavailable, so deposits are blocked: {{ feesError }}
            </p>
            <p v-if="!feeError" class="text-muted-foreground">
              Plus the XNT network fee: {{ formatXnt(fee) }} XNT
            </p>
          </div>

          <p class="text-xs text-muted-foreground">
            The deposit is proven on this device, which can take a minute. Uses at most
            {{ settingsStore.maxInputsPerTx }} UTXOs (Max inputs in Settings).
          </p>

          <Button
            size="lg"
            class="w-full"
            :disabled="!isFormValid || submitting"
            @click="handleSubmit"
          >
            Deposit
          </Button>
        </CardContent>
      </Card>

      <p v-if="loadError" class="text-xs text-destructive">
        Could not read saved deposits: {{ loadError }}
      </p>
      <BridgeDepositList :deposits="inProgress" :refresh-error="refreshError" />
    </div>

    <ProofGenerationModal :open="submitting" />
  </div>
</template>
