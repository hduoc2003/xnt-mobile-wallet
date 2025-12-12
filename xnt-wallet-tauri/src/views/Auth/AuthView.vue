<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores'
import { WelcomeTab, CreateWalletFlow, RecoverWalletFlow } from './components'
import { useAuthRouting, useNeptuneWallet } from '@/composables'

const authStore = useAuthStore()
const router = useRouter()
const { goToCreate, goToRecover, goBackToWelcome } = useAuthRouting()

const currentState = computed(() => authStore.currentState.value)

onMounted(() => {
  checkKeystore()
})

const handleGoToCreate = () => {
  goToCreate()
}

const handleGoToRecover = () => {
  goToRecover()
}

const handleAccessWallet = () => {
  router.push({ name: 'wallet' })
}

const handleGoToWelcome = () => {
  goBackToWelcome()
}

const checkKeystore = async () => {
  const neptuneWallet = useNeptuneWallet()
  await neptuneWallet.checkKeystore()
}
</script>

<template>
  <div class="min-h-screen">
    <!-- Welcome: Welcome screen with login -->
    <WelcomeTab
      v-if="currentState === 'welcome'"
      @go-to-create="handleGoToCreate"
      @go-to-recover="handleGoToRecover"
      @go-to-welcome="handleGoToWelcome"
    />

    <!-- Create: New wallet creation flow -->
    <CreateWalletFlow
      v-else-if="currentState === 'create'"
      @go-to-recover="handleGoToRecover"
      @access-wallet="handleAccessWallet"
      @go-to-welcome="handleGoToWelcome"
    />

    <!-- Recovery: Recover wallet from seed phrase -->
    <RecoverWalletFlow
      v-else-if="currentState === 'recovery'"
      @cancel="handleGoToWelcome"
      @access-wallet="handleAccessWallet"
    />
  </div>
</template>
