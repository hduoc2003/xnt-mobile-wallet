<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { formatNumberToLocaleString } from '@/utils'
import { useNeptuneStore } from '@/stores/neptuneStore'
import { useNeptuneWallet } from '@/composables/useNeptuneWallet'
import { message } from 'ant-design-vue'
import { POLLING_INTERVAL } from '@/utils'

const neptuneStore = useNeptuneStore()
const { getBlockHeight } = useNeptuneWallet()

const blockHeight = ref(0)
const loading = ref(true)
const error = ref('')
const lastUpdate = ref<Date | null>(null)

let pollingInterval: number | null = null

const network = computed(() => neptuneStore.getNetwork)

const loadNetworkData = async () => {
    try {
        loading.value = true
        error.value = ''
        const result = await getBlockHeight()
        if (result.height || result) blockHeight.value = Number(result.height || result)

        lastUpdate.value = new Date()
        loading.value = false
    } catch (err) {
        const errorMsg = err instanceof Error ? err.message : 'Failed to load network data'
        error.value = errorMsg
        loading.value = false
        message.error(errorMsg)
    }
}

const retryConnection = async () => {
    loading.value = true
    error.value = ''
    await loadNetworkData()
}

const startPolling = () => {
    pollingInterval = window.setInterval(async () => {
        if (!loading.value) await loadNetworkData()
    }, POLLING_INTERVAL)
}

const stopPolling = () => {
    if (pollingInterval) {
        clearInterval(pollingInterval)
        pollingInterval = null
    }
}

onMounted(async () => {
    await loadNetworkData()
    startPolling()
})

onUnmounted(() => {
    stopPolling()
})
</script>

<template>
    <CardBase class="content-card">
        <div class="network-status-container">
            <h2 class="section-title">NETWORK STATUS</h2>

            <!-- Loading State -->
            <div v-if="loading && !blockHeight" class="loading-state">
                <SpinnerCommon size="medium" />
                <p>Loading network data...</p>
            </div>

            <!-- Error State -->
            <div v-else-if="error" class="error-state">
                <p>{{ error }}</p>
                <button @click="retryConnection" class="retry-button">Retry Connection</button>
            </div>

            <!-- Data Display -->
            <div v-else class="status-grid">
                <div class="status-item">
                    <span class="status-label">Network</span>
                    <span class="status-value">XNT {{ network }}</span>
                </div>

                <div class="status-item">
                    <span class="status-label">Block Height</span>
                    <span class="status-value">{{ formatNumberToLocaleString(blockHeight) }}</span>
                </div>

                <div v-if="lastUpdate" class="status-item">
                    <span class="status-label">Last Updated</span>
                    <span class="status-value">{{ lastUpdate.toLocaleTimeString() }}</span>
                </div>
            </div>
        </div>
    </CardBase>
</template>

<style lang="scss" scoped>
.network-status-container {
    .section-title {
        font-size: var(--font-xl);
        font-weight: var(--font-bold);
        color: var(--text-primary);
        margin-bottom: var(--spacing-2xl);
        letter-spacing: var(--tracking-wider);
        text-align: center;
        padding-bottom: var(--spacing-lg);
        border-bottom: 3px solid var(--primary-color);
    }

    .status-grid {
        display: flex;
        flex-direction: column;
        gap: var(--spacing-xl);
    }

    .status-item {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: var(--spacing-lg) var(--spacing-xl);
        background: var(--bg-light);
        border-radius: var(--radius-md);
        transition: var(--transition-all);

        &:hover {
            background: var(--bg-hover);
            transform: translateX(5px);
        }

        .status-label {
            font-weight: var(--font-semibold);
            color: var(--text-secondary);
            font-size: var(--font-md);
        }

        .status-value {
            font-weight: var(--font-semibold);
            color: var(--text-primary);
            font-size: var(--font-lg);
            text-align: right;
        }
    }

    // Loading State
    .loading-state {
        @include center_flex;
        padding: var(--spacing-4xl);
        color: var(--text-secondary);
    }

    // Error State
    .error-state {
        @include center_flex;
        padding: var(--spacing-4xl);
        color: var(--error-color);

        p {
            margin-bottom: var(--spacing-lg);
            font-size: var(--font-md);
        }

        .retry-button {
            padding: var(--spacing-sm) var(--spacing-lg);
            background: var(--primary-color);
            color: var(--text-light);
            border: none;
            border-radius: var(--radius-md);
            cursor: pointer;
            font-size: var(--font-sm);
            transition: var(--transition-all);

            &:hover {
                background: var(--primary-hover);
            }
        }
    }
}
</style>
