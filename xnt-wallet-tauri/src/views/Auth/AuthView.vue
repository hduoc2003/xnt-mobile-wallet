<script setup lang="ts">
import { computed } from 'vue'
import { OnboardingTab } from './components'
import { useAuthStore } from '@/stores'
import { LoginTab, CreateTab, RecoverSeedTab } from './components'

const authStore = useAuthStore()

const currentState = computed(() => authStore.getCurrentState())

const handleGoToCreate = () => {
    authStore.goToCreate()
}

const handleGoToRecover = () => {
    authStore.goToRecover()
}
</script>

<template>
    <div class="auth-root" style="min-height: 100vh; width: 100%; background: radial-gradient(circle at top left, #1e293b 0, #020617 40%, #000000 100%); display: flex; align-items: stretch; justify-content: center;">
        <div class="auth-container" style="flex: 1; max-width: 480px; margin: 0 auto; padding: 24px 16px 32px; display: flex; flex-direction: column; color: #e5e7eb;">
            <header class="auth-header" style="text-align: center; margin-bottom: 24px;">
                <h1 class="auth-title" style="font-size: 24px; font-weight: 700; letter-spacing: 0.04em; color: #ffffff; margin: 0;">XNT Wallet</h1>
                <p class="auth-subtitle" style="margin-top: 4px; font-size: 13px; opacity: 0.7; color: #e5e7eb;">Secure, private wallet for the XNT network</p>
            </header>

            <main class="auth-main" style="flex: 1; display: flex; flex-direction: column;">
                <OnboardingTab
                    v-if="currentState === 'onboarding'"
                    @go-to-create="handleGoToCreate"
                    @go-to-recover="handleGoToRecover"
                />
                <LoginTab v-else-if="currentState === 'login'" @go-to-create="handleGoToCreate" />
                <CreateTab v-else-if="currentState === 'create'" @go-to-recover="handleGoToRecover" />
                <RecoverSeedTab v-else-if="currentState === 'recovery'" />
                <div v-else style="padding: 20px; color: #ffffff; text-align: center;">
                    Loading...
                </div>
            </main>
        </div>
    </div>
</template>

<style lang="scss" scoped>
.auth-root {
    min-height: 100vh;
    background: radial-gradient(circle at top left, #1e293b 0, #020617 40%, #000000 100%);
    display: flex;
    align-items: stretch;
    justify-content: center;
}

.auth-container {
    flex: 1;
    max-width: 480px;
    margin: 0 auto;
    padding: 24px 16px 32px;
    display: flex;
    flex-direction: column;
    color: #e5e7eb;
}

.auth-header {
    text-align: center;
    margin-bottom: 24px;
}

.auth-title {
    font-size: 24px;
    font-weight: 700;
    letter-spacing: 0.04em;
}

.auth-subtitle {
    margin-top: 4px;
    font-size: 13px;
    opacity: 0.7;
}

.auth-main {
    flex: 1;
    display: flex;
    flex-direction: column;
}

.complete-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    text-align: center;
    padding: var(--spacing-xl);

    h2 {
        color: var(--success-color);
        margin-bottom: var(--spacing-md);
    }

    p {
        color: var(--text-secondary);
        font-size: var(--font-lg);
    }
}
</style>
