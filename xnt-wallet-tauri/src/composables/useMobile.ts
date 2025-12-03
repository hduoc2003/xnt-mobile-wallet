import { computed } from 'vue'
import { useMediaQuery } from '@vueuse/core'

/**
 * Detect if device is mobile
 */
export function useMobile() {
  const isMobile = useMediaQuery('(max-width: 768px)')

  const isIOS = computed(() => {
    if (typeof window === 'undefined') return false
    return /iPhone|iPad|iPod/i.test(navigator.userAgent)
  })

  /**
   * Get safe area insets for notched devices
   */
  function useSafeArea() {
    return {
      top: computed(() => {
        if (typeof window === 'undefined') return 0
        return parseInt(
          getComputedStyle(document.documentElement).getPropertyValue('env(safe-area-inset-top)') ||
            '0'
        )
      }),
      bottom: computed(() => {
        if (typeof window === 'undefined') return 0
        return parseInt(
          getComputedStyle(document.documentElement).getPropertyValue(
            'env(safe-area-inset-bottom)'
          ) || '0'
        )
      }),
    }
  }

  /**
   * Prevent pull-to-refresh on mobile browsers
   */
  function usePreventPullToRefresh() {
    if (typeof window === 'undefined') return

    let touchStartY = 0

    document.addEventListener(
      'touchstart',
      e => {
        touchStartY = e.touches[0]?.clientY ?? 0
      },
      { passive: false }
    )

    document.addEventListener(
      'touchmove',
      e => {
        const touchY = e.touches[0]?.clientY ?? 0
        const touchYDelta = touchY - touchStartY

        // Prevent pull-to-refresh if scrolling up at the top
        if (touchYDelta > 0 && window.scrollY === 0) {
          e.preventDefault()
        }
      },
      { passive: false }
    )
  }

  return {
    isMobile,
    isIOS,
    useSafeArea,
    usePreventPullToRefresh,
  }
}
