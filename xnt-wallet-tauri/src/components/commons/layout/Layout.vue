<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { Database, Wallet, History, Settings } from 'lucide-vue-next'
import { useRoute, useRouter } from 'vue-router'
import { ThemeToggle } from '@/components/ui/theme-toggle'

// Tab views stay mounted when switching tabs and refresh in the background on return.
const CACHED_VIEWS = [
  'WalletView',
  'UTXOView',
  'TransactionHistoryView',
  'NetworkView',
  'BridgeDepositView',
]

const route = useRoute()
const router = useRouter()

let backButtonUnlisten: (() => void) | null = null

onMounted(async () => {
  try {
    // Dynamically import to avoid errors on non-Android platforms
    const { onBackButtonPress } = await import('@tauri-apps/api/app')

    // Set up back button handler
    const unlisten = await onBackButtonPress(() => {
      const currentPath = router.currentRoute.value.path
      const currentName = router.currentRoute.value.name

      if (currentPath !== '/' && currentName !== 'wallet') {
        router.push({ name: 'wallet' })
      }

      return true
    })

    backButtonUnlisten = unlisten as unknown as () => void
  } catch (error) {
    console.debug('Back button handler not available:', error)
  }
})

onUnmounted(() => {
  if (backButtonUnlisten) {
    backButtonUnlisten()
    backButtonUnlisten = null
  }
})

const navItems = [
  { name: 'Wallet', icon: Wallet, route: '/', label: 'Wallet' },
  { name: 'UTXO', icon: Database, route: '/utxo', label: 'UTXO' },
  { name: 'History', icon: History, route: '/transaction-history', label: 'History' },
  { name: 'Settings', icon: Settings, route: '/settings', label: 'Settings' },
]

const isActiveRoute = (routePath: string) => {
  return route.path === routePath
}
</script>

<template>
  <div class="flex h-screen flex-col overflow-hidden bg-background">
    <!-- Header -->
    <header class="safe-area-top fixed top-0 left-0 right-0 z-50 border-b border-border bg-card">
      <div class="flex h-14 items-center justify-between px-4">
        <div class="flex items-center gap-3">
          <img
            src="@/assets/imgs/neptune_logo.jpg"
            alt="XNT"
            class="h-8 w-8 rounded-lg object-cover"
          />
          <span class="text-lg font-semibold text-foreground">XNT Wallet</span>
        </div>
        <ThemeToggle />
      </div>
    </header>

    <!-- Main Content Area (Scrollable) -->
    <main
      class="flex-1 overflow-y-auto pb-16"
      :style="{ paddingTop: `calc(4.5rem + env(safe-area-inset-top, 0px))` }"
    >
      <router-view v-slot="{ Component }">
        <keep-alive :include="CACHED_VIEWS">
          <component :is="Component" />
        </keep-alive>
      </router-view>
    </main>

    <!-- Bottom Navigation Bar -->
    <nav
      class="safe-area-bottom fixed bottom-0 left-0 right-0 z-10 border-t border-border bg-card shadow-[0_-4px_6px_-1px_rgb(0_0_0/0.1),0_-2px_4px_-2px_rgb(0_0_0/0.1)]"
      role="navigation"
      aria-label="Main navigation"
    >
      <div class="grid grid-cols-4">
        <button
          v-for="item in navItems"
          :key="item.name"
          type="button"
          class="flex flex-col items-center justify-center gap-1 py-2 transition-colors"
          :class="[
            isActiveRoute(item.route)
              ? 'text-primary'
              : 'text-muted-foreground hover:text-foreground active:text-foreground',
          ]"
          :aria-label="item.label"
          :aria-current="isActiveRoute(item.route) ? 'page' : undefined"
          @click="router.push(item.route)"
        >
          <component
            :is="item.icon"
            :class="['h-5 w-5', isActiveRoute(item.route) && 'stroke-[2.5]']"
          />
          <span class="text-xs font-medium">{{ item.label }}</span>
        </button>
      </div>
    </nav>
  </div>
</template>

<style scoped>
.safe-area-top {
  padding-top: calc(env(safe-area-inset-top, 0) + 1rem);
}

.safe-area-bottom {
  padding-bottom: env(safe-area-inset-bottom, 0);
}

/* Prevent overscroll on iOS */
main {
  overscroll-behavior: contain;
  -webkit-overflow-scrolling: touch;
}

/* Active tab indicator animation */
button {
  position: relative;
  -webkit-tap-highlight-color: transparent;
}

/* Ripple effect for better touch feedback */
button:active {
  transform: scale(0.95);
}
</style>
