<script setup lang="ts">
import type { IconProps } from '@/interface'
import { computed } from 'vue'

const props = withDefaults(defineProps<IconProps>(), {
    size: 16,
    color: 'currentColor',
})

const iconSize = computed(() => {
    if (typeof props.size === 'number') {
        return `${props.size}px`
    }
    return props.size
})
</script>

<template>
    <svg
        :width="iconSize"
        :height="iconSize"
        :class="props.class"
        :style="{ color: props.color }"
        viewBox="0 0 24 24"
        fill="none"
    >
        <!-- Eye Icon -->
        <path
            v-if="icon === 'eye'"
            d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"
            stroke="currentColor"
            stroke-width="2"
        />
        <circle
            v-if="icon === 'eye'"
            cx="12"
            cy="12"
            r="3"
            stroke="currentColor"
            stroke-width="2"
        />

        <!-- Eye Off Icon -->
        <path
            v-if="icon === 'eye-off'"
            d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"
            stroke="currentColor"
            stroke-width="2"
        />
        <line
            v-if="icon === 'eye-off'"
            x1="1"
            y1="1"
            x2="23"
            y2="23"
            stroke="currentColor"
            stroke-width="2"
        />

        <!-- Wallet Icon -->
        <path
            v-if="icon === 'wallet'"
            d="M21 12C21 16.9706 16.9706 21 12 21C7.02944 21 3 16.9706 3 12C3 7.02944 7.02944 3 12 3C16.9706 3 21 7.02944 21 12Z"
            stroke="currentColor"
            stroke-width="2"
        />
        <path
            v-if="icon === 'wallet'"
            d="M9 12L11 14L15 10"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
        />

        <!-- Shield Icon -->
        <path
            v-if="icon === 'shield'"
            d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"
            stroke="currentColor"
            stroke-width="2"
        />
        <path
            v-if="icon === 'shield'"
            d="M9 12l2 2 4-4"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
        />

        <!-- Lock Icon -->
        <rect
            v-if="icon === 'lock'"
            x="3"
            y="11"
            width="18"
            height="11"
            rx="2"
            ry="2"
            stroke="currentColor"
            stroke-width="2"
        />
        <path
            v-if="icon === 'lock'"
            d="M7 11V7a5 5 0 0 1 10 0v4"
            stroke="currentColor"
            stroke-width="2"
        />

        <!-- Key Icon -->
        <path
            v-if="icon === 'key'"
            d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
        />

        <!-- Plus Icon -->
        <line
            v-if="icon === 'plus'"
            x1="12"
            y1="5"
            x2="12"
            y2="19"
            stroke="currentColor"
            stroke-width="2"
        />
        <line
            v-if="icon === 'plus'"
            x1="5"
            y1="12"
            x2="19"
            y2="12"
            stroke="currentColor"
            stroke-width="2"
        />

        <!-- Check Icon -->
        <path
            v-if="icon === 'check'"
            d="M20 6L9 17l-5-5"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
        />

        <!-- X Icon -->
        <line
            v-if="icon === 'x'"
            x1="18"
            y1="6"
            x2="6"
            y2="18"
            stroke="currentColor"
            stroke-width="2"
        />
        <line
            v-if="icon === 'x'"
            x1="6"
            y1="6"
            x2="18"
            y2="18"
            stroke="currentColor"
            stroke-width="2"
        />

        <!-- Arrow Right Icon -->
        <path
            v-if="icon === 'arrow-right'"
            d="M5 12h14m-7-7l7 7-7 7"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
        />

        <!-- Copy Icon -->
        <rect
            v-if="icon === 'copy'"
            x="9"
            y="9"
            width="13"
            height="13"
            rx="2"
            ry="2"
            stroke="currentColor"
            stroke-width="2"
        />
        <path
            v-if="icon === 'copy'"
            d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"
            stroke="currentColor"
            stroke-width="2"
        />
    </svg>
</template>
