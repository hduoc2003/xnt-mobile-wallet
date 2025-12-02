import { ref } from 'vue'

// Auth flow states
export type AuthState = 'onboarding' | 'login' | 'create' | 'recovery' | 'confirm' | 'complete'

// Auth store to manage the flow
const currentState = ref<AuthState>('onboarding')
const previousState = ref<AuthState | null>(null)

export const useAuthStore = () => {
    const getCurrentState = () => currentState.value
    const getPreviousState = () => previousState.value

    const setState = (state: AuthState) => {
        previousState.value = currentState.value
        currentState.value = state
    }

    const goToCreate = () => {
        setState('create')
    }

    const goToLogin = () => {
        setState('login')
    }

    const goToRecover = () => {
        setState('recovery')
    }

    const goBack = () => {
        previousState.value ? setState(previousState.value) : setState('onboarding')
    }

    return {
        currentState: currentState.value,
        getCurrentState,
        getPreviousState,
        setState,
        goToCreate,
        goToLogin,
        goToRecover,
        goBack,
    }
}
