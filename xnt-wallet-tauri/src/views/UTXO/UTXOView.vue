<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { Database, RefreshCw } from 'lucide-vue-next'
import { toast } from 'vue-sonner'
import { useNeptuneStore } from '@/stores/neptuneStore'
import { useNeptuneWallet, useFormat } from '@/composables'

const neptuneStore = useNeptuneStore()
const { getUtxos } = useNeptuneWallet()
const { formatHash, formatAmount } = useFormat()

const loading = ref(false)

const utxosList = computed(() => [...(neptuneStore.getUtxos || [])])

const inUseUtxosCount = computed(() => utxosList.value?.length || 0)
const inUseUtxosAmount = computed(() => {
  if (!utxosList.value?.length) return '0'

  const total = utxosList.value.reduce((sum: number, utxo: any) => {
    const amount = parseFloat(utxo.amount || 0)
    return sum + amount
  }, 0)

  return total.toFixed(8)
})

const handleRefresh = async () => {
  await getUtxosData()
}

const getUtxosData = async () => {
  try {
    loading.value = true
    await getUtxos()
  } catch (error) {
    toast.error('Failed to get UTXOs')
  } finally {
    loading.value = false
  }
}

onMounted(async () => {
  await getUtxosData()
})
</script>

<template>
  <div class="p-4">
    <div class="mx-auto max-w-4xl space-y-6">
      <!-- Page Title -->
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-3">
          <div class="rounded-full bg-primary/10 p-3">
            <Database class="size-6 text-primary" />
          </div>
          <div>
            <h1 class="text-2xl font-bold text-foreground">UTXOs</h1>
            <p class="text-sm text-muted-foreground">Unspent transaction outputs</p>
          </div>
        </div>

        <Button variant="outline" size="icon" @click="handleRefresh" :disabled="loading">
          <RefreshCw :class="['size-4', loading && 'animate-spin']" />
        </Button>
      </div>

      <!-- Summary Card -->
      <Card class="border-2 border-border/50 shadow-xl">
        <CardHeader>
          <CardTitle class="text-lg font-semibold">Summary</CardTitle>
        </CardHeader>
        <CardContent>
          <div class="grid grid-cols-1 gap-6 sm:grid-cols-2">
            <div class="space-y-1">
              <p class="text-sm text-muted-foreground">Total Count</p>
              <p class="text-2xl font-bold text-primary">{{ inUseUtxosCount }}</p>
            </div>
            <div class="space-y-1">
              <p class="text-sm text-muted-foreground">Total Amount</p>
              <p class="break-all text-xl font-bold text-primary sm:text-2xl">
                {{ formatAmount(+inUseUtxosAmount) }} <span class="text-lg">XNT</span>
              </p>
            </div>
          </div>
        </CardContent>
      </Card>

      <!-- UTXOs List -->
      <Card class="border border-border">
        <CardHeader>
          <CardTitle class="text-lg font-semibold">UTXO List</CardTitle>
        </CardHeader>
        <CardContent>
          <!-- Loading State -->
          <div v-if="loading" class="flex items-center justify-center py-12">
            <div
              class="size-8 animate-spin rounded-full border-4 border-primary border-t-transparent"
            />
          </div>

          <!-- Empty State -->
          <div
            v-else-if="!utxosList.length"
            class="flex flex-col items-center justify-center py-12 text-center"
          >
            <Database class="mb-4 size-12 text-muted-foreground opacity-50" />
            <p class="text-sm text-muted-foreground">No UTXOs found</p>
          </div>

          <!-- UTXO Cards (Mobile) -->
          <div class="space-y-3 md:hidden">
            <Card
              v-for="(utxo, index) in utxosList"
              :key="index"
              class="border border-border transition-colors hover:bg-muted/50"
            >
              <CardContent class="space-y-3 p-4">
                <div class="space-y-1">
                  <Label class="text-xs text-muted-foreground">UTXO Hash</Label>
                  <code class="block text-xs text-foreground">{{ formatHash(utxo.utxoHash) }}</code>
                </div>
                <Separator />
                <div class="grid grid-cols-2 gap-4">
                  <div class="space-y-1">
                    <Label class="text-xs text-muted-foreground">Amount</Label>
                    <p class="break-all font-mono text-sm font-semibold text-primary">
                      {{ utxo.amount }} <span class="text-xs">XNT</span>
                    </p>
                  </div>
                  <div class="space-y-1">
                    <Label class="text-xs text-muted-foreground">Block Height</Label>
                    <p class="text-sm font-medium text-foreground">{{ utxo.blockHeight }}</p>
                  </div>
                </div>
              </CardContent>
            </Card>
          </div>
        </CardContent>
      </Card>
    </div>
  </div>
</template>
