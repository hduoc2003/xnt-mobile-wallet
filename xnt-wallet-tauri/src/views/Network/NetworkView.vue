<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { Network, Activity, RefreshCw, AlertCircle } from 'lucide-vue-next'
import { toast } from 'vue-sonner'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Label } from '@/components/ui/label'
import { Separator } from '@/components/ui/separator'
import { useNeptuneStore } from '@/stores/neptuneStore'
import { useNeptuneWallet } from '@/composables/useNeptuneWallet'

const neptuneStore = useNeptuneStore()
const { getBlockHeight } = useNeptuneWallet()

const blockHeight = ref(0)
const loading = ref(false)
const error = ref('')
const lastUpdate = ref<Date | null>(null)

let pollingInterval: number | null = null
const POLLING_INTERVAL = 60000 // 60 seconds

const network = computed(() => neptuneStore.getNetwork || 'mainnet')

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

const loadNetworkData = async () => {
  try {
    loading.value = true
    error.value = ''

    const result = await getBlockHeight()
    blockHeight.value = Number(result.height || result)

    lastUpdate.value = new Date()
  } catch (err) {
    const errorMsg = err instanceof Error ? err.message : 'Failed to load network data'
    error.value = errorMsg
    toast.error(errorMsg)
  } finally {
    loading.value = false
  }
}

const handleRefresh = () => {
  loadNetworkData()
}

const startPolling = () => {
  pollingInterval = window.setInterval(async () => {
    if (!loading.value) await loadNetworkData()
  }, POLLING_INTERVAL)
}

const stopPolling = () => {
  if (pollingInterval) {
    clearInterval(pollingInterval)
    pollingInterval = null
  }
}

onMounted(async () => {
  await loadNetworkData()
  startPolling()
})

onUnmounted(() => {
  stopPolling()
})
</script>

<template>
  <div class="p-4">
    <div class="mx-auto max-w-2xl space-y-6">
      <!-- Page Title -->
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

        <Button variant="outline" size="icon" @click="handleRefresh" :disabled="loading">
          <RefreshCw :class="['size-4', loading && 'animate-spin']" />
        </Button>
      </div>

      <!-- Loading State -->
      <Card v-if="loading && !blockHeight" class="border-2 border-border/50 shadow-xl">
        <CardContent class="flex flex-col items-center justify-center py-16">
          <div class="mb-4 size-12 animate-spin rounded-full border-4 border-primary border-t-transparent" />
          <p class="text-sm text-muted-foreground">Loading network data...</p>
        </CardContent>
      </Card>

      <!-- Error State -->
      <Card v-else-if="error" class="border-2 border-destructive/50 shadow-xl">
        <CardContent class="space-y-4 py-12 text-center">
          <div class="flex justify-center">
            <div class="rounded-full bg-destructive/10 p-3">
              <AlertCircle class="size-8 text-destructive" />
            </div>
          </div>
          <div class="space-y-2">
            <p class="font-medium text-destructive">Connection Error</p>
            <p class="text-sm text-muted-foreground">{{ error }}</p>
          </div>
          <Button variant="outline" @click="handleRefresh">
            <RefreshCw class="mr-2 size-4" />
            Retry Connection
          </Button>
        </CardContent>
      </Card>

      <!-- Network Data -->
      <template v-else>
        <!-- Network Card -->
        <Card class="border-2 border-border/50 shadow-xl">
          <CardHeader class="space-y-1">
            <CardTitle class="flex items-center gap-2 text-lg">
              <Activity class="size-5 text-primary" />
              Network Information
            </CardTitle>
          </CardHeader>
          <CardContent class="space-y-4">
            <!-- Network Name -->
            <div class="flex items-center justify-between rounded-lg border border-border bg-muted/50 p-4 transition-colors hover:bg-muted">
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
                <div class="rounded-lg border border-border bg-gradient-to-br from-primary/5 to-primary/10 p-6 text-center">
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

        <!-- Info Card -->
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
    </div>
  </div>
</template>
