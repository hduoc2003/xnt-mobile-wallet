<script setup lang="ts">
import { ref } from 'vue'
import { toast } from 'vue-sonner'
import CreatePasswordStep from './CreatePasswordStep.vue'
import SeedPhraseDisplayStep from './SeedPhraseDisplayStep.vue'
import ConfirmSeedStep from './ConfirmSeedStep.vue'
import WalletCreatedStep from './WalletCreatedStep.vue'
import { useNeptuneWallet, useTransaction } from '@/composables'

const emit = defineEmits<{
  goToRecover: []
  accessWallet: []
  goToWelcome: []
}>()

const { generateWallet, createKeystore } = useNeptuneWallet()
const { clearTransactions } = useTransaction()

const step = ref(1)
const seedPhrase = ref<string[]>([])
const password = ref('')
const isLoading = ref(false)

const handleGoToRecover = () => {
  emit('goToRecover')
}

const handleGoToWelcome = () => {
  emit('goToWelcome')
}

const handleNextFromPassword = async (pwd: string) => {
  try {
    isLoading.value = true

    const result = await generateWallet()
    seedPhrase.value = result.seed_phrase

    password.value = pwd
    step.value = 2
  } catch (err) {
    console.error('Failed to generate wallet:', err)
    const message = err instanceof Error ? err.message : 'Failed to generate wallet'
    toast.error('Wallet Generation Failed', {
      description: message,
    })
  } finally {
    isLoading.value = false
  }
}

const handleBackToPassword = () => {
  step.value = 1
}

const handleNextToConfirm = () => {
  step.value = 3
}

const handleBackToSeedDisplay = () => {
  step.value = 2
}

const handleNextToSuccess = () => {
  step.value = 4
}

const handleAccessWallet = async () => {
  try {
    isLoading.value = true

    const seedPhraseString = seedPhrase.value.join(' ')
    await createKeystore(seedPhraseString, password.value, false)
    clearTransactions()

    emit('accessWallet')
  } catch (err) {
    console.error('Failed to create keystore:', err)
    const message = err instanceof Error ? err.message : 'Failed to create keystore'
    toast.error('Keystore Creation Failed', {
      description: message,
    })
  } finally {
    isLoading.value = false
  }
}
</script>

<template>
  <div>
    <!-- Step 1: Create Password -->
    <CreatePasswordStep
      v-if="step === 1"
      @next="handleNextFromPassword"
      @go-to-recover="handleGoToRecover"
      @go-to-welcome="handleGoToWelcome"
    />

    <!-- Step 2: Display Seed Phrase -->
    <div
      v-else-if="step === 2"
      class="flex min-h-screen items-center justify-center bg-linear-to-br from-background via-background to-primary/5 p-4"
    >
      <SeedPhraseDisplayStep
        :seed-phrase="seedPhrase"
        @back="handleBackToPassword"
        @next="handleNextToConfirm"
      />
    </div>

    <!-- Step 3: Confirm Seed Phrase -->
    <div
      v-else-if="step === 3"
      class="flex min-h-screen items-center justify-center bg-linear-to-br from-background via-background to-primary/5 p-4"
    >
      <Card class="w-full max-w-2xl border-2 border-border/50 shadow-xl">
        <CardContent class="p-6 md:p-8">
          <ConfirmSeedStep
            :seed-phrase="seedPhrase"
            @back="handleBackToSeedDisplay"
            @next="handleNextToSuccess"
          />
        </CardContent>
      </Card>
    </div>

    <!-- Step 4: Wallet Created Success -->
    <div
      v-else-if="step === 4"
      class="flex min-h-screen items-center justify-center bg-linear-to-br from-background via-background to-primary/5 p-4"
    >
      <Card class="w-full max-w-2xl border-2 border-border/50 shadow-xl">
        <CardContent class="p-6 md:p-8">
          <WalletCreatedStep
            :seed-phrase="seedPhrase"
            :password="password"
            @access-wallet="handleAccessWallet"
          />
        </CardContent>
      </Card>
    </div>
  </div>
</template>
