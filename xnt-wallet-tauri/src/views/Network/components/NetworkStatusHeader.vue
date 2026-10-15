<script setup lang="ts">
import { RefreshCw, Settings } from 'lucide-vue-next'

defineProps<{
  loading?: boolean
}>()

defineEmits<{
  refresh: []
}>()
</script>

<template>
  <div class="flex items-center justify-between">
    <div class="flex items-center gap-3">
      <div class="rounded-full bg-primary/10 p-3">
        <Settings class="size-6 text-primary" />
      </div>
      <div>
        <h1 class="text-2xl font-bold text-foreground">Settings</h1>
        <p class="text-sm text-muted-foreground">Network, RPC endpoint and transactions</p>
      </div>
    </div>

    <Button variant="outline" size="icon" :disabled="loading" @click="$emit('refresh')">
      <RefreshCw :class="['size-4', loading && 'animate-spin']" />
    </Button>
  </div>
</template>
