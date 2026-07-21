<script setup lang="ts">
import { computed, ref } from 'vue'
import { OnboardingTab, LoginTab } from '@/views/Auth/components'
import { useNeptuneStore } from '@/stores'
import { useNeptuneWallet, useTransaction } from '@/composables'
import { useRouter } from 'vue-router'
import { toast } from 'vue-sonner'

const neptuneStore = useNeptuneStore()
const router = useRouter()
const { decryptKeystore } = useNeptuneWallet()
const { loadTransactionsFromStorage } = useTransaction()

const emit = defineEmits<{
  goToCreate: []
  goToRecover: []
  goToWelcome: []
}>()

const showLogin = computed(() => neptuneStore.getKeystoreFileName !== null)
const isLoading = ref(false)

const handleGoToCreate = () => {
  emit('goToCreate')
}

const handleGoToRecover = () => {
  emit('goToRecover')
}

const handleSubmitPassword = async (password: string) => {
  try {
    isLoading.value = true
    await decryptKeystore(password)
    loadTransactionsFromStorage()
    router.push({ name: 'wallet' })
  } catch (err: any) {
    const message = 'Failed to unlock wallet'
    toast.error('Unlock Failed', {
      description: message,
    })
  } finally {
    isLoading.value = false
  }
}
</script>

<template>
  <div>
    <!-- Onboarding Screen -->
    <OnboardingTab
      v-if="!showLogin"
      @go-to-create="handleGoToCreate"
      @go-to-recover="handleGoToRecover"
    />

    <!-- Login Screen -->
    <LoginTab
      v-else
      :loading="isLoading"
      @go-to-create="handleGoToCreate"
      @submit="handleSubmitPassword"
    />
  </div>
</template>
