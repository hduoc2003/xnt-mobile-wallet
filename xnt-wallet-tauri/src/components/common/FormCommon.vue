<script setup lang="ts">
import { ref, computed } from 'vue'
import { IconCommon } from '../icon'

defineOptions({
    inheritAttrs: false,
})

const props = defineProps({
    modelValue: { type: String, default: '' },
    type: { type: String, default: 'text' },
    placeholder: { type: String, default: '' },
    label: { type: String, default: '' },
    required: { type: Boolean, default: false },
    disabled: { type: Boolean, default: false },
    showPasswordToggle: { type: Boolean, default: false },
    error: { type: String, default: '' },
})

const emit = defineEmits(['update:modelValue', 'focus', 'blur'])

const showPassword = ref(false)
const isFocused = ref(false)

const inputType = computed(() => {
    if (props.type === 'password') {
        return showPassword.value ? 'text' : 'password'
    }
    return props.type
})

const togglePassword = () => (showPassword.value = !showPassword.value)
const handleInput = (e: Event) => emit('update:modelValue', (e.target as HTMLInputElement).value)
const handleFocus = (e: FocusEvent) => {
    isFocused.value = true
    emit('focus', e)
}
const handleBlur = (e: FocusEvent) => {
    isFocused.value = false
    emit('blur', e)
}
</script>

<template>
    <div class="form-field">
        <label v-if="label" class="form-label">
            {{ label }}<span v-if="required" class="required-asterisk">*</span>
        </label>

        <div class="input-container" :class="{ focused: isFocused, error, disabled }">
            <input
                v-bind="$attrs"
                :type="inputType"
                :value="modelValue"
                :placeholder="placeholder"
                :disabled="disabled"
                :required="required"
                class="form-input"
                @input="handleInput"
                @focus="handleFocus"
                @blur="handleBlur"
            />

            <div
                v-if="type === 'password' && showPasswordToggle"
                class="password-toggle flex-center"
                @click="togglePassword"
            >
                <IconCommon :size="20" :icon="showPassword ? 'eye-off' : 'eye'" />
            </div>
        </div>

        <div v-if="error" class="error-message">{{ error }}</div>
    </div>
</template>

<style lang="scss" scoped>
.form-field {
    margin-bottom: 16px;
}

.form-label {
    display: block;
    margin-bottom: 8px;
    font-size: 14px;
    font-weight: 500;
    color: var(--vt-c-black-bold);
    .required-asterisk {
        color: var(--vt-c-red-v3);
        margin-left: 2px;
    }
}

.input-container {
    position: relative;
    border: 1px solid var(--border-color);
    border-radius: 6px;
    transition: all 0.2s ease;
    &.focused {
        border-color: var(--border-primary);
        box-shadow: var(--shadow-primary);
    }
    &.error {
        border-color: var(--error-color);
        background-color: var();
    }
    &.disabled {
        background-color: var(--bg-hover);
        cursor: not-allowed;
    }
}

.form-input {
    width: 100%;
    padding: 12px 40px 12px 12px;
    border: var(--border-color);
    background: var(--text-light);
    font-size: var(--font-base);
    color: var(--text-primary);
    outline: none;
    border-radius: var(--radius-md);
    &::placeholder {
        color: var(--text-muted);
    }
    &:disabled {
        cursor: not-allowed;
        color: var(--text-muted);
    }
}

.password-toggle {
    position: absolute;
    right: 12px;
    top: 50%;
    transform: translateY(-50%);
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-muted);
    padding: 4px;
    &:hover {
        color: var(--text-primary);
    }
    &:disabled {
        cursor: not-allowed;
        opacity: 0.5;
    }
}

.error-message {
    margin-top: 4px;
    font-size: 12px;
    color: var(--error-color);
    display: flex;
    align-items: center;
}
</style>
