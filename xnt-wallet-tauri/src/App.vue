<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { Toaster } from '@/components/ui/sonner'
import { useNeptuneWallet } from '@/composables'
import { useNeptuneStore } from '@/stores'

const router = useRouter()
const neptuneStore = useNeptuneStore()
const { clearWallet } = useNeptuneWallet()

let unlistenFocus: UnlistenFn | null = null
let unlistenBlur: UnlistenFn | null = null

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

onMounted(async () => {
  unlistenBlur = await listen('tauri://blur', () => {
    clearWallet()
  })

  unlistenFocus = await listen('tauri://focus', () => {
    setTimeoutRedirect()
  })
})

onUnmounted(() => {
  if (unlistenFocus) unlistenFocus()
  if (unlistenBlur) unlistenBlur()
  unlistenFocus = null
  unlistenBlur = null
})
</script>

<template>
  <router-view />
  <Toaster richColors closeButton :duration="2000" />
</template>
  