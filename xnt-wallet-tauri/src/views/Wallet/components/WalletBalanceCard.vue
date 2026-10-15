<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { Wallet, Copy, Check, X } from 'lucide-vue-next'
import { useNeptuneStore, useTickerStore } from '@/stores'
import { toast } from 'vue-sonner'
import { useFormat } from '@/composables'
import { useTickers } from '@/composables/useTickers'
import { POLLING_INTERVAL_XNT_PRICE } from '@/utils'

interface Props {
  availableBalance: string
  pendingBalance: string
  loading?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  loading: false,
})

const { formatAmount, formatHash, formatXnt } = useFormat()
const neptuneStore = useNeptuneStore()
const tickerStore = useTickerStore()
const isCopied = ref(false)
const showCopyModal = ref(false)

const { fetchXntPrice } = useTickers()

const usdValue = computed(() => {
  const price = tickerStore.getXntPrice ?? 0
  const balance = Number(props.availableBalance) || 0
  return balance * price
})

const change24h = computed(() => tickerStore.getXntChange24h)

const selectedType = computed({
  get: () => neptuneStore.getAddressType,
  set: (val: 'dctidh' | 'generation') => neptuneStore.setAddressType(val),
})

const dctidh_address = computed(
  () => neptuneStore.getDctidh_address || neptuneStore.getWallet?.address || '',
)
const generation_address = computed(() => neptuneStore.getGeneration_address || '')

const receiveAddress = computed(() => {
  if (selectedType.value === 'generation') return generation_address.value
  return dctidh_address.value
})

const displayAddress = computed(() => {
  if (selectedType.value === 'generation') {
    return formatHash(generation_address.value)
  }
  return dctidh_address.value
})

const handleCopyAddress = async () => {
  try {
    await navigator.clipboard.writeText(receiveAddress.value)
    isCopied.value = true
    toast.success('Address copied to clipboard')
    setTimeout(() => {
      isCopied.value = false
    }, 2000)
  } catch {
    toast.error('Failed to copy address')
  }
}

const handleCopyFromModal = async () => {
  try {
    await navigator.clipboard.writeText(receiveAddress.value)
    isCopied.value = true
    toast.success('Address copied to clipboard')
    setTimeout(() => {
      isCopied.value = false
      showCopyModal.value = false
    }, 1500)
  } catch {
    toast.error('Failed to copy address')
  }
}

let intervalId: number | null = null

onMounted(() => {
  const lastFetchTime = localStorage.getItem('xntPriceLastFetchTime')
  const cachedPrice = localStorage.getItem('xntPrice')
  // Caches from before the 24h change was stored lack it; refetch so the line is complete.
  const cachedChange = Number(localStorage.getItem('xntPriceChange24h') || NaN)
  const currentTime = Date.now()

  if (lastFetchTime && cachedPrice && Number.isFinite(cachedChange)) {
    const timeSinceLastFetch = currentTime - Number(lastFetchTime)
    if (timeSinceLastFetch < POLLING_INTERVAL_XNT_PRICE) {
      tickerStore.setXntPrice(Number(cachedPrice))
      tickerStore.setXntChange24h(cachedChange)
      const remainingTime = POLLING_INTERVAL_XNT_PRICE - timeSinceLastFetch
      setTimeout(() => {
        fetchAndSetInterval()
      }, remainingTime)
    } else {
      fetchAndSetInterval()
    }
  } else {
    fetchAndSetInterval()
  }
})

const fetchAndSetInterval = async () => {
  await fetchXntPrice()
  intervalId = window.setInterval(fetchXntPrice, POLLING_INTERVAL_XNT_PRICE)
}

onUnmounted(() => {
  if (intervalId != null) {
    clearInterval(intervalId)
    intervalId = null
  }
})
</script>

