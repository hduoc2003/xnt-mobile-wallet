<script setup lang="ts">
import { provide, computed } from 'vue'

interface TabItem {
    key: string
    label: string
    disabled?: boolean
}

interface Props {
    modelValue: string
    items: TabItem[]
    size?: 'small' | 'medium' | 'large'
}

const props = withDefaults(defineProps<Props>(), {
    size: 'medium',
})

const emit = defineEmits<{
    'update:modelValue': [value: string]
}>()

const activeKey = computed({
    get: () => props.modelValue,
    set: (value: string) => emit('update:modelValue', value),
})

provide('activeTabKey', activeKey)

const handleTabClick = (key: string, disabled?: boolean) => {
    if (disabled) return
    activeKey.value = key
}

const tabClasses = computed(() => {
    return ['tabs-common', `tabs-${props.size}`]
})
</script>

<template>
    <div :class="tabClasses">
        <div class="tabs-nav">
            <div
                v-for="item in items"
                :key="item.key"
                class="tab-item"
                :class="{
                    'tab-active': activeKey === item.key,
                    'tab-disabled': item.disabled,
                }"
                @click="handleTabClick(item.key, item.disabled)"
            >
                {{ item.label }}
            </div>
            <div
                class="tabs-ink-bar"
                :style="{
                    left: `${items.findIndex((item) => item.key === activeKey) * (100 / items.length)}%`,
                    width: `${100 / items.length}%`,
                }"
            />
        </div>
        <div class="tabs-content">
            <slot />
        </div>
    </div>
</template>

<style lang="scss" scoped>
.tabs-common {
    height: 100%;

    .tabs-nav {
        display: flex;
        position: relative;
        border-bottom: 2px solid var(--border-color);
        margin-bottom: var(--spacing-md);
        flex-shrink: 0;

        .tab-item {
            flex: 1;
            text-align: center;
            cursor: pointer;
            transition: var(--transition-all);
            color: var(--text-secondary);
            font-weight: var(--font-semibold);
            letter-spacing: var(--tracking-wide);
            user-select: none;

            &:hover:not(.tab-disabled) {
                color: var(--primary-color);
            }

            &.tab-active {
                color: var(--primary-color);
            }

            &.tab-disabled {
                opacity: 0.5;
                cursor: not-allowed;
            }
        }

        .tabs-ink-bar {
            position: absolute;
            bottom: -2px;
            height: var(--tabs-height);
            background: var(--primary-color);
            transition: all 0.3s ease;
        }
    }

    // Sizes
    &.tabs-small .tab-item {
        padding: 8px 12px;
        font-size: var(--font-sm);
    }

    &.tabs-medium .tab-item {
        padding: 10px 16px;
        font-size: var(--font-base);
    }

    &.tabs-large .tab-item {
        padding: 12px 20px;
        font-size: var(--font-lg);
    }

    .tabs-content {
        flex: 1;
        display: flex;
        flex-direction: column;
    }

    @include screen(mobile) {
        &.tabs-large .tab-item {
            padding: 8px 12px;
            font-size: var(--font-sm);
        }
    }
}
</style>
