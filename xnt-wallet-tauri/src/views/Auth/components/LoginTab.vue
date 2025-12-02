<script setup lang="ts">
import { ref } from 'vue'
import { useNeptuneWallet } from '@/composables/useNeptuneWallet'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/authStore'

const authStore = useAuthStore()
const router = useRouter()
const neptuneWallet = useNeptuneWallet()

const isLoading = ref(false)
const error = ref(false)

const handlePasswordSubmit = async (password: string) => {
    try {
        isLoading.value = true
        error.value = false
        await neptuneWallet.decryptKeystore(password)
        router.push({ name: 'home' })
    } catch (err) {
        error.value = true
    } finally {
        isLoading.value = false
    }
}

const handleNewWallet = () => {
    authStore.goToCreate()
    router.push({ name: 'auth' })
}
</script>

<template>
    <div class="login-tab">
        <div class="login-password-form">
            <div class="auth-card-header">
                <div class="logo-container">
                    <div class="logo-circle">
                        <img src="@/assets/imgs/logo.png" alt="XNT Logo" class="neptune-logo" />
                    </div>
                    <div class="logo-text">
                        <span class="coin-name">XNT</span>
                        <span class="coin-symbol">XNT</span>
                    </div>
                </div>
                <h1 class="auth-title">Access Wallet</h1>
                <p class="auth-subtitle">Enter your password to unlock your wallet</p>
            </div>

            <PasswordForm
                button-text="Unlock Wallet"
                placeholder="Enter your password"
                back-button-text="New Wallet"
                label="Password"
                :loading="isLoading"
                @submit="handlePasswordSubmit"
                @back="handleNewWallet"
                :error="error"
                :errorMessage="'Invalid password'"
            />
        </div>
    </div>
</template>

<style lang="scss" scoped>
.login-tab {
    padding: var(--spacing-lg);
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100%;
}

.login-password-form {
    width: 100%;
    max-width: 480px;
}

.auth-card-header {
    text-align: center;
    margin-bottom: var(--spacing-2xl);
    padding-bottom: var(--spacing-xl);
    border-bottom: 1px solid var(--border-color);

    .logo-container {
        display: flex;
        align-items: center;
        justify-content: center;
        gap: var(--spacing-md);
        margin-bottom: var(--spacing-lg);

        .logo-circle {
            width: 48px;
            height: 48px;
            border-radius: 50%;
            background: linear-gradient(135deg, var(--primary-light), var(--bg-white));
            display: flex;
            align-items: center;
            justify-content: center;
            padding: var(--spacing-sm);
            box-shadow: 0 2px 8px rgba(0, 127, 207, 0.15);

            .neptune-logo {
                width: 100%;
                height: 100%;
                object-fit: contain;
                border-radius: 50%;
            }
        }

        .logo-text {
            display: flex;
            flex-direction: column;
            align-items: flex-start;

            .coin-name {
                font-size: var(--font-lg);
                font-weight: var(--font-bold);
                color: var(--text-primary);
                line-height: 1;
                margin-bottom: 2px;
            }

            .coin-symbol {
                font-size: var(--font-xs);
                font-weight: var(--font-medium);
                color: var(--primary-color);
                background: var(--primary-light);
                padding: 2px 8px;
                border-radius: var(--radius-sm);
            }
        }
    }

    .auth-title {
        font-size: var(--font-2xl);
        font-weight: var(--font-bold);
        color: var(--text-primary);
        margin-bottom: var(--spacing-xs);
    }

    .auth-subtitle {
        font-size: var(--font-sm);
        color: var(--text-secondary);
        margin: 0;
    }
}

@include screen(mobile) {
    .auth-card-header {
        .logo-container {
            .logo-circle {
                width: 40px;
                height: 40px;
            }

            .logo-text {
                .coin-name {
                    font-size: var(--font-md);
                }
            }
        }

        .auth-title {
            font-size: var(--font-xl);
        }
    }
}
</style>
