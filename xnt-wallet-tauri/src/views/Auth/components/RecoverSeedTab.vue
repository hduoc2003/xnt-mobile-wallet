<script setup lang="ts">
import { RecoverWalletComponent } from '.'
import { useAuthStore } from '@/stores'
import { useRouter } from 'vue-router'

const authStore = useAuthStore()
const router = useRouter()

const handleCancel = () => {
    authStore.goBack()
}

const handleAccessWallet = () => {
    router.push({ name: 'home' })
}
</script>

<template>
    <div class="recover-seed-tab">
        <RecoverWalletComponent @cancel="handleCancel" @access-wallet="handleAccessWallet" />
    </div>
</template>

<style lang="scss" scoped>
.recover-seed-tab {
    padding: var(--spacing-lg);
}
</style>
