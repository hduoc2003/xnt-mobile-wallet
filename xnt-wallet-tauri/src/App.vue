<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { Toaster } from '@/components/ui/sonner'
import { useNeptuneWallet } from '@/composables'
import { useNeptuneStore } from '@/stores'

const router = useRouter()
const neptuneStore = useNeptuneStore()
const { clearWallet } = useNeptuneWallet()

const handleVisibilityChange = () => {
  if (document.hidden) clearWallet()
  else setTimeoutRedirect()
}

const setTimeoutRedirect = () => {
  setTimeout(() => {
    if (!neptuneStore.hasWallet) {
      const currentRoute = router.currentRoute.value
      if (currentRoute.path !== '/auth' && currentRoute.name !== 'auth') {
        router.replace('/auth').catch(() => {
          router.push('/auth').catch(() => {})
        })
      }
    }
  }, 50)
}

onMounted(() => {
  document.addEventListener('visibilitychange', handleVisibilityChange)

  window.addEventListener('blur', () => { clearWallet() })
  window.addEventListener('focus', () => { setTimeoutRedirect() })
})

onUnmounted(() => {
  document.removeEventListener('visibilitychange', handleVisibilityChange)
  window.removeEventListener('blur', handleVisibilityChange)
  window.removeEventListener('focus', handleVisibilityChange)
})
</script>

<template>
  <router-view />
  <Toaster richColors closeButton :duration="2000" />
</template>
