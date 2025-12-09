<script setup lang="ts">
import { Network, RefreshCw } from 'lucide-vue-next'

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
        <Network class="size-6 text-primary" />
      </div>
      <div>
        <h1 class="text-2xl font-bold text-foreground">Network Status</h1>
        <p class="text-sm text-muted-foreground">Real-time blockchain information</p>
      </div>
    </div>

    <Button variant="outline" size="icon" @click="$emit('refresh')" :disabled="loading">
      <RefreshCw :class="['size-4', loading && 'animate-spin']" />
    </Button>
  </div>
</template>

