<script setup lang="ts">
import { Activity } from 'lucide-vue-next'

defineProps<{
  network: string
  blockHeight: number
  lastUpdate: Date | null
}>()

const formatNumber = (num: number) => {
  return new Intl.NumberFormat('en-US').format(num)
}

const formatTime = (date: Date | null) => {
  if (!date) return 'N/A'
  return date.toLocaleTimeString('en-US', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  })
}
</script>

<template>
  <Card class="border-2 border-border/50 shadow-xl">
    <CardHeader class="space-y-1">
      <CardTitle class="flex items-center gap-2 text-lg">
        <Activity class="size-5 text-primary" />
        Network Information
      </CardTitle>
    </CardHeader>
    <CardContent class="space-y-4">
      <!-- Network Name -->
      <div
        class="flex items-center justify-between rounded-lg border border-border bg-muted/50 p-4 transition-colors hover:bg-muted"
      >
        <div class="space-y-1">
          <Label class="text-xs text-muted-foreground">Network</Label>
          <p class="text-sm font-semibold text-foreground">
            XNT <span class="capitalize text-primary">{{ network }}</span>
          </p>
        </div>
        <div class="flex items-center gap-2">
          <div class="size-2 animate-pulse rounded-full bg-green-500" />
          <span class="text-xs font-medium text-green-600 dark:text-green-400">Live</span>
        </div>
      </div>

      <Separator />

      <!-- Block Height -->
      <div class="space-y-3">
        <Label class="text-xs text-muted-foreground">Current Block Height</Label>
        <div
          class="rounded-lg border border-border bg-linear-to-br from-primary/5 to-primary/10 p-6 text-center"
        >
          <div class="flex items-baseline justify-center gap-2">
            <span class="text-4xl font-bold text-primary">{{ formatNumber(blockHeight) }}</span>
          </div>
        </div>
      </div>

      <Separator />

      <!-- Last Update -->
      <div class="flex items-center justify-between rounded-lg bg-muted/30 p-3">
        <Label class="text-xs text-muted-foreground">Last Updated</Label>
        <span class="font-mono text-sm font-medium text-foreground">
          {{ formatTime(lastUpdate) }}
        </span>
      </div>
    </CardContent>
  </Card>
</template>
