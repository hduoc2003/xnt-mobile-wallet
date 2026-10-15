<script setup lang="ts">
import { CopyButton, ExplorerLink } from '@/components/commons'
import type { BridgeDepositRecord } from '@/types'
import { bridgeDepositBadge as badge, bridgeDepositDetail as detail, formatXnt } from '@/utils'

defineProps<{
  deposits: BridgeDepositRecord[]
  refreshError?: string
}>()

const shortId = (hex: string) => `${hex.slice(0, 10)}…${hex.slice(-6)}`
</script>

<template>
  <Card>
    <CardHeader>
      <CardTitle>Deposits in progress</CardTitle>
    </CardHeader>
    <CardContent class="space-y-3">
      <p v-if="refreshError" class="text-xs text-destructive">
        Status refresh failed: {{ refreshError }}
      </p>
      <p v-if="deposits.length === 0" class="text-sm text-muted-foreground">
        No deposits in progress; finished ones are in History
      </p>
      <div
        v-for="record in deposits"
        :key="record.utxoCommitment"
        class="space-y-1 rounded-lg border p-3"
      >
        <div class="flex items-center justify-between gap-2">
          <span class="font-semibold">{{ formatXnt(record.amount) }} XNT</span>
          <Badge :variant="badge(record).variant">{{ badge(record).label }}</Badge>
        </div>
        <p class="text-xs text-muted-foreground">{{ detail(record) }}</p>
        <p class="font-mono text-[10px] text-muted-foreground" :title="record.utxoCommitment">
          {{ shortId(record.utxoCommitment) }}
          <CopyButton :value="record.utxoCommitment" label="Commitment" />
          <ExplorerLink :path="`/utxo/${record.utxoCommitment}`" />
          · {{ new Date(record.createdMs).toLocaleString() }}
        </p>
      </div>
    </CardContent>
  </Card>
</template>
