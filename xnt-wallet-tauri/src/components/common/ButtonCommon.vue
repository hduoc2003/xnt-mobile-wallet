<script setup lang="ts">
import type { ButtonProps } from '@/interface'
import { computed } from 'vue'

const props = withDefaults(defineProps<ButtonProps>(), {
    type: 'default',
    size: 'large',
    block: false,
    disabled: false,
    loading: false,
    htmlType: 'button',
})

const emit = defineEmits(['click'])

const handleClick = () => {
    if (!props.disabled && !props.loading) {
        emit('click')
    }
}

const buttonClasses = computed(() => {
    return [
        'btn-common',
        `btn-${props.type}`,
        `btn-${props.size}`,
        {
            'btn-block': props.block,
            'btn-disabled': props.disabled || props.loading,
            'btn-loading': props.loading,
        },
    ]
})
</script>

<template>
    <button
        :type="props.htmlType"
        :class="buttonClasses"
        :disabled="props.disabled || props.loading"
        @click="handleClick"
    >
        <span v-if="props.loading" class="btn-spinner">
            <SpinnerCommon size="small" />
        </span>
        <slot />
    </button>
</template>

<style lang="scss" scoped>
.btn-common {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--spacing-xs);
    font-weight: var(--font-semibold);
    border-radius: var(--btn-radius);
    letter-spacing: var(--tracking-wide);
    transition: var(--transition-all);
    cursor: pointer;
    border: 2px solid transparent;
    outline: none;
    white-space: nowrap;
    user-select: none;

    // Sizes
    &.btn-small {
        padding: 6px 12px;
        font-size: var(--font-sm);
    }

    &.btn-medium {
        padding: 10px 16px;
        font-size: var(--font-base);
    }

    &.btn-large {
        padding: var(--btn-padding-y) var(--btn-padding-x);
        font-size: var(--font-base);
    }

    // Types
    &.btn-primary {
        background: var(--primary-color);
        border-color: var(--primary-color);
        color: var(--text-light);

        &:hover:not(.btn-disabled) {
            background: var(--primary-hover);
            border-color: var(--primary-hover);
        }

        &:active:not(.btn-disabled),
        &:focus:not(.btn-disabled) {
            background: var(--primary-hover);
            border-color: var(--primary-hover);
        }
    }

    &.btn-default {
        background: var(--bg-white);
        border-color: var(--border-color);
        color: var(--text-primary);

        &:hover:not(.btn-disabled) {
            background: var(--bg-hover);
            border-color: var(--primary-color);
            color: var(--primary-color);
        }

        &:active:not(.btn-disabled),
        &:focus:not(.btn-disabled) {
            background: var(--bg-hover);
            border-color: var(--primary-color);
        }
    }

    &.btn-dashed {
        background: transparent;
        border-style: dashed;
        border-color: var(--border-color);
        color: var(--text-primary);

        &:hover:not(.btn-disabled) {
            border-color: var(--primary-color);
            color: var(--primary-color);
        }
    }

    &.btn-link {
        background: transparent;
        border-color: transparent;
        color: var(--primary-color);

        &:hover:not(.btn-disabled) {
            color: var(--primary-hover);
        }
    }

    &.btn-text {
        background: transparent;
        border-color: transparent;
        color: var(--text-primary);

        &:hover:not(.btn-disabled) {
            background: var(--bg-hover);
        }
    }

    // States
    &.btn-block {
        display: flex;
        width: 100%;
    }

    &.btn-disabled {
        opacity: 0.6;
        cursor: not-allowed;
    }

    &.btn-loading {
        cursor: wait;
    }

    .btn-spinner {
        display: inline-flex;
        align-items: center;
    }
}
</style>
