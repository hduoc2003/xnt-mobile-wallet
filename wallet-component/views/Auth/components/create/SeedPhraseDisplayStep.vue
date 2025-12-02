<script setup lang="ts">
import { computed, ref } from 'vue'
import { Copy, AlertTriangle, ChevronLeft, Eye, CheckCheck } from 'lucide-vue-next'
import { toast } from 'vue-sonner'

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
const isCopied = ref(false)

const handleNext = () => {
  if (seedWords.value && seedWords.value.length > 0) {
    emit('next')
  }
}

const handleBack = () => {
  emit('back')
}

const handleReveal = () => {
  isRevealed.value = true
}

const handleCopySeed = async () => {
  if (!seedWords.value || seedWords.value.length === 0) return

  try {
    const seedPhrase = seedWords.value.join(' ')
    await navigator.clipboard.writeText(seedPhrase)
    isCopied.value = true
    toast.success('Seed phrase copied to clipboard', {
      description: 'You have 5 seconds to copy it to a safe place',
    })
    setTimeout(() => {
      isCopied.value = false
    }, 5000)
  } catch (err) {
    toast.error('Failed to copy seed phrase')
    console.error('Failed to copy seed phrase')
  }
}
</script>

<template>
  <div class="space-y-6">
    <!-- Header -->
    <div class="space-y-2 text-center">
      <h1 class="text-2xl font-bold text-foreground">Secret Recovery Phrase</h1>
      <p class="text-sm text-muted-foreground">
        Write down these <span class="font-semibold text-primary">18 words</span> in order
      </p>
    </div>

    <Separator />

    <!-- Warning Alert -->
    <Alert variant="destructive" class="border-2">
      <AlertTriangle :size="20" />
      <AlertTitle class="text-base">Never share your recovery phrase!</AlertTitle>
      <AlertDescription class="text-sm">
        Anyone with these words can access your wallet and steal your funds. Store them safely offline.
      </AlertDescription>
    </Alert>

    <!-- Seed Words Grid -->
    <div v-if="seedWords.length > 0" class="space-y-4">
      <Card class="border-2 border-primary/20">
        <CardContent class="p-6">
          <div class="relative">
            <!-- Blur overlay when not revealed -->
            <div
              v-if="!isRevealed"
              class="absolute inset-0 z-10 flex flex-col items-center justify-center rounded-lg bg-background/80 backdrop-blur-md"
            >
              <Eye :size="40" class="mb-4 text-muted-foreground" />
              <p class="mb-4 text-center text-sm font-medium text-muted-foreground">
                Make sure no one is watching your screen
              </p>
              <Button size="lg" @click="handleReveal">
                <Eye :size="20" class="mr-2" />
                Reveal Seed Phrase
              </Button>
            </div>

            <!-- Seed words grid -->
            <div class="grid grid-cols-2 gap-3 md:grid-cols-3">
              <div
                v-for="(word, index) in seedWords"
                :key="index"
                class="flex items-center gap-2 rounded-lg border-2 border-border bg-muted/30 px-4 py-3 transition-all hover:border-primary/40"
              >
                <span class="text-xs font-bold text-muted-foreground">{{ index + 1 }}.</span>
                <span class="flex-1 select-all text-base font-semibold text-foreground">{{ word }}</span>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      <!-- Copy Button -->
      <Button
        variant="outline"
        size="lg"
        class="h-12 w-full gap-2"
        :disabled="!isRevealed"
        @click="handleCopySeed"
      >
        <CheckCheck v-if="isCopied" :size="20" class="text-green-500" />
        <Copy v-else :size="20" />
        {{ isCopied ? 'Copied!' : 'Copy to Clipboard' }}
      </Button>
    </div>

    <!-- No Seed Warning -->
    <Alert v-else variant="destructive">
      <AlertTriangle :size="18" />
      <AlertDescription>
        No seed phrase found. Please go back and generate a wallet first.
      </AlertDescription>
    </Alert>

    <!-- Action Buttons -->
    <div class="flex gap-3">
      <Button variant="outline" size="lg" class="flex-1 gap-2" @click="handleBack">
        <ChevronLeft :size="18" />
        Back
      </Button>
      <Button
        size="lg"
        class="flex-1 text-base font-semibold"
        :disabled="!seedWords || seedWords.length === 0 || !isRevealed"
        @click="handleNext"
      >
        I've Saved It
      </Button>
    </div>
  </div>
</template>
