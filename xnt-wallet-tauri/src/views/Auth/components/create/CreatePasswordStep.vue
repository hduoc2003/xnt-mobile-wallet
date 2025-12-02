<script setup lang="ts">
import { ref, computed } from 'vue'

const emit = defineEmits<{
    next: [password: string]
    navigateToRecoverWallet: []
}>()

const password = ref('')
const confirmPassword = ref('')
const passwordError = ref('')
const confirmPasswordError = ref('')

const passwordStrength = computed(() => {
    if (!password.value) return { level: 0, text: '', color: '' }

    let strength = 0
    const checks = {
        length: password.value.length >= 8,
        uppercase: /[A-Z]/.test(password.value),
        lowercase: /[a-z]/.test(password.value),
        number: /[0-9]/.test(password.value),
        special: /[!@#$%^&*(),.?":{}|<>]/.test(password.value),
    }
    strength = Object.values(checks).filter(Boolean).length
    if (strength <= 2) return { level: 1, text: 'Weak', color: 'var(--error-color)' }
    if (strength <= 3) return { level: 2, text: 'Medium', color: 'var(--warning-color)' }
    if (strength <= 4) return { level: 3, text: 'Good', color: 'var(--info-color)' }
    return { level: 4, text: 'Strong', color: 'var(--success-color)' }
})

const isPasswordMatch = computed(() => {
    if (!confirmPassword.value) return true
    return password.value === confirmPassword.value
})

const canProceed = computed(() => {
    return (
        password.value.length >= 8 &&
        confirmPassword.value.length >= 8 &&
        isPasswordMatch.value &&
        passwordStrength.value.level >= 2
    )
})

const handleNext = () => {
    if (!canProceed.value) {
        if (password.value.length < 8) {
            passwordError.value = 'Password must be at least 8 characters'
        }
        if (!isPasswordMatch.value) {
            confirmPasswordError.value = 'Passwords do not match'
        }
        return
    }
    emit('next', password.value)
}

const handleIHaveWallet = () => {
    emit('navigateToRecoverWallet')
}
</script>

<template>
    <div class="create-password-step">
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
            <h1 class="auth-title">Create New Wallet</h1>
            <p class="auth-subtitle">Secure your wallet with a strong password</p>
        </div>

        <div class="auth-card-content">
            <div class="form-group">
                <FormCommon
                    v-model="password"
                    type="password"
                    label="Create Password"
                    placeholder="Enter your password"
                    show-password-toggle
                    required
                    :error="passwordError"
                    @input="passwordError = ''"
                />
                <div v-if="password" class="password-strength">
                    <div class="strength-bar">
                        <div
                            class="strength-fill"
                            :style="{
                                width: `${(passwordStrength.level / 4) * 100}%`,
                                backgroundColor: passwordStrength.color,
                            }"
                        ></div>
                    </div>
                    <span class="strength-text" :style="{ color: passwordStrength.color }">{{
                        passwordStrength.text
                    }}</span>
                </div>
            </div>

            <div class="form-group">
                <FormCommon
                    v-model="confirmPassword"
                    type="password"
                    label="Confirm Password"
                    placeholder="Re-enter your password"
                    show-password-toggle
                    required
                    :error="confirmPasswordError"
                    @input="confirmPasswordError = ''"
                    @keyup.enter="handleNext"
                />
                <div
                    v-if="confirmPassword"
                    class="password-match"
                    :class="{ match: isPasswordMatch }"
                >
                    <span v-if="isPasswordMatch" class="match-text"> ✓ Passwords match </span>
                    <span v-else class="match-text error"> Passwords do not match </span>
                </div>
            </div>

            <p class="helper-text">
                Password must be at least 8 characters with uppercase, lowercase, and numbers.
            </p>

            <div class="auth-button-group">
                <ButtonCommon
                    type="primary"
                    size="large"
                    class="auth-button"
                    block
                    :disabled="!canProceed"
                    @click="handleNext"
                >
                    Create Wallet
                </ButtonCommon>
                <div class="secondary-actions">
                    <ButtonCommon
                        type="link"
                        size="small"
                        class="link-button"
                        @click="handleIHaveWallet"
                    >
                        Already have a wallet?
                    </ButtonCommon>
                </div>
            </div>
        </div>
    </div>
</template>

<style lang="scss" scoped>
.create-password-step {
    width: 100%;
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

.auth-card-content {
    .form-group {
        margin-bottom: var(--spacing-xl);
    }
}

.password-strength {
    margin-top: var(--spacing-sm);
    display: flex;
    align-items: center;
    gap: var(--spacing-md);

    .strength-bar {
        flex: 1;
        height: 4px;
        background: var(--border-light);
        border-radius: var(--radius-full);
        overflow: hidden;

        .strength-fill {
            height: 100%;
            transition: all 0.3s ease;
        }
    }

    .strength-text {
        font-size: var(--font-xs);
        font-weight: var(--font-medium);
        min-width: 50px;
        text-align: right;
    }
}

.password-match {
    margin-top: var(--spacing-sm);
    font-size: var(--font-xs);

    &.match .match-text {
        color: var(--success-color);
    }

    .match-text.error {
        color: var(--error-color);
    }
}

.helper-text {
    font-size: var(--font-xs);
    color: var(--text-muted);
    margin: 0 0 var(--spacing-xl);
    line-height: var(--leading-normal);
}

.auth-button {
    width: fit-content;
    margin: 0 auto;
}

.auth-button-group {
    margin-top: var(--spacing-2xl);
    display: flex;
    flex-direction: column;

    .secondary-actions {
        display: flex;
        justify-content: center;
        align-items: center;
        gap: var(--spacing-sm);
        margin-top: var(--spacing-md);
    }

    .link-button {
        background: none;
        border: none;
        color: var(--primary-color);
        font-size: var(--font-sm);
        cursor: pointer;
        transition: color 0.2s ease;
        padding: 0;

        &:hover {
            color: var(--primary-hover);
            text-decoration: underline;
        }
    }

    .separator {
        color: var(--text-muted);
        font-size: var(--font-sm);
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
