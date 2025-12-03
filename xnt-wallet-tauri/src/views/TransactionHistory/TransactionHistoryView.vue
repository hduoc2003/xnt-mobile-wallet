<script setup lang="ts">
import { computed } from 'vue'
import { History, ArrowUpRight } from 'lucide-vue-next'
import { openUrl } from '@tauri-apps/plugin-opener'
import { useFormat } from '@/composables'
import type { TransactionDetail } from '@/types'

const { formatAddress, formatAmount } = useFormat()

	// Mock data – replace with real API later
const mockTransactions: TransactionDetail[] = [
  {
    id: 'c76b15df5f5b5b8567dd3eb38ee734e1bebd1f85e451c09d1de699531aea5eabaa6ed602cae15c87',
    toAddress: 'xnt1qqp9c0u9k9w0x3z6r4g2d7f8s2a9l0m5n7v3y8k',
    amount: 12.3456789,
    fee: 0.001,
    time: new Date(Date.now() - 5 * 60 * 1000).toISOString(), // 5 minutes ago
  },
  {
    id: 'c76b15df5f5b5b8567dd3eb38ee734e1bebd1f85e451c09d1de699531aea5eabaa6ed602cae15c87',
    toAddress: 'xnt1qqp3k9u2m4n5b7v8c9x1z2a3s4d5f6g7h8j9k0',
    amount: 0.5,
    fee: 0.0005,
    time: new Date(Date.now() - 2 * 60 * 60 * 1000).toISOString(), // 2 hours ago
  },
  {
    id: 'c76b15df5f5b5b8567dd3eb38ee734e1bebd1f85e451c09d1de699531aea5eabaa6ed602cae15c87',
    toAddress: 'xnt1qqp7h6g5f4d3s2a1z9x8c7v6b5n4m3k2j1h0g9',
    amount: 100,
    fee: 0.002,
    time: new Date(Date.now() - 24 * 60 * 60 * 1000).toISOString(), // 1 day ago
  },
]

const hasTransactions = computed(() => mockTransactions.length > 0)

const formatDateTime = (iso: string) => {
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return iso
  return d.toLocaleString()
}

const openExplorer = async (txid: string) => {
  try {
    await openUrl(`http://neptunescan.io/tx?id=${txid}`)
  } catch (error) {
    console.error('Failed to open explorer:', error)
  }
}
</script>

<template>
  <div class="p-4">
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

      <!-- Empty state -->
      <div v-if="!hasTransactions" class="rounded-lg border border-border bg-card p-6 text-center">
        <p class="text-muted-foreground">No transactions yet</p>
      </div>

      <!-- Transactions list -->
      <div v-else class="space-y-3">
        <Card
          v-for="tx in mockTransactions"
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
    </div>
  </div>
</template>