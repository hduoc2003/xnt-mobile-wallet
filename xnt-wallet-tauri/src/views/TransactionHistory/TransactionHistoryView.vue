<script setup lang="ts">
import { computed, ref, onMounted } from 'vue'
import { History, ArrowUpRight } from 'lucide-vue-next'
import { openUrl } from '@tauri-apps/plugin-opener'
import { useFormat, useTransaction } from '@/composables'
import { useTransactionStore } from '@/stores'

const { formatAddress, formatAmount } = useFormat()
const { refreshTransactionStatuses } = useTransaction()
const transactionStore = useTransactionStore()
const refreshing = ref(false)

const transactions = computed(() => transactionStore.getTransactions)

const formatDateTime = (iso: string) => {
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return iso
  return d.toLocaleString()
}

const openExplorer = async (utxoDigest: string) => {
  try {
    const cleanDigest = utxoDigest.startsWith('0x') ? utxoDigest.slice(2) : utxoDigest
    await openUrl(`https://neptunescan.io/utxo/${cleanDigest}`)
  } catch (error) {
    console.error('Failed to open explorer:', error)
  }
}

const statusBadgeClass = (status?: string) => {
  switch (status) {
    case 'confirmed': return 'bg-green-500/15 text-green-600 border-green-500/30'
    case 'pending':   return 'bg-amber-500/15 text-amber-600 border-amber-500/30'
    case 'failed':    return 'bg-red-500/15 text-red-600 border-red-500/30'
    default:          return 'bg-muted text-muted-foreground border-border'
  }
}

const statusLabel = (status?: string) => {
  switch (status) {
    case 'confirmed': return 'Confirmed'
    case 'pending':   return 'Pending'
    case 'failed':    return 'Failed'
    default:          return 'Unknown'
  }
}

onMounted(async () => {
  if (transactionStore.getTransactions.length) {
    refreshing.value = true
    await refreshTransactionStatuses()
    refreshing.value = false
  }
})
</script>

<template>
  <div class="p-4 h-full">
    <div class="mx-auto max-w-2xl space-y-6">
      <!-- Header -->
      <div class="flex items-center gap-3">
        <div class="rounded-full bg-primary/10 p-3">
          <History class="size-6 text-primary" />
        </div>
        <div>
          <h1 class="text-2xl font-bold text-foreground">Transaction History</h1>
          <p class="text-sm text-muted-foreground">View your recent transactions</p>
        </div>
      </div>

      <!-- Transactions list -->
      <div v-if="transactions.length === 0" class="flex flex-col items-center justify-center py-16 text-center">
        <History class="mb-4 size-12 text-muted-foreground opacity-50" />
        <p class="text-sm text-muted-foreground">No transactions yet</p>
      </div>

      <div v-else class="space-y-3">
        <Card
          v-for="tx in transactions"
          :key="tx.id"
          class="border border-border/70 bg-card transition-all hover:bg-muted/50 hover:shadow-md cursor-pointer"
          @click="openExplorer(tx.id)"
        >
          <CardContent class="flex items-center justify-between gap-3 py-3">
            <div class="flex items-start gap-3">
              <div class="mt-1 rounded-full bg-primary/10 p-2">
                <ArrowUpRight class="size-4 text-primary" />
              </div>
              <div class="space-y-1">
                <div class="flex items-center gap-2">
                  <span class="text-sm font-medium text-foreground">To</span>
                  <Badge variant="outline" class="text-[10px] uppercase tracking-wide">
                    Send
                  </Badge>
                  <span
                    class="inline-flex items-center rounded-full border px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide"
                    :class="statusBadgeClass(tx.status)"
                  >
                    {{ statusLabel(tx.status) }}
                  </span>
                </div>
                <p
                  class="max-w-[220px] truncate text-xs font-mono text-muted-foreground md:max-w-xs"
                >
                  {{ formatAddress(tx.toAddress) }}
                </p>
                <p class="text-xs text-muted-foreground">
                  {{ formatDateTime(tx.time) }}
                </p>
              </div>
            </div>

            <div class="text-right space-y-1">
              <div class="text-sm font-semibold text-foreground">
                {{ formatAmount(tx.amount) }}
                <span class="text-xs text-muted-foreground">XNT</span>
              </div>
              <div class="text-[11px] text-muted-foreground">
                Fee: {{ formatAmount(tx.fee) }} XNT
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  </div>
</template>