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
  let historyError: string | undefined
  try {
    isLoading.value = true
    ;({ historyError } = await decryptKeystore(password))
  } catch (err: unknown) {
    toast.error('Unlock Failed', {
      description: err instanceof Error ? err.message : String(err),
    })
    isLoading.value = false
    return
  }
  if (historyError !== undefined) {
    toast.error('Could not move the History to Account 1', { description: historyError })
  }
  try {
    loadTransactionsFromStorage()
  } catch (err: unknown) {
    toast.error('Could not read History', {
      description: err instanceof Error ? err.message : String(err),
    })
  }
  isLoading.value = false
  router.push({ name: 'wallet' })
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
