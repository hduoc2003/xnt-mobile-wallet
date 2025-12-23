<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { Database, RefreshCw } from 'lucide-vue-next'
import { toast } from 'vue-sonner'
import { useNeptuneStore } from '@/stores'
import { useNeptuneWallet, useFormat } from '@/composables'
import type { Utxo } from '@/types'
import { listen } from '@tauri-apps/api/event'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Progress } from '@/components/ui/progress'
import { Label } from '@/components/ui/label'
import { Separator } from '@/components/ui/separator'
import InfiniteScrollList from '@/components/commons/infinite-scroll-list/InfiniteScrollList.vue'

const neptuneStore = useNeptuneStore()
const { getUtxos, forceRescanUtxos } = useNeptuneWallet()
const { formatHash, formatAmount } = useFormat()

const loading = ref(false)
const isScanning = ref(false)
const scanProgress = ref({
  currentBlock: 0,
  endBlock: 0,
  startBlock: 0,
  utxosFound: 0,
  percentage: 0,
  status: '',
})
const displayedUtxos = ref<Utxo[]>([])
const itemsPerPage = 20

let progressUnlisten: (() => void) | null = null

const utxosList = computed(() => [...(neptuneStore.getUtxos || [])])

const hasMore = computed(() => {
  return displayedUtxos.value?.length < utxosList.value?.length
})

const loadMoreUtxos = () => {
  if (!hasMore.value) return

  const currentLength = displayedUtxos.value.length
  const nextItems = utxosList.value.slice(currentLength, currentLength + itemsPerPage)
  displayedUtxos.value = [...displayedUtxos.value, ...nextItems]
}

watch(utxosList, (newList: Utxo[]) => {
  displayedUtxos.value = newList.slice(0, itemsPerPage)
}, { immediate: true })

const inUseUtxosCount = computed(() => utxosList.value?.length || 0)
const inUseUtxosAmount = computed(() => {
  if (!utxosList.value?.length) return '0'

  const total = utxosList.value.reduce((sum: number, utxo: any) => {
    const amount = parseFloat(utxo.amount || '0')
    if (isNaN(amount)) return sum
    return sum + amount
  }, 0)

  return total.toFixed(8)
})

const handleRefresh = async () => {
  try {
    loading.value = true
    toast.info('Starting full rescan from block 0...')
    await forceRescanUtxos()
    toast.success('Full rescan completed')
  } catch (error) {
    toast.error('Failed to force rescan UTXOs')
  } finally {
    loading.value = false
  }
}

const getUtxosData = async () => {
  try {
    loading.value = true
    isScanning.value = true
    scanProgress.value = {
      currentBlock: 0,
      endBlock: 0,
      startBlock: 0,
      utxosFound: 0,
      percentage: 0,
      status: 'Starting scan...',
    }
    await getUtxos()
  } catch (error) {
    toast.error('Failed to get UTXOs')
  } finally {
    loading.value = false
    isScanning.value = false
  }
}

onMounted(async () => {
  // Listen for UTXO scan progress events
  try {
    progressUnlisten = await listen('utxo-scan-progress', (event: any) => {
      const progress = event.payload as {
        currentBlock: number
        endBlock: number
        startBlock: number
        utxosFound: number
        percentage: number
        status: string
      }
      scanProgress.value = progress
      isScanning.value = progress.percentage < 100
      
      if (progress.percentage >= 100) {
        setTimeout(() => {
          isScanning.value = false
        }, 1000)
      }
    })
  } catch (error) {
    console.error('Failed to listen for scan progress:', error)
  }
  
  await getUtxosData()
})

