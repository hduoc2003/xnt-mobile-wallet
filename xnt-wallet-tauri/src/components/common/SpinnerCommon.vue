<script setup lang="ts">
import { computed } from 'vue'

interface Props {
    size?: 'small' | 'medium' | 'large'
    color?: string
}

const props = withDefaults(defineProps<Props>(), {
    size: 'medium',
    color: 'var(--primary-color)',
})

const spinnerSize = computed(() => {
    const sizes = {
        small: '24px',
        medium: '40px',
        large: '60px',
    }
    return sizes[props.size]
})

const borderWidth = computed(() => {
    const widths = {
        small: '3px',
        medium: '4px',
        large: '5px',
    }
    return widths[props.size]
})
</script>

<template>
    <div
        class="spinner"
        :style="{
            width: spinnerSize,
            height: spinnerSize,
            borderWidth: borderWidth,
            borderTopColor: color,
        }"
    ></div>
</template>

<style lang="scss" scoped>
.spinner {
    border: 4px solid var(--border-light);
    border-top-color: var(--primary-color);
    border-radius: 50%;
    animation: spin 1s linear infinite;
}
</style>
