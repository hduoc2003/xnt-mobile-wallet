<script setup lang="ts">
import { CreateWalletComponent } from '.'

const emit = defineEmits<{
    goToRecover: []
}>()

const handleNavigateToRecoverWallet = () => {
    emit('goToRecover')
}
</script>

<template>
    <div class="create-tab">
        <CreateWalletComponent @navigate-to-recover-wallet="handleNavigateToRecoverWallet" />
    </div>
</template>

<style lang="scss" scoped>
.create-tab {
    padding: var(--spacing-lg);
}
</style>
