import { ref } from 'vue'

// Auth flow states for the new UI
export type AuthState =
  | 'welcome'
  | 'create'
  | 'recovery'
  | 'confirm'
  | 'complete'

// Singletons to preserve state across composables
const currentState = ref<AuthState>('welcome')
const previousState = ref<AuthState | null>(null)

export const useAuthStore = () => {
  const getCurrentState = () => currentState.value
  const getPreviousState = () => previousState.value

  const setState = (state: AuthState) => {
    previousState.value = currentState.value
    currentState.value = state
  }

  const goToWelcome = () => {
    setState('welcome')
  }

  const goToCreate = () => {
    setState('create')
  }

  const goToRecover = () => {
    setState('recovery')
  }

  const goBack = () => {
    previousState.value ? setState(previousState.value) : setState('welcome')
  }

  return {
    currentState,
    previousState,
    getCurrentState,
    getPreviousState,
    setState,
    goToWelcome,
    goToCreate,
    goToRecover,
    goBack,
  }
}
