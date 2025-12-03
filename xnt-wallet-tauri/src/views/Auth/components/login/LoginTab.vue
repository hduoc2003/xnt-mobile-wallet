<script setup lang="ts">
import { ref, computed } from 'vue'
import { ThemeToggle } from '@/components/ui/theme-toggle'
import { PasswordForm, Logo } from '@/components/commons'
import { Card, CardContent } from '@/components/ui/card'
import { Button } from '@/components/ui/button'

const props = withDefaults(defineProps<{
  loading?: boolean
}>(), {
  loading: false
})

const emit = defineEmits<{
  goToCreate: []
  submit: [password: string]
}>()

const isLoading = computed(() => props.loading)
const error = ref(false)

const handleSubmit = async (password: string) => {
  try {
    error.value = false
    emit('submit', password)
  } catch (err) {
    error.value = true
  }
}

const handleNewWallet = () => {
  emit('goToCreate')
}
</script>

<template>
  <div class="relative flex min-h-screen flex-col bg-gradient-to-br from-background via-background to-primary/5 p-4">
    <!-- Theme Toggle -->
    <div class="absolute right-4 top-4 z-10">
      <ThemeToggle />
    </div>

    <!-- Content Container -->
    <div class="flex flex-1 flex-col items-center justify-center">
      <div class="w-full max-w-md space-y-8">
        <!-- Header -->
        <div class="flex flex-col items-center space-y-4 text-center">
          <Logo />

          <div class="space-y-2">
            <h1 class="text-3xl font-bold tracking-tight text-foreground">
              Welcome Back
            </h1>
            <p class="text-base text-muted-foreground">
              Enter your password to unlock
            </p>
          </div>
        </div>

        <!-- Form -->
        <Card class="border-2 border-border/50 shadow-xl">
          <CardContent class="p-6">
            <PasswordForm
              label="Password"
              placeholder="Enter your password"
              button-text="Unlock Wallet"
              :loading="isLoading"
              :error="error"
              error-message="Incorrect password. Please try again."
              autocomplete="current-password"
              @submit="handleSubmit"
            />
          </CardContent>
        </Card>

        <!-- Footer Link -->
        <div class="text-center">
          <p class="text-sm text-muted-foreground">
            Don't have a wallet?
            <Button variant="link" class="p-0 text-sm font-semibold text-primary" @click="handleNewWallet">
              Create one now
            </Button>
          </p>
        </div>
      </div>
    </div>
  </div>
</template>
