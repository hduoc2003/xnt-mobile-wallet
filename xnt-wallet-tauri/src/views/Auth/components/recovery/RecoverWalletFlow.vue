<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { ChevronLeft, KeyRound, Shield } from 'lucide-vue-next'
import { toast } from 'vue-sonner'
import RecoverSeedInput from './RecoverSeedInput.vue'
import { PasswordCreationForm } from '@/components'
import { useNeptuneWallet, useTransaction } from '@/composables'
import { useAuthStore } from '@/stores'

const emit = defineEmits<{
  cancel: []
  accessWallet: []
}>()

const { recoverWalletFromSeed, createKeystore } = useNeptuneWallet()
const { clearTransactions } = useTransaction()

const authStore = useAuthStore()
const recoverSeedInputRef = ref<InstanceType<typeof RecoverSeedInput>>()
const passwordFormRef = ref<InstanceType<typeof PasswordCreationForm>>()
const showPasswordForm = ref(false)
const seedPhrase = ref<string[]>([])
const isLoading = ref(false)
const isSeedPhraseValid = ref(true)
const password = ref('')

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

const resetFlow = () => {
  showPasswordForm.value = false
  seedPhrase.value = []
  password.value = ''
  passwordFormRef.value?.reset()
  isSeedPhraseValid.value = true
}

const handleCancel = () => {
  emit('cancel')
  resetFlow()
}

const handlePasswordSubmit = async (submittedPassword: string) => {
  password.value = submittedPassword

  try {
    isLoading.value = true

    const result = await recoverWalletFromSeed(seedPhrase.value)

    if (!result.address) {
      throw new Error('Failed to recover wallet: No address returned')
    }

    const keystoreFileName = await createKeystore(seedPhrase.value.join(' '), password.value, true)

    if (!keystoreFileName) {
      throw new Error('Failed to create keystore')
    }

    clearTransactions()
    emit('accessWallet')
  } catch (err) {
    const errorMessage = err instanceof Error ? err.message : String(err)

    if (err instanceof Error && err.message.includes('Invalid seed phrase')) {
      isSeedPhraseValid.value = false
      toast.error('Invalid Seed Phrase', {
        description: 'The seed phrase you entered is invalid. Please check and try again.',
      })
    } else {
      toast.error('Import Failed', {
        description: errorMessage || 'Failed to import wallet. Please try again.',
      })
    }
  } finally {
    isLoading.value = false
  }
}

onMounted(() => {
  authStore.enableBackButton(handleCancel)
})

onUnmounted(() => {
  authStore.disableBackButton()
})
</script>

<template>
  <div
    class="relative flex min-h-full flex-col bg-linear-to-br from-background via-background to-primary/5 p-4"
  >
    <!-- Content Container -->
    <div class="flex flex-1 flex-col items-center justify-center py-4">
      <div class="w-full max-w-md space-y-8">
        <!-- Header -->
        <div class="flex flex-col items-center space-y-4 text-center">
          <div
            class="flex h-16 w-16 items-center justify-center rounded-2xl bg-linear-to-br from-primary to-accent shadow-xl"
          >
            <KeyRound :size="32" class="text-primary-foreground" />
          </div>

          <div class="space-y-2">
            <h1 class="text-3xl font-bold tracking-tight text-foreground">
              {{ showPasswordForm ? 'Set Password' : 'Import Wallet' }}
            </h1>
            <p class="text-base text-muted-foreground">
              {{
                showPasswordForm
                  ? 'Secure your recovered wallet'
                  : 'Enter your 18-word recovery phrase'
              }}
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
                <Button variant="outline" size="lg" class="h-12 flex-1 gap-2" @click="handleCancel">
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
            <div v-else class="space-y-6">
              <!-- Info Alert -->
              <Alert class="border-2 border-primary/20 bg-primary/5">
                <Shield :size="16" />
                <AlertDescription class="text-sm">
                  Create a password to encrypt and secure your wallet locally.
                </AlertDescription>
              </Alert>

              <PasswordCreationForm
                ref="passwordFormRef"
                form-id="recover-password-form"
                password-placeholder="Create a strong password"
                submit-button-text="Import Wallet"
                loading-text="Importing..."
                :loading="isLoading"
                :show-security-info="false"
                @submit="handlePasswordSubmit"
              />
            </div>
          </CardContent>
        </Card>

        <!-- Footer Info -->
        <div
          v-if="!showPasswordForm"
          class="rounded-xl border border-border/50 bg-muted/30 p-4 backdrop-blur-sm"
        >
          <p class="text-center text-xs text-muted-foreground">
            <Shield :size="14" class="mr-1 inline" />
            Your recovery phrase is never shared and stays on your device
          </p>
        </div>
      </div>
    </div>
  </div>
</template>
