<script setup lang="ts">
import { ref, computed } from 'vue'
import { z } from 'zod'
import { ChevronLeft, Eye, EyeOff, Lock, Loader2, KeyRound, Shield } from 'lucide-vue-next'
import { ThemeToggle } from '@/components/ui/theme-toggle'
import RecoverSeedInput from './RecoverSeedInput.vue'

const emit = defineEmits<{
  cancel: []
  accessWallet: []
}>()

// TODO: Import useNeptuneWallet composable
// const { recoverWalletFromSeed, createKeystore } = useNeptuneWallet()

const recoverSeedInputRef = ref<InstanceType<typeof RecoverSeedInput>>()
const showPasswordForm = ref(false)
const seedPhrase = ref<string[]>([])
const showPassword = ref(false)
const showConfirmPassword = ref(false)
const isLoading = ref(false)
const isSeedPhraseValid = ref(true)

// Form state
const password = ref('')
const confirmPassword = ref('')
const passwordTouched = ref(false)
const confirmPasswordTouched = ref(false)

// Validation schema
const passwordSchema = z
  .string()
  .min(8, 'Password must be at least 8 characters')

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

const isPasswordMatch = computed(() => {
  if (!confirmPassword.value) return true
  return password.value === confirmPassword.value
})

const canSubmit = computed(() => {
  return (
    password.value.length >= 8 &&
    confirmPassword.value.length >= 8 &&
    isPasswordMatch.value &&
    !passwordError.value
  )
})

const handleSeedWordsUpdate = (words: string[]) => {
  seedPhrase.value = words
}

const handleSeedPhraseSubmit = async (words: string[]) => {
  seedPhrase.value = words
  showPasswordForm.value = true
}

const handleContinue = () => {
  recoverSeedInputRef.value?.handleSubmit?.()
}

const handlePasswordBack = () => {
  showPasswordForm.value = false
  password.value = ''
  confirmPassword.value = ''
  passwordTouched.value = false
  confirmPasswordTouched.value = false
}

const handleCancel = () => {
  emit('cancel')
  showPasswordForm.value = false
  seedPhrase.value = []
  password.value = ''
  confirmPassword.value = ''
  passwordTouched.value = false
  confirmPasswordTouched.value = false
  isSeedPhraseValid.value = true
}

const handleSubmit = async () => {
  passwordTouched.value = true
  confirmPasswordTouched.value = true

  if (!canSubmit.value) return

  try {
    isLoading.value = true
    // TODO: Recover wallet from seed
    // const result = await recoverWalletFromSeed(seedPhrase.value)
    // if (result.address) {
    //   await createKeystore(seedPhrase.value.join(' '), password.value)
    //   emit('accessWallet')
    // }

    // Mock success for now
    emit('accessWallet')
  } catch (err) {
    if (err instanceof Error && err.message.includes('Invalid seed phrase')) {
      isSeedPhraseValid.value = false
    }
    console.error('Failed to recover wallet:', err)
  } finally {
    isLoading.value = false
  }
}
</script>

