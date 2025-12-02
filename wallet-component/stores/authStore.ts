import { ref, computed } from 'vue'
import { defineStore } from 'pinia'

export type AuthState = 'welcome' | 'create' | 'recovery'

export const useAuthStore = defineStore('auth', () => {
  // State
  const currentState = ref<AuthState>('welcome')

  // Getters
  const getCurrentState = computed(() => currentState.value)

  // Actions
  const setState = (state: AuthState) => {
    currentState.value = state
  }

  return {
    getCurrentState,
    setState,
  }
})
