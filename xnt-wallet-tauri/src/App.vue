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
let nativePickerActive = false
let focusInHandler: ((e: FocusEvent) => void) | null = null

const handleAppBlur = () => {
  if (router.currentRoute.value.name === 'auth') return
  // If a native date/time picker was just opened, skip logout regardless of duration
  if (nativePickerActive) return
  clearWallet()
  shouldRedirect = true
}

const handleAppFocus = () => {
  if (nativePickerActive) {
    // Native picker closed — WebView regained focus, resume normally
    nativePickerActive = false
    return
  }
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

  if (typeof document !== 'undefined') {
    // Detect native date/time picker activation so we don't logout during picking
    focusInHandler = (e: FocusEvent) => {
      const target = e.target as HTMLElement
      if (target instanceof HTMLInputElement && (target.type === 'date' || target.type === 'time')) {
        nativePickerActive = true
      }
    }
    document.addEventListener('focusin', focusInHandler)

    // Page Visibility API
    visibilityChangeHandler = () => {
      if (document.hidden) {
        handleAppBlur()
      } else {
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

  if (typeof document !== 'undefined') {
    if (focusInHandler) {
      document.removeEventListener('focusin', focusInHandler)
      focusInHandler = null
    }
    if (visibilityChangeHandler) {
      document.removeEventListener('visibilitychange', visibilityChangeHandler)
      visibilityChangeHandler = null
    }
  }
})
</script>

<template>
  <router-view />
  <Toaster richColors closeButton :duration="2000" />
</template>
  