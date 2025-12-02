<script setup lang="ts">
import { computed } from 'vue'
import { message } from 'ant-design-vue'
import { useNeptuneStore } from '@/stores/neptuneStore'

const neptuneStore = useNeptuneStore()

const receiveAddress = computed(() => neptuneStore.getWallet?.address || '')

const copyAddress = async () => {
    if (!receiveAddress.value) {
        message.error('No address available')
        return
    }
    try {
        await navigator.clipboard.writeText(receiveAddress.value)
        message.success('Address copied to clipboard!')
    } catch (err) {
        message.error('Failed to copy address')
    }
}
</script>

<template>
    <div v-if="!receiveAddress" class="empty-state">
        <p>No address available</p>
    </div>

    <div v-else class="receive-section">
        <div class="address-label">Receive Address:</div>
        <div class="address-value" @click="copyAddress">
            <span class="address-text">
                {{ receiveAddress }}
            </span>
            <svg
                class="copy-icon"
                xmlns="http://www.w3.org/2000/svg"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
            >
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
            </svg>
        </div>
    </div>
</template>

<style lang="scss" scoped>
.empty-state {
    text-align: center;
    padding: var(--spacing-lg);
    color: var(--text-secondary);

    p {
        margin: 0;
        font-size: var(--font-base);
    }
}

.receive-section {
    margin-bottom: var(--spacing-lg);
    flex-shrink: 0;

    .address-label {
        font-size: var(--font-base);
        color: var(--text-secondary);
        margin-bottom: var(--spacing-md);
        font-weight: var(--font-semibold);
    }

    .address-value {
        background: var(--bg-light);
        padding: var(--spacing-lg);
        border-radius: var(--radius-md);
        word-break: break-all;
        font-family: var(--font-mono);
        font-size: var(--font-sm);
        color: var(--primary-color);
        cursor: pointer;
        transition: var(--transition-all);
        display: flex;
        align-items: flex-start;
        justify-content: space-between;
        gap: var(--spacing-sm);
        border: 2px solid transparent;
        line-height: 1.5;
        position: relative;
        height: 120px;
        overflow: hidden;

        .address-text {
            flex: 1;
            overflow-y: auto;
            overflow-x: hidden;
            max-height: 100%;
            padding-right: var(--spacing-xs);

            &::-webkit-scrollbar {
                width: 6px;
            }

            &::-webkit-scrollbar-track {
                background: transparent;
            }

            &::-webkit-scrollbar-thumb {
                background: var(--border-color);
                border-radius: 3px;

                &:hover {
                    background: var(--text-muted);
                }
            }
        }

        &:hover {
            background: var(--bg-hover);
            border-color: var(--border-primary);
        }

        .copy-icon {
            width: 18px;
            height: 18px;
            flex-shrink: 0;
            color: var(--primary-color);
            margin-top: 2px;
            align-self: flex-start;
        }
    }
}
</style>
