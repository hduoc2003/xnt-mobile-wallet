<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { Toaster } from '@/components/ui/sonner'
import { useNeptuneWallet } from '@/composables'

const { clearWallet } = useNeptuneWallet()

const handleVisibilityChange = () => {
  if (document.hidden) clearWallet()
}

onMounted(() => {
  document.addEventListener('visibilitychange', handleVisibilityChange)
  window.addEventListener('blur', handleVisibilityChange)
})

onUnmounted(() => {
  document.removeEventListener('visibilitychange', handleVisibilityChange)
  window.removeEventListener('blur', handleVisibilityChange)
})
</script>

<template>
  <router-view />
  <Toaster richColors closeButton :duration="2000" />
</template>
