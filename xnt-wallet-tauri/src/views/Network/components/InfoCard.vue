<script setup lang="ts">
import { Activity } from 'lucide-vue-next'
</script>

<template>
  <Card class="border border-blue-200 bg-blue-50 dark:border-blue-900 dark:bg-blue-950/30">
    <CardContent class="flex items-start gap-3 p-4">
      <div class="mt-0.5 rounded-full bg-blue-500/10 p-1.5">
        <Activity class="size-4 text-blue-600 dark:text-blue-400" />
      </div>
      <div class="flex-1 space-y-1">
        <p class="text-sm font-medium text-blue-900 dark:text-blue-100">
          Auto-Refresh
        </p>
        <p class="text-xs text-blue-700 dark:text-blue-300">
          Network data updates automatically every 60 seconds
        </p>
      </div>
    </CardContent>
  </Card>
</template>

