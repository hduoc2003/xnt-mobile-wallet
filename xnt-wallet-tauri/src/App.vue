<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { Toaster } from '@/components/ui/sonner'
import { useNeptuneWallet } from '@/composables'
import { useNeptuneStore } from '@/stores'
import { DESKTOP_IDLE_LOCK_MS } from '@/utils'

const router = useRouter()
const neptuneStore = useNeptuneStore()
const { clearWallet } = useNeptuneWallet()

let unlistenFocus: UnlistenFn | null = null
let unlistenBlur: UnlistenFn | null = null

let shouldRedirect = false
let visibilityChangeHandler: (() => void) | null = null
let nativePickerActive = false
let focusInHandler: ((e: FocusEvent) => void) | null = null

// Phones lock when the app goes to the background; desktops after a quiet spell instead,
// since any click on another window would otherwise lock them.
const platform = import.meta.env.TAURI_ENV_PLATFORM
const IS_MOBILE = platform === 'android' || platform === 'ios'
const ACTIVITY_EVENTS = ['pointerdown', 'pointermove', 'keydown', 'wheel', 'touchstart'] as const
let idleTimer: ReturnType<typeof setTimeout> | undefined
let lastActivityMs = Date.now()

const lockAfterIdle = () => {
  if (router.currentRoute.value.name === 'auth') return
  clearWallet()
  router.replace('/auth').catch(() => {})
}

const resetIdleTimer = () => {
  // Input right after waking must not erase an idle spell that already ran out.
  if (Date.now() - lastActivityMs >= DESKTOP_IDLE_LOCK_MS) lockAfterIdle()
  lastActivityMs = Date.now()
  clearTimeout(idleTimer)
  idleTimer = setTimeout(lockAfterIdle, DESKTOP_IDLE_LOCK_MS)
}

// Timers pause while the machine sleeps, so also check the elapsed time on wake.
const lockIfIdleTooLong = () => {
  if (Date.now() - lastActivityMs >= DESKTOP_IDLE_LOCK_MS) lockAfterIdle()
}

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
  if (!IS_MOBILE) {
    window.addEventListener('focus', lockIfIdleTooLong)
    document.addEventListener('visibilitychange', lockIfIdleTooLong)
    ACTIVITY_EVENTS.forEach((event) => window.addEventListener(event, resetIdleTimer))
    resetIdleTimer()
    return
  }
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
      if (
        target instanceof HTMLInputElement &&
        (target.type === 'date' || target.type === 'time')
      ) {
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
  window.removeEventListener('focus', lockIfIdleTooLong)
  document.removeEventListener('visibilitychange', lockIfIdleTooLong)
  ACTIVITY_EVENTS.forEach((event) => window.removeEventListener(event, resetIdleTimer))
  clearTimeout(idleTimer)
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
  <Toaster position="top-center" rich-colors :duration="2000" />
</template>
