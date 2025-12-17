<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { AlertTriangle, ChevronLeft } from 'lucide-vue-next'
import { SeedPhraseGrid } from '@/components'
import { useAuthStore } from '@/stores'

const authStore = useAuthStore()
interface Props {
  seedPhrase: string[]
}

const props = defineProps<Props>()

const emit = defineEmits<{
  next: []
  back: []
}>()

const seedWords = computed(() => props.seedPhrase || [])
const isRevealed = ref(false)

const handleNext = () => {
  if (seedWords.value && seedWords.value.length > 0 && isRevealed.value) {
    emit('next')
  }
}

const handleBack = () => {
  emit('back')
}

const handleReveal = () => {
  isRevealed.value = true
}

onMounted(() => {
  authStore.enableBackButton(handleBack)
})

onUnmounted(() => {
  authStore.disableBackButton()
})
</script>

<template>
  <div class="space-y-8">
    <!-- Header -->
    <div class="space-y-3 text-center">
      <h1 class="text-3xl font-bold tracking-tight text-foreground">Secret Seed Phrase</h1>
      <p class="text-base text-muted-foreground">
        Write down these <span class="font-semibold text-primary">18 words</span> in order
      </p>
    </div>

    <Separator class="my-6" />

    <!-- Warning Alert -->
    <Alert variant="destructive" class="border-2">
      <AlertTriangle :size="20" />
      <AlertTitle class="text-base font-semibold">Never share your recovery phrase!</AlertTitle>
      <AlertDescription class="text-sm mt-1">
        Anyone with these words can access your wallet and steal your funds. Store them safely offline.
      </AlertDescription>
    </Alert>

    <!-- Seed Words Grid -->
    <SeedPhraseGrid
      v-if="seedWords && seedWords.length > 0"
      :seed-phrase="seedWords"
      :require-reveal="true"
      @reveal="handleReveal"
    />

    <!-- No Seed Warning -->
    <Alert v-else variant="destructive" class="border-2">
      <AlertTriangle :size="18" />
      <AlertDescription class="text-sm">
        No seed phrase found. Please go back and generate a wallet first.
      </AlertDescription>
    </Alert>

    <!-- Action Buttons -->
    <div class="flex flex-col gap-3 pt-4 md:flex-row">
      <Button
        size="lg"
        class="h-12 w-full text-base font-semibold md:flex-1"
        :disabled="!seedWords || seedWords.length === 0 || !isRevealed"
        @click="handleNext"
      >
        I've Saved It
      </Button>
      <Button
        variant="outline"
        size="lg"
        class="h-12 w-full gap-2 text-base font-medium md:flex-1"
        @click="handleBack"
      >
        <ChevronLeft :size="18" />
        Back
      </Button>
    </div>
  </div>
</template>
