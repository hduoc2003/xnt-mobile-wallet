<script setup lang="ts">
import { ref, computed, onMounted, inject, watch, type ComputedRef } from 'vue'
import { Table, message } from 'ant-design-vue'
import { useNeptuneWallet } from '@/composables/useNeptuneWallet'
import { useNeptuneStore } from '@/stores/neptuneStore'
import { columns } from '../utils'

const { getUtxos } = useNeptuneWallet()
const neptuneStore = useNeptuneStore()
const activeTabKey = inject<ComputedRef<string>>('activeTabKey')

const loading = ref(false)

const utxosList = computed(() => [
    ...(neptuneStore.getUtxos || []),
])

const inUseUtxosCount = computed(() => (utxosList.value?.length ? utxosList.value.length : 0))
const inUseUtxosAmount = computed(() => {
    if (!utxosList.value?.length) return '0.00000000'

    const total = utxosList.value.reduce((total: number, utxo: any) => {
        const amount = parseFloat(utxo.amount || utxo.value || 0)
        return total + amount
    }, 0)

    return total.toFixed(8)
})

const loadUtxos = async () => {
    try {
        loading.value = true
        const result = await getUtxos()
        if (result.error) {
            message.error('Failed to load UTXOs')
            loading.value = false
            return
        }
        loading.value = false
    } catch (err) {
        message.error('Failed to load UTXOs')
    } finally {
        loading.value = false
    }
}

onMounted(() => {
    loadUtxos()
})

watch(
    () => activeTabKey?.value,
    (newTab) => {
        if (newTab === 'UTXOs') loadUtxos()
    }
)
</script>

<template>
    <CardBaseScrollable class="content-card debug-card">
        <div class="debug-header">
            <h3 class="debug-title">IN USE UTXOS</h3>
            <div class="debug-info">
                <p><span>COUNT</span> <strong>{{ inUseUtxosCount }}</strong></p>
                <p><span>AMOUNT</span> <strong>{{ inUseUtxosAmount }}</strong> <strong>XNT</strong></p>
            </div>
        </div>

        <div v-if="loading" class="loading-container">
            <SpinnerCommon />
        </div>

        <div v-else class="list-pagination">
            <Table
                :columns="columns"
                :data-source="utxosList"
                :scroll="{ x: 'max-content', y: '200px' }"
            />
        </div>
    </CardBaseScrollable>
</template>

<style lang="scss" scoped>
.debug-card {
    .debug-header {
        text-align: center;
        margin-bottom: var(--spacing-2xl);
        padding-bottom: var(--spacing-sm);
        border-bottom: 2px solid var(--border-color);

        .debug-title {
            font-size: var(--font-2xl);
            font-weight: var(--font-bold);
            color: var(--text-primary);
            margin-bottom: var(--spacing-xl);
            letter-spacing: var(--tracking-wide);
            display: flex;
            align-items: center;
            justify-content: center;
        }

        .debug-info {
            display: flex;
            justify-content: flex-start;
            gap: var(--spacing-4xl);

            p {
                margin: 0;
                font-size: var(--font-lg);
                color: var(--text-secondary);

                span {
                    color: var(--text-primary);
                    font-weight: var(--font-bold);
                    margin-right: var(--spacing-sm);
                }

                strong {
                    color: var(--primary-color);
                }
            }
        }
    }

    .loading-container {
        display: flex;
        justify-content: center;
        align-items: center;
        min-height: 300px;
    }

    .list-pagination {
        :deep(.ant-table) {
            background: transparent;

            .ant-table-thead > tr > th {
                background: var(--bg-light);
                font-weight: var(--font-semibold);
                color: var(--text-primary);
                border-bottom: 2px solid var(--border-color);
            }

            .ant-table-tbody > tr > td {
                border-bottom: 1px solid var(--border-light);
            }

            .ant-table-tbody > tr:hover > td {
                background: var(--bg-hover);
            }
        }

        :deep(.ant-pagination) {
            margin-top: var(--spacing-lg);
            text-align: center;
        }
    }
}
</style>
