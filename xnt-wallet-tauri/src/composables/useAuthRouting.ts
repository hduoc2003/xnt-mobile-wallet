import { useAuthStore } from '@/stores'

export function useAuthRouting() {
  const authStore = useAuthStore()

  const goToCreate = () => {
    if ((window as any).logger) {
      ;(window as any).logger.info('[useAuthRouting] Navigating to create')
    }
    authStore.goToCreate()
  }

  const goToRecover = () => {
    if ((window as any).logger) {
      ;(window as any).logger.info('[useAuthRouting] Navigating to recovery')
    }
    authStore.goToRecover()
  }

  const goBackToWelcome = () => {
    if ((window as any).logger) {
      ;(window as any).logger.info('[useAuthRouting] Navigating to welcome')
    }
    authStore.goToWelcome()
  }

  return {
    goToCreate,
    goToRecover,
    goBackToWelcome,
  }
}
