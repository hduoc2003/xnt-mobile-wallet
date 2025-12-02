<script setup lang="ts">
import { computed } from 'vue'
import { OnboardingTab, LoginTab } from '@/views/Auth/components'
import { useNeptuneStore } from '@/stores'

const neptuneStore = useNeptuneStore()

const emit = defineEmits<{
  goToCreate: []
  goToRecover: []
}>()

// Internal state to switch between onboarding and login
const showLogin = computed(() => neptuneStore.getKeystoreFileName !== null)
const handleGoToCreate = () => {
  emit('goToCreate')
}

const handleGoToRecover = () => {
  emit('goToRecover')
}

const handleSubmit = (password: string) => {
  
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
      @go-to-create="handleGoToCreate"
      @submit="handleSubmit"
    />
  </div>
</template>
