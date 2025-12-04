<script setup lang="ts">
import { ThemeToggle } from '@/components/ui/theme-toggle'
import { Logo, PasswordCreationForm } from '@/components/commons'
import { ArrowLeftCommon } from '@/components/commons'
import { Card, CardContent } from '@/components/ui/card'
import { Button } from '@/components/ui/button'

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
</script>

<template>
  <div
    class="relative flex min-h-screen flex-col bg-linear-to-br from-background via-background to-primary/5 p-4"
  >
    <!-- Theme Toggle -->
    <div class="absolute right-4 top-4 z-10">
      <ThemeToggle />
    </div>

    <!-- Back Button -->
    <div class="absolute left-4 top-4 z-10">
      <ArrowLeftCommon @click="handleGoToWelcome" />
    </div>

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
