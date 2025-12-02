<script setup lang="ts">
import type { BalanceResult } from '@/interface'

interface Props {
    isLoadingData: boolean
    availableBalance: BalanceResult['balance']
    pendingBalance: BalanceResult['pendingBalance']
}

const props = defineProps<Props>()
</script>

<template>
    <div class="balance-section">
        <div class="balance-label">Available</div>
        <div class="balance-amount">
            <span v-if="props.isLoadingData"><SpinnerCommon size="medium" /></span>
            <span v-else>{{ props.availableBalance }} XNT</span>
        </div>
        <div class="pending-section">
            <span class="pending-label">Pending</span>
            <span class="pending-amount">
                {{ props.isLoadingData ? '...' : props.pendingBalance }}
                XNT
            </span>
        </div>
    </div>
</template>

<style lang="scss" scoped>
.balance-section {
    display: flex;
    flex-direction: column;
    align-items: center;
    margin-bottom: var(--spacing-xl);
    padding-bottom: var(--spacing-lg);
    border-bottom: 2px solid var(--border-color);
    flex-shrink: 0;

    .balance-label {
        font-size: var(--font-base);
        font-weight: var(--font-medium);
        margin-bottom: var(--spacing-sm);
        text-transform: uppercase;
        letter-spacing: var(--tracking-wider);
    }

    .balance-amount {
        font-size: var(--font-4xl);
        font-weight: var(--font-semibold);
        color: var(--primary-color);
        margin-bottom: var(--spacing-lg);
        letter-spacing: var(--tracking-tight);
    }

    .pending-section {
        display: flex;
        justify-content: center;
        align-items: center;
        gap: var(--spacing-sm);
        font-size: var(--font-md);

        .pending-label {
            font-weight: var(--font-medium);
        }

        .pending-amount {
            font-weight: var(--font-semibold);
            color: var(--primary-color);
        }
    }
}
</style>
