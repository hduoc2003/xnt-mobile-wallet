<script setup lang="ts">
import { ref, onActivated, onDeactivated, onUnmounted } from 'vue'
import { toast } from 'vue-sonner'
import { Wallet } from 'lucide-vue-next'
import { useRouter } from 'vue-router'
import { useNeptuneStore } from '@/stores'
import WalletBalanceCard from './components/WalletBalanceCard.vue'
import WalletActions from './components/WalletActions.vue'
import { useNeptuneWallet } from '@/composables'
import { POLLING_INTERVAL } from '@/utils'

const router = useRouter()
const neptuneStore = useNeptuneStore()
const { getUtxos, getBalance, checkKeystore } = useNeptuneWallet()

const availableBalance = ref('0')
const pendingBalance = ref('0')
const loading = ref(false)
// After the first load, refreshes keep showing the last balance instead of a spinner.
const loadedOnce = ref(false)

const loadWalletData = async () => {
  try {
    loading.value = true

    if (neptuneStore.getKeystoreFileName && !neptuneStore.hasWallet) {
      await checkKeystore()
      if (!neptuneStore.hasWallet) {
        router.push({ name: 'auth' })
        return
      }
    }

    await getUtxos()
    const result = await getBalance()
    availableBalance.value = result?.balance ?? '0'
    pendingBalance.value = result?.pendingBalance ?? '0'
  } catch (error: any) {
    const errorMsg = error?.message || 'Failed to load wallet data'
    console.error('Failed to load wallet data:', error)

    if (errorMsg.includes('view key') && neptuneStore.getKeystoreFileName) {
      toast.error('Please unlock your wallet')
      router.push({ name: 'auth' })
      return
    }

    toast.error(errorMsg)
  } finally {
    loading.value = false
    loadedOnce.value = true
  }
}

// Pending amounts only clear on a reload, so keep reloading while the tab is shown and any remain.
let pollTimer: ReturnType<typeof setTimeout> | undefined
let active = false

const stopPolling = () => {
  clearTimeout(pollTimer)
  pollTimer = undefined
}

const refresh = async () => {
  // The in-flight load schedules the next poll itself.
  if (loading.value) return
  await loadWalletData()
  stopPolling()
  if (active && Number(pendingBalance.value) > 0) pollTimer = setTimeout(refresh, POLLING_INTERVAL)
}

onActivated(() => {
  active = true
  void refresh()
})

onDeactivated(() => {
  active = false
  stopPolling()
})

onUnmounted(stopPolling)
</script>

<template>
  <div class="p-4">
    <div class="mx-auto max-w-2xl space-y-6">
      <!-- Page Title -->
      <div class="flex items-center gap-3">
        <div class="rounded-full bg-primary/10 p-3">
          <Wallet class="size-6 text-primary" />
        </div>
        <div>
          <h1 class="text-2xl font-bold text-foreground">My Wallet</h1>
          <p class="text-sm text-muted-foreground">View balance and manage your wallet</p>
        </div>
      </div>

      <!-- Balance Card -->
      <WalletBalanceCard
        :available-balance="availableBalance"
        :pending-balance="pendingBalance"
        :loading="loading && !loadedOnce"
      />

      <!-- Action Buttons -->
      <WalletActions />
    </div>
  </div>
</template>