<template>
  <Card class="border-2 border-border/50 shadow-xl">
    <CardHeader class="space-y-1 pb-4">
      <CardTitle class="flex items-center gap-2 text-sm font-medium text-muted-foreground">
        <Wallet class="size-4" />
        Available Balance
      </CardTitle>
    </CardHeader>
    <CardContent class="space-y-6">
      <!-- Balance Display -->
      <div class="space-y-2">
        <div v-if="loading" class="flex items-center justify-center py-8">
          <div
            class="size-8 animate-spin rounded-full border-4 border-primary border-t-transparent"
          />
        </div>
        <div v-else class="space-y-1">
          <div class="flex items-baseline gap-2">
            <span class="text-4xl font-bold text-foreground">
              {{ formatXnt(availableBalance) }}
            </span>
            <span class="text-xl font-medium text-muted-foreground">XNT</span>
            <span v-if="usdValue" class="text-lg font-medium text-muted-foreground">
              = ${{ formatAmount(usdValue, 2) }}
            </span>
          </div>
          <div v-if="tickerStore.getXntPrice" class="flex items-center gap-2 text-sm">
            <span class="text-muted-foreground">1 XNT =</span>
            <span class="font-medium text-foreground"
              >${{ formatAmount(tickerStore.getXntPrice, 4) }}</span
            >
            <span
              v-if="change24h !== null"
              :class="change24h >= 0 ? 'text-green-600' : 'text-red-600'"
            >
              {{ change24h >= 0 ? '+' : '' }}{{ change24h.toFixed(2) }}% 24h
            </span>
          </div>
          <div v-if="pendingBalance" class="flex items-center gap-2 text-sm">
            <span class="text-muted-foreground">Pending:</span>
            <span class="font-medium text-foreground">{{ formatXnt(pendingBalance) }} XNT</span>
          </div>
        </div>
      </div>

      <Separator />

      <!-- Receiving Address -->
      <div class="space-y-3">
        <div class="flex items-center justify-between">
          <Label class="text-sm font-medium">Receiving Address</Label>
          <!-- Address type selector -->
          <div class="flex rounded-md border border-border overflow-hidden text-xs">
            <button
              :class="[
                'px-2 py-1 transition-colors',
                selectedType === 'dctidh'
                  ? 'bg-primary text-primary-foreground'
                  : 'bg-transparent text-muted-foreground hover:bg-muted',
              ]"
              @click="selectedType = 'dctidh'"
            >
              dCTIDH
            </button>
            <button
              :class="[
                'px-2 py-1 transition-colors',
                selectedType === 'generation'
                  ? 'bg-primary text-primary-foreground'
                  : 'bg-transparent text-muted-foreground hover:bg-muted',
              ]"
              :disabled="!generation_address"
              @click="selectedType = 'generation'"
            >
              Generation
            </button>
          </div>
        </div>

        <!-- Clickable address box -->
        <div
          class="flex items-center gap-2 rounded-lg border border-border bg-muted/50 p-3 transition-colors hover:bg-muted cursor-pointer"
          @click="showCopyModal = true"
        >
          <code
            class="flex-1 overflow-hidden text-ellipsis whitespace-nowrap text-xs text-foreground md:text-sm"
          >
            {{ displayAddress }}
          </code>
          <Button
            variant="ghost"
            size="icon"
            class="size-8 shrink-0"
            @click.stop="handleCopyAddress"
          >
            <Check v-if="isCopied" class="size-4 text-green-500" />
            <Copy v-else class="size-4" />
          </Button>
        </div>
      </div>
    </CardContent>
  </Card>

  <!-- Copy Address Modal -->
  <Teleport to="body">
    <Transition name="fade">
      <div
        v-if="showCopyModal"
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
        @click.self="showCopyModal = false"
      >
        <div class="w-full max-w-md rounded-xl border border-border bg-background shadow-2xl">
          <div class="flex items-center justify-between border-b border-border px-4 py-3">
            <span class="text-sm font-semibold">
              {{ selectedType === 'dctidh' ? 'dCTIDH' : 'Generation' }} Receiving Address
            </span>
            <Button variant="ghost" size="icon" class="size-7" @click="showCopyModal = false">
              <X class="size-4" />
            </Button>
          </div>
          <div class="p-4 space-y-4">
            <div class="rounded-lg border border-border bg-muted/50 p-3">
              <p class="break-all text-xs text-foreground font-mono leading-relaxed">
                {{ receiveAddress }}
              </p>
            </div>
            <Button
              class="w-full"
              :variant="isCopied ? 'outline' : 'default'"
              @click="handleCopyFromModal"
            >
              <Check v-if="isCopied" class="mr-2 size-4 text-green-500" />
              <Copy v-else class="mr-2 size-4" />
              {{ isCopied ? 'Copied!' : 'Copy Address' }}
            </Button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.15s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
