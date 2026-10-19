<script setup lang="ts">
import { ref, computed, onActivated, onDeactivated, onMounted } from 'vue'
import { AlertCircle, RefreshCw } from 'lucide-vue-next'
import { toast } from 'vue-sonner'
import { useNeptuneStore } from '@/stores'
import { useNeptuneWallet, useRpc } from '@/composables'
import NetworkStatusHeader from './components/NetworkStatusHeader.vue'
import NetworkInfoCard from './components/NetworkInfoCard.vue'
import AccountsCard from './components/AccountsCard.vue'
import RpcEndpointCard from './components/RpcEndpointCard.vue'
import TransactionSettingsCard from './components/TransactionSettingsCard.vue'
import InfoCard from './components/InfoCard.vue'

const neptuneStore = useNeptuneStore()
const { getBlockHeight } = useNeptuneWallet()
const { loadRpcEndpoint } = useRpc()

const blockHeight = ref(0)
const loading = ref(false)
const error = ref(false)
const lastUpdate = ref<Date | null>(null)

let pollingInterval: number | null = null
const POLLING_INTERVAL = 60000 // 60 seconds

const network = computed(() => neptuneStore.getNetwork || 'mainnet')

const loadNetworkData = async () => {
  try {
    loading.value = true
    error.value = false

    const result = await getBlockHeight()
    blockHeight.value = Number(result.height || result)

    lastUpdate.value = new Date()
  } catch (err) {
    const errorMsg = err instanceof Error ? err.message : 'Failed to load network data'
    toast.error(errorMsg)
    blockHeight.value = 0
    lastUpdate.value = null
    error.value = true
  } finally {
    loading.value = false
  }
}

const handleRefresh = () => {
  loadNetworkData()
}

const startPolling = () => {
  stopPolling()
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

onMounted(loadRpcEndpoint)

onActivated(() => {
  startPolling()
  void loadNetworkData()
})

onDeactivated(stopPolling)
</script>

<template>
  <div class="min-h-full p-4">
    <div class="mx-auto max-w-2xl space-y-6 pb-4">
      <!-- Page Header -->
      <NetworkStatusHeader :loading="loading" @refresh="handleRefresh" />

      <!-- Loading State -->
      <Card v-if="loading" class="border-2 border-border/50 shadow-xl">
        <CardContent class="flex flex-col items-center justify-center py-16">
          <div
            class="mb-4 size-12 animate-spin rounded-full border-4 border-primary border-t-transparent"
          />
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
          </div>
          <Button variant="outline" @click="handleRefresh">
            <RefreshCw class="mr-2 size-4" />
            Retry Connection
          </Button>
        </CardContent>
      </Card>

      <template v-else>
        <!-- Network Data -->
        <NetworkInfoCard :network="network" :block-height="blockHeight" :last-update="lastUpdate" />
      </template>

      <AccountsCard />

      <RpcEndpointCard @endpoint-updated="loadNetworkData" />

      <TransactionSettingsCard />

      <InfoCard />
    </div>
  </div>
</template>
