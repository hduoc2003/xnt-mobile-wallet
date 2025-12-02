import { useAuthStore } from '@/stores'

export function useAuthRouting() {
  const authStore = useAuthStore()

  const goToCreate = () => {
    authStore.setState('create')
  }

  const goToRecover = () => {
    authStore.setState('recovery')
  }

  const goBackToWelcome = () => {
    authStore.setState('welcome')
  }

  return {
    goToCreate,
    goToRecover,
    goBackToWelcome,
  }
}