<template>
  <div class="relative flex min-h-screen flex-col bg-linear-to-br from-background via-background to-primary/5 p-4">
    <!-- Theme Toggle -->
    <div class="absolute right-4 top-4 z-10">
      <ThemeToggle />
    </div>

    <!-- Back Button -->
    <div class="absolute left-4 top-4 z-10">
      <ArrowLeftCommon @click="handleCancel" />
    </div>

    <!-- Content Container -->
    <div class="flex flex-1 flex-col items-center justify-center">
      <div class="w-full max-w-md space-y-8">
        <!-- Header -->
        <div class="flex flex-col items-center space-y-4 text-center">
          <div class="flex h-16 w-16 items-center justify-center rounded-2xl bg-linear-to-br from-primary to-accent shadow-xl">
            <KeyRound :size="32" class="text-primary-foreground" />
          </div>

          <div class="space-y-2">
            <h1 class="text-3xl font-bold tracking-tight text-foreground">
              {{ showPasswordForm ? 'Set Password' : 'Import Wallet' }}
            </h1>
            <p class="text-base text-muted-foreground">
              {{ showPasswordForm ? 'Secure your recovered wallet' : 'Enter your 18-word recovery phrase' }}
            </p>
          </div>
        </div>

        <!-- Form Card -->
        <Card class="border-2 border-border/50 shadow-xl">
          <CardContent class="p-6">
            <!-- Seed Phrase Step -->
            <div v-if="!showPasswordForm" class="space-y-6">
              <RecoverSeedInput
                ref="recoverSeedInputRef"
                :valid="isSeedPhraseValid"
                @submit="handleSeedPhraseSubmit"
                @update:words="handleSeedWordsUpdate"
                @update:valid="isSeedPhraseValid = $event"
              />

              <!-- Action Buttons -->
              <div class="flex gap-3">
                <Button
                  variant="outline"
                  size="lg"
                  class="h-12 flex-1 gap-2"
                  @click="handleCancel"
                >
                  <ChevronLeft :size="18" />
                  Cancel
                </Button>
                <Button
                  size="lg"
                  class="h-12 flex-1 text-base font-semibold"
                  :disabled="isLoading || seedPhrase.length !== 18"
                  @click="handleContinue"
                >
                  Continue
                </Button>
              </div>
            </div>

            <!-- Password Step -->
            <form v-else id="recover-password-form" @submit.prevent="handleSubmit">
              <div class="space-y-6">
                <!-- Info Alert -->
                <Alert class="border-2 border-primary/20 bg-primary/5">
                  <Shield :size="16" />
                  <AlertDescription class="text-sm">
                    Create a password to encrypt and secure your wallet locally.
                  </AlertDescription>
                </Alert>

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
                      placeholder="Create a strong password"
                      class="h-12 pl-11 pr-11 text-base"
                      :class="{ 'border-destructive': passwordError }"
                      @blur="passwordTouched = true"
                    />
                    <button
                      type="button"
                      class="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground transition-colors hover:text-foreground"
                      @click="showPassword = !showPassword"
                    >
                      <Eye v-if="!showPassword" :size="20" />
                      <EyeOff v-else :size="20" />
                    </button>
                  </div>
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
                    <button
                      type="button"
                      class="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground transition-colors hover:text-foreground"
                      @click="showConfirmPassword = !showConfirmPassword"
                    >
                      <Eye v-if="!showConfirmPassword" :size="20" />
                      <EyeOff v-else :size="20" />
                    </button>
                  </div>
                  <p v-if="confirmPassword && !isPasswordMatch" class="text-sm text-destructive">
                    Passwords do not match
                  </p>
                </div>

                <!-- Action Buttons -->
                <div class="flex gap-3">
                  <Button
                    type="button"
                    variant="outline"
                    size="lg"
                    class="h-12 flex-1 gap-2"
                    :disabled="isLoading"
                    @click="handlePasswordBack"
                  >
                    <ChevronLeft :size="18" />
                    Back
                  </Button>
                  <Button
                    type="submit"
                    size="lg"
                    class="h-12 flex-1 text-base font-semibold"
                    :disabled="!canSubmit || isLoading"
                  >
                    <Loader2 v-if="isLoading" :size="20" class="mr-2 animate-spin" />
                    {{ isLoading ? 'Importing...' : 'Import Wallet' }}
                  </Button>
                </div>
              </div>
            </form>
          </CardContent>
        </Card>

        <!-- Footer Info -->
        <div v-if="!showPasswordForm" class="rounded-xl border border-border/50 bg-muted/30 p-4 backdrop-blur-sm">
          <p class="text-center text-xs text-muted-foreground">
            <Shield :size="14" class="mr-1 inline" />
            Your recovery phrase is never shared and stays on your device
          </p>
        </div>
      </div>
    </div>
  </div>
</template>
