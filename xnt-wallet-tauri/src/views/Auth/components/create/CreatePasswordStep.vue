<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue';
import { Logo, PasswordCreationForm } from '@/components'
import { useAuthStore } from '@/stores'

const authStore = useAuthStore()  
const emit = defineEmits<{
  next: [password: string]
  goToRecover: []
  goToWelcome: []
}>()

const handlePasswordSubmit = (password: string) => {
  emit('next', password)
}

const handleIHaveWallet = () => {
  emit('goToRecover')
}

const handleGoToWelcome = () => {
  emit('goToWelcome')
}

// onMounted(() => {
//   authStore.enableBackButton(handleGoToWelcome)
// })

// onUnmounted(() => {
//   authStore.disableBackButton()
// })
</script>

<template>
  <div
    class="relative flex h-full flex-col bg-linear-to-br from-background via-background to-primary/5 p-4"
  >
    <!-- Content Container -->
    <div class="flex flex-1 flex-col items-center justify-center">
      <div class="w-full max-w-md space-y-8">
        <!-- Header -->
        <div class="flex flex-col items-center space-y-4 text-center">
          <Logo />

          <div class="space-y-2">
            <h1 class="text-3xl font-bold tracking-tight text-foreground">Create Password</h1>
            <p class="text-base text-muted-foreground">Secure your new wallet</p>
          </div>
        </div>

        <!-- Form -->
        <Card class="border-2 border-border/50 shadow-xl">
          <CardContent class="p-6">
            <PasswordCreationForm
              form-id="create-password-form"
              submit-button-text="Continue"
              @submit="handlePasswordSubmit"
            />
          </CardContent>
        </Card>

        <!-- Footer Link -->
        <div class="text-center">
          <p class="text-sm text-muted-foreground">
            Already have a wallet?
            <Button
              variant="link"
              class="p-0 text-sm font-semibold text-primary"
              @click="handleIHaveWallet"
            >
              Import wallet
            </Button>
          </p>
        </div>
      </div>
    </div>
  </div>
</template>
