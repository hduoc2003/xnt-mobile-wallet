<script setup lang="ts">
import { ref, computed } from 'vue'

interface Props {
    title?: string
    subtitle?: string
    buttonText?: string
    hideBackButton?: boolean
    backButtonText?: string
    placeholder?: string
    label?: string
    loading?: boolean
    error?: boolean
    errorMessage?: string
    validateFormat?: boolean
}

const props = withDefaults(defineProps<Props>(), {
    title: 'Access Wallet',
    subtitle: 'Enter your password to unlock your wallet',
    buttonText: 'Unlock Wallet',
    backButtonText: 'Back',
    hideBackButton: false,
    placeholder: 'Enter your password',
    label: 'Password',
    loading: false,
    error: false,
    errorMessage: 'Invalid password',
    validateFormat: false,
})

const emit = defineEmits<{
    submit: [password: string]
    back: []
}>()

const password = ref('')
const passwordError = ref('')

const passwordStrength = computed(() => {
    if (!password.value || !props.validateFormat) return { level: 0, text: '', color: '' }

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

const canProceed = computed(() => {
    if (!password.value || passwordError.value) return false

    if (props.validateFormat) {
        return password.value.length >= 8 && passwordStrength.value.level >= 2
    }

    return password.value.length > 0
})

const handleSubmit = () => {
    if (!canProceed.value) {
        if (!password.value) {
            passwordError.value = 'Please enter your password'
        } else if (props.validateFormat && password.value.length < 8) {
            passwordError.value = 'Password must be at least 8 characters'
        } else if (props.validateFormat && passwordStrength.value.level < 2) {
            passwordError.value = 'Password is too weak'
        }
        return
    }
    passwordError.value = ''
    emit('submit', password.value)
}

const handleBack = () => {
    password.value = ''
    passwordError.value = ''
    emit('back')
}
</script>

<template>
    <CardBase>
        <div class="auth-card-content">
            <div class="form-group">
                <FormCommon
                    v-model="password"
                    type="password"
                    :label="props.label"
                    :placeholder="props.placeholder"
                    show-password-toggle
                    required
                    :error="passwordError"
                    @input="passwordError = ''"
                    @keyup.enter="handleSubmit"
                />

                <!-- Password Strength Indicator -->
                <div v-if="props.validateFormat && password" class="password-strength">
                    <div class="strength-bar">
                        <div
                            class="strength-fill"
                            :style="{
                                width: `${(passwordStrength.level / 4) * 100}%`,
                                backgroundColor: passwordStrength.color,
                            }"
                        ></div>
                    </div>
                    <span class="strength-text" :style="{ color: passwordStrength.color }">
                        {{ passwordStrength.text }}
                    </span>
                </div>

                <!-- Helper Text -->
                <p v-if="props.validateFormat" class="helper-text">
                    Password must be at least 8 characters with uppercase, lowercase, and numbers.
                </p>

                <span v-if="error" class="error-message">{{ errorMessage }}</span>
            </div>

            <div class="auth-button-group">
                <ButtonCommon
                    v-if="!props.hideBackButton"
                    type="default"
                    size="large"
                    class="auth-button"
                    block
                    @click="handleBack"
                >
                    {{ props.backButtonText }}
                </ButtonCommon>
                <ButtonCommon
                    type="primary"
                    size="large"
                    class="auth-button"
                    block
                    :disabled="!canProceed"
                    :loading="props.loading"
                    @click="handleSubmit"
                >
                    {{ props.buttonText }}
                </ButtonCommon>
            </div>
        </div>
    </CardBase>
</template>

<style lang="scss" scoped>
.auth-card-content {
    .form-group {
        margin-bottom: var(--spacing-xl);
    }
}

.auth-button {
    width: fit-content;
    margin: 0 auto;
}

.auth-button-group {
    width: 50%;
    margin: 0 auto;
    margin-top: var(--spacing-2xl);
    @include center_flex;
    gap: var(--spacing-md);
}

.error-message {
    display: block;
    padding: var(--spacing-sm) var(--spacing-md);
    background: var(--error-light);
    color: var(--error-color);
    border-radius: var(--radius-sm);
    font-size: var(--font-sm);
    text-align: center;
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

.helper-text {
    font-size: var(--font-xs);
    color: var(--text-muted);
    margin: var(--spacing-xs) 0 0;
    line-height: var(--leading-normal);
}
</style>
