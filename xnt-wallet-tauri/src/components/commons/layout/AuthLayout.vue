<script setup lang="ts">
import { ThemeToggle } from '@/components/ui/theme-toggle'
import { ArrowLeftCommon } from '@/components'
import { useAuthStore } from '@/stores' 

const authStore = useAuthStore()
const handleBack = () => {
  authStore.getHandleBackAction?.() || (() => {})()
}
</script>

<template>
  <div class="flex h-screen flex-col overflow-hidden bg-background">
    <header class="safe-area-top relative bg-transparent">
      <div class="flex h-10 items-center justify-between px-4 pt-5">
        <div>
          <ArrowLeftCommon v-if="authStore.getShowBackButton" @click="handleBack" />
        </div>
        <ThemeToggle />
      </div>
    </header>
    <main class="main-content flex-1 overflow-y-auto from-background via-background to-primary/5 pt-10">
      <slot />
    </main>
  </div>
</template>

<style scoped>
.safe-area-top {
  padding-top: calc(env(safe-area-inset-top, 0) + 1rem);
}

main {
  overscroll-behavior: contain;
  -webkit-overflow-scrolling: touch;
}
</style>
