<script setup lang="ts">
import { Database, Wallet, History, Network } from 'lucide-vue-next'

const route = useRoute()
const router = useRouter()

// Navigation items for bottom tab bar
const navItems = [
  { name: 'Wallet', icon: Wallet, route: '/', label: 'Wallet' },
  { name: 'UTXO', icon: Database, route: '/utxo', label: 'UTXO' },
  { name: 'History', icon: History, route: '/transaction-history', label: 'History' },
  { name: 'Network', icon: Network, route: '/network', label: 'Network' },
]

const isActiveRoute = (routePath: string) => {
  return route.path === routePath
}
</script>

<template>
  <div class="flex min-h-screen flex-col bg-background">
    <!-- Header -->
    <header class="fixed top-0 left-0 right-0 z-10 border-b border-border bg-card">
      <div class="flex h-14 items-center justify-between px-4">
        <div class="flex items-center gap-3">
          <img
            src="@/assets/imgs/neptune_logo.jpg"
            alt="Neptune"
            class="h-8 w-8 rounded-lg object-cover"
          />
          <span class="text-lg font-semibold text-foreground">Neptune Privacy</span>
        </div>
        <ThemeToggle />
      </div>
    </header>

    <!-- Main Content Area (Scrollable) -->
    <main class="flex-1 overflow-y-auto pt-14 pb-16">
      <router-view />
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
          <component :is="item.icon" :class="['h-5 w-5', isActiveRoute(item.route) && 'stroke-[2.5]']" />
          <span class="text-xs font-medium">{{ item.label }}</span>
        </button>
      </div>
    </nav>
  </div>
</template>

<style scoped>
/* Safe area for notched devices (iPhone X, etc.) */
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