onUnmounted(() => {
  if (progressUnlisten) {
    progressUnlisten()
    progressUnlisten = null
  }
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

        <Button variant="outline" size="icon" @click="handleRefresh" :disabled="loading" title="Force rescan from block 0">
          <RefreshCw :class="['size-4', loading && 'animate-spin']" />
        </Button>
      </div>

      <!-- Summary Card -->
      <Card class="border-2 border-border/50 shadow-xl">
        <CardHeader>
          <CardTitle class="text-lg font-semibold">Summary</CardTitle>
        </CardHeader>
        <CardContent>
          <div class="flex justify-between items-center gap-6">
            <div class="space-y-1">
              <p class="text-sm text-muted-foreground">Total Count</p>
              <p class="text-2xl font-bold text-primary">{{ inUseUtxosCount }}</p>
            </div>
            <div class="space-y-1">
              <p class="text-sm text-muted-foreground">Total Amount</p>
              <p class="break-all text-xl font-bold text-primary sm:text-2xl">
                {{ formatAmount(+inUseUtxosAmount, 4) }} <span class="text-lg">XNT</span>
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
        <CardContent v-if="isScanning" class="space-y-4">
          <div class="space-y-3">
            <div class="flex items-center justify-between">
              <p class="text-sm font-medium text-foreground">
                {{ scanProgress.status }}
              </p>
              <p class="text-sm font-semibold text-primary">
                {{ Math.round(scanProgress.percentage) }}%
              </p>
            </div>
            <Progress :model-value="scanProgress.percentage" class="h-2" />
            <div class="grid grid-cols-2 gap-4 text-xs text-muted-foreground">
              <div>
                <span class="font-medium">Blocks:</span> {{ scanProgress.currentBlock.toLocaleString() }} / {{ scanProgress.endBlock.toLocaleString() }}
              </div>
              <div>
                <span class="font-medium">UTXOs Found:</span> {{ scanProgress.utxosFound }}
              </div>
            </div>
          </div>
        </CardContent>
        <CardContent v-else>
          <!-- Loading State -->
          <div v-if="loading" class="flex items-center justify-center py-4">
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

          <!-- UTXO Cards -->
          <InfiniteScrollList
            v-if="!loading && utxosList.length > 0"
            :items="displayedUtxos"
            :loading="loading"
            :has-more="hasMore"
            empty-message="No UTXOs found"
            class="max-h-[550px] overflow-y-auto"
            @load-more="loadMoreUtxos"
          >
            <template #default="{ items }">
              <div class="space-y-3">
                <Card
                  v-for="(utxo, index) in items"
                  :key="index"
                  class="border border-border transition-colors hover:bg-muted/50"
                >
                  <CardContent class="space-y-3 p-4">
                    <div class="space-y-1">
                      <Label class="text-xs text-muted-foreground">UTXO Hash</Label>
                      <code class="block text-xs text-foreground">{{ formatHash(utxo.utxoHash) }}</code>
                    </div>
                    <Separator />
                    <div class="grid grid-cols-1 gap-4 sm:grid-cols-3">
                      <div class="space-y-1">
                        <Label class="text-xs text-muted-foreground">Amount</Label>
                        <p class="break-all font-mono text-sm font-semibold text-primary">
                          {{formatAmount(+utxo.amount, 4) }} <span class="text-xs">XNT</span>
                        </p>
                      </div>
                      <div class="space-y-1">
                        <Label class="text-xs text-muted-foreground">Block Height</Label>
                        <p class="text-sm font-medium text-foreground">{{ utxo.blockHeight }}</p>
                      </div>
                      <div class="space-y-1">
                        <Label class="text-xs text-muted-foreground">Time Lock</Label>
                        <p v-if="utxo.releaseDateMs" class="text-sm font-medium text-foreground">
                          Unlocks: {{ new Date(utxo.releaseDateMs).toLocaleString() }}
                        </p>
                        <p v-else class="text-sm text-muted-foreground">
                          Not timelocked
                        </p>
                      </div>
                    </div>
                  </CardContent>
                </Card>
              </div>
            </template>
          </InfiniteScrollList>
        </CardContent>
      </Card>
    </div>
  </div>
</template>
