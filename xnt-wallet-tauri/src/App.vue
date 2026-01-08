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
let visibilityChangeHandler: (() => void) | null = null

const handleAppBlur = () => {
  if(router.currentRoute.value.name === 'auth') return
  clearWallet()
  shouldRedirect = true
}

const handleAppFocus = () => {
  if (!shouldRedirect) return
  shouldRedirect = false

  requestAnimationFrame(() => {
    if (!neptuneStore.hasWallet) {
      router.replace('/auth').catch(() => {})
    }
  })
}

onMounted(async () => {
  try {
    unlistenBlur = await listen('tauri://blur', handleAppBlur)
    unlistenFocus = await listen('tauri://focus', handleAppFocus)
  } catch (error) {
    console.warn('Failed to listen to Tauri blur/focus events:', error)
  }

  // Page Visibility API 
  if (typeof document !== 'undefined') {
    visibilityChangeHandler = () => {
      if (document.hidden) {
        // App went to background
        handleAppBlur()
      } else {
        // App came to foreground
        handleAppFocus()
      }
    }
    document.addEventListener('visibilitychange', visibilityChangeHandler)
  }
})

onUnmounted(() => {
  unlistenFocus?.()
  unlistenBlur?.()
  unlistenFocus = null
  unlistenBlur = null
  
  if (visibilityChangeHandler && typeof document !== 'undefined') {
    document.removeEventListener('visibilitychange', visibilityChangeHandler)
    visibilityChangeHandler = null
  }
})
</script>

<template>
  <router-view />
  <Toaster richColors closeButton :duration="2000" />
</template>
  