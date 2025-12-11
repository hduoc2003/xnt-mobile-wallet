<script setup lang="ts">
import { computed, ref, watch, onMounted } from 'vue'
import { History, ArrowUpRight } from 'lucide-vue-next'
import { openUrl } from '@tauri-apps/plugin-opener'
import { useFormat } from '@/composables'
import { useTransactionStore } from '@/stores'
import { PER_PAGE } from '@/utils'
import type { TransactionDetail } from '@/types'

const { formatAddress, formatAmount } = useFormat()
const transactionStore = useTransactionStore()
const loading = ref(false)

const transactions = computed(() => transactionStore.getTransactions)
const displayedTransactions = ref<TransactionDetail[]>([])

const hasMore = computed(() => {
  return displayedTransactions.value?.length < transactions.value?.length
})

const loadMoreTransactions = () => {
  loading.value = true
  if (!hasMore.value) {
    loading.value = false
    return
  }

  const currentLength = displayedTransactions.value.length
  const nextItems = transactions.value.slice(currentLength, currentLength + PER_PAGE)
  displayedTransactions.value = [...displayedTransactions.value, ...nextItems]
  loading.value = false
}

watch(transactions, (newTransactions) => {
  displayedTransactions.value = newTransactions.slice(0, PER_PAGE)
}, { immediate: true })

const formatDateTime = (iso: string) => {
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return iso
  return d.toLocaleString()
}

const openExplorer = async (txid: string) => {
  try {
    const cleanTxid = txid.startsWith('0x') ? txid.slice(2) : txid
    await openUrl(`http://neptunescan.io/tx?id=${cleanTxid}`)
  } catch (error) {
    console.error('Failed to open explorer:', error)
  }
}
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
      <InfiniteScrollList
        :items="displayedTransactions"
        :loading="loading"
        :has-more="hasMore"
        empty-message="No transactions yet"
        class="max-h-[550px] overflow-y-auto"
        @load-more="loadMoreTransactions"
      >
        <template #default="{ items }">
          <div class="space-y-3">
            <Card
              v-for="tx in items"
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
        </template>
      </InfiniteScrollList>
    </div>
  </div>
</template>