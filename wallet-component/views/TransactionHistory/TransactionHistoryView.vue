<script setup lang="ts">
import { History } from 'lucide-vue-next'

const { t } = useI18n()
</script>

<template>
  <div class="container mx-auto px-4 py-6">
    <div class="flex items-center gap-3">
      <div class="flex items-center gap-3">
        <div class="rounded-full bg-primary/10 p-3">
          <History class="size-6 text-primary" />
        </div>
        <div>
          <h1 class="text-2xl font-bold text-foreground">Transaction History</h1>
          <p class="text-sm text-muted-foreground">View your transaction history</p>
        </div>
      </div>
    </div>

    <!-- History content will go here -->
    <div class="mt-6">
      <div class="rounded-lg border border-border bg-card p-6 text-center">
        <p class="text-muted-foreground">No transactions yet</p>
      </div>
    </div>
  </div>
</template>
