import { defineStore } from 'pinia'
import { computed, ref } from 'vue'

export type AuthState =
  | 'welcome'
  | 'create'
  | 'recovery'
  | 'confirm'
  | 'complete'

export const useAuthStore = defineStore('auth', () => {
  const currentState = ref<AuthState>('welcome')
  const previousState = ref<AuthState | null>(null)
  const showBackButton  = ref(false)
  const handleBackAction = ref<(() => void) | null>(null)

  const getCurrentState = computed(() => currentState.value)
  const getPreviousState = computed(() => previousState.value)
  const getShowBackButton = computed(() => showBackButton.value)
  const getHandleBackAction = computed(() => handleBackAction.value)

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

  const enableBackButton = (fn: () => void) => {
    showBackButton.value = true
    handleBackAction.value = fn
  }

  const disableBackButton = () => {
    showBackButton.value = false
    handleBackAction.value = null
  }

  return {
    getCurrentState,
    getPreviousState,
    getShowBackButton,
    getHandleBackAction,
    setState,
    goToWelcome,
    goToCreate,
    goToRecover,
    goBack,
    enableBackButton,
    disableBackButton,
  }
})
