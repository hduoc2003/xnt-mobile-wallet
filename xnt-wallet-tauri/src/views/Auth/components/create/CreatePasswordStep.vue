<script setup lang="ts">
import { ref, computed } from 'vue'
import { z } from 'zod'
import { Eye, EyeOff, Lock, Check, X, Shield, KeyRound } from 'lucide-vue-next'
import { ThemeToggle } from '@/components/ui/theme-toggle'
import { Logo } from '@/components/commons'
import { ArrowLeftCommon } from '@/components/commons'
import { Card, CardContent } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Alert, AlertDescription } from '@/components/ui/alert'

const emit = defineEmits<{
  next: [password: string]
  goToRecover: []
  goToWelcome: []
}>()

// Form state
const password = ref('')
const confirmPassword = ref('')
const showPassword = ref(false)
const showConfirmPassword = ref(false)
const passwordTouched = ref(false)
const confirmPasswordTouched = ref(false)

// Validation schema
const passwordSchema = z
  .string()
  .min(8, 'Password must be at least 8 characters')
  .regex(/[A-Z]/, 'Must contain at least one uppercase letter')
  .regex(/[a-z]/, 'Must contain at least one lowercase letter')
  .regex(/[0-9]/, 'Must contain at least one number')

// Validate password
const passwordError = computed(() => {
  if (!passwordTouched.value || !password.value) return ''
  try {
    passwordSchema.parse(password.value)
    return ''
  } catch (error) {
    if (error instanceof z.ZodError) {
      return error.issues[0]?.message || ''
    }
    return ''
  }
})

const passwordStrength = computed(() => {
  if (!password.value) return { level: 0, text: '', color: '', width: '0%' }

  let strength = 0
  const checks = {
    length: password.value.length >= 8,
    uppercase: /[A-Z]/.test(password.value),
    lowercase: /[a-z]/.test(password.value),
    number: /[0-9]/.test(password.value),
    special: /[!@#$%^&*(),.?":{}|<>]/.test(password.value),
  }

  strength = Object.values(checks).filter(Boolean).length

  if (strength <= 2) return { level: 1, text: 'Weak', color: 'bg-destructive', width: '25%' }
  if (strength <= 3) return { level: 2, text: 'Fair', color: 'bg-yellow-500', width: '50%' }
  if (strength <= 4) return { level: 3, text: 'Good', color: 'bg-blue-500', width: '75%' }
  return { level: 4, text: 'Strong', color: 'bg-green-500', width: '100%' }
})

const isPasswordMatch = computed(() => {
  if (!confirmPassword.value) return true
  return password.value === confirmPassword.value
})

const canProceed = computed(() => {
  return (
    password.value.length >= 8 &&
    confirmPassword.value.length >= 8 &&
    isPasswordMatch.value &&
    passwordStrength.value.level >= 2 &&
    !passwordError.value
  )
})

const handleSubmit = () => {
  passwordTouched.value = true
  confirmPasswordTouched.value = true
  
  if (canProceed.value) {
    emit('next', password.value)
  }
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
            <form id="create-password-form" @submit.prevent="handleSubmit">
              <div class="space-y-6">
                <!-- Password Field -->
                <div class="space-y-2">
                  <Label for="password" class="text-base">Password</Label>
                  <div class="relative">
                    <Lock
                      :size="20"
                      class="absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground"
                    />
                    <Input
                      id="password"
                      v-model="password"
                      :type="showPassword ? 'text' : 'password'"
                      placeholder="Enter your password"
                      class="h-12 pl-11 pr-11 text-base"
                      :class="{ 'border-destructive': passwordError }"
                      @blur="passwordTouched = true"
                    />
                    <div
                      class="absolute right-3 top-1/2 -translate-y-1/2 cursor-pointer text-muted-foreground transition-colors hover:text-foreground"
                      @click="showPassword = !showPassword"
                    >
                      <Eye v-if="!showPassword" :size="20" />
                      <EyeOff v-else :size="20" />
                    </div>
                  </div>

                  <!-- Password Strength -->
                  <div v-if="password" class="space-y-2">
                    <div class="flex items-center gap-2">
                      <div class="h-2 flex-1 overflow-hidden rounded-full bg-muted">
                        <div
                          class="h-full transition-all duration-300"
                          :class="passwordStrength.color"
                          :style="{ width: passwordStrength.width }"
                        />
                      </div>
                      <span
                        class="text-xs font-semibold"
                        :class="`text-${passwordStrength.color.replace('bg-', '')}`"
                      >
                        {{ passwordStrength.text }}
                      </span>
                    </div>
                  </div>

                  <!-- Error Message -->
                  <p v-if="passwordError" class="text-sm text-destructive">
                    {{ passwordError }}
                  </p>
                </div>

                <!-- Confirm Password Field -->
                <div class="space-y-2">
                  <Label for="confirmPassword" class="text-base">Confirm Password</Label>
                  <div class="relative">
                    <KeyRound
                      :size="20"
                      class="absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground"
                    />
                    <Input
                      id="confirmPassword"
                      v-model="confirmPassword"
                      :type="showConfirmPassword ? 'text' : 'password'"
                      placeholder="Re-enter your password"
                      class="h-12 pl-11 pr-11 text-base"
                      :class="{ 'border-destructive': confirmPassword && !isPasswordMatch }"
                      @blur="confirmPasswordTouched = true"
                    />
                    <div
                      class="absolute right-3 top-1/2 -translate-y-1/2 cursor-pointer text-muted-foreground transition-colors hover:text-foreground"
                      @click="showConfirmPassword = !showConfirmPassword"
                    >
                      <Eye v-if="!showConfirmPassword" :size="20" />
                      <EyeOff v-else :size="20" />
                    </div>
                  </div>

                  <!-- Password Match Indicator -->
                  <div v-if="confirmPassword" class="flex items-center gap-2">
                    <Check v-if="isPasswordMatch" :size="16" class="text-green-500" />
                    <X v-else :size="16" class="text-destructive" />
                    <span
                      class="text-sm"
                      :class="isPasswordMatch ? 'text-green-500' : 'text-destructive'"
                    >
                      {{ isPasswordMatch ? 'Passwords match' : 'Passwords do not match' }}
                    </span>
                  </div>
                </div>

                <!-- Security Info -->
                <Alert>
                  <Shield :size="16" />
                  <AlertDescription class="text-xs">
                    Use at least 8 characters with uppercase, lowercase, and numbers for a strong
                    password.
                  </AlertDescription>
                </Alert>

                <!-- Submit Button -->
                <Button
                  type="submit"
                  size="lg"
                  class="h-12 w-full text-base font-semibold"
                  :disabled="!canProceed"
                >
                  Continue
                </Button>
              </div>
            </form>
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
