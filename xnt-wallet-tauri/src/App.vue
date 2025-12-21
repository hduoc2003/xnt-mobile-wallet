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

let shouldRedirect = false

onMounted(async () => {
  unlistenBlur = await listen('tauri://blur', () => {
    if(router.currentRoute.value.name === 'auth') return
    clearWallet()
    shouldRedirect = true
  })

  unlistenFocus = await listen('tauri://focus', () => {
    if (!shouldRedirect) return
    shouldRedirect = false

    requestAnimationFrame(() => {
      if (!neptuneStore.hasWallet) {
        router.replace('/auth').catch(() => {})
      }
    })
  })
})

onUnmounted(() => {
  unlistenFocus?.()
  unlistenBlur?.()
  unlistenFocus = null
  unlistenBlur = null
})
</script>

<template>
  <router-view />
  <Toaster richColors closeButton :duration="2000" />
</template>
  