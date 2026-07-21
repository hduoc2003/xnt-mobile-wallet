<script setup lang="ts">
import { ref } from 'vue'
import { Copy, Eye, CheckCheck } from 'lucide-vue-next'
import { toast } from 'vue-sonner'
import { Card, CardContent } from '@/components/ui/card'
import { Button } from '@/components/ui/button'

interface Props {
  seedPhrase: string[]
  requireReveal?: boolean
  revealMessage?: string
}

const props = withDefaults(defineProps<Props>(), {
  requireReveal: true,
  revealMessage: 'Make sure no one is watching your screen',
})

const emit = defineEmits<{
  reveal: []
}>()

const isRevealed = ref(!props.requireReveal)
const isCopied = ref(false)

const handleReveal = () => {
  isRevealed.value = true
  emit('reveal')
}

const handleCopySeed = async () => {
  if (!props.seedPhrase || props.seedPhrase.length === 0) return

  try {
    const seedPhrase = props.seedPhrase.join(' ')
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

defineExpose({
  isRevealed: () => isRevealed.value,
})
</script>

<template>
  <div v-if="seedPhrase && seedPhrase.length > 0" class="space-y-4">
    <Card class="border-2 border-primary/20 shadow-lg">
      <CardContent class="p-6 md:p-8">
        <div class="relative min-h-[400px]">
          <!-- Blur overlay when not revealed -->
          <div
            v-if="!isRevealed"
            class="absolute inset-0 z-10 flex flex-col items-center justify-center rounded-lg bg-background/95 backdrop-blur-lg p-6"
          >
            <div class="flex flex-col items-center space-y-4 text-center">
              <div class="rounded-full bg-primary/10 p-4">
                <Eye :size="48" class="text-primary" />
              </div>
              <p class="max-w-sm text-center text-sm font-medium text-muted-foreground">
                {{ revealMessage }}
              </p>
              <Button
                size="lg"
                class="mt-2 h-12 px-8 text-base font-semibold"
                @click="handleReveal"
              >
                <Eye :size="20" class="mr-2" />
                Reveal Seed Phrase
              </Button>
            </div>
          </div>

          <!-- Seed words grid -->
          <div class="grid grid-cols-2 gap-3 md:grid-cols-3">
            <div
              v-for="(word, index) in seedPhrase"
              :key="index"
              class="flex items-center gap-2 rounded-lg border-2 border-border bg-muted/30 px-4 py-3 transition-all hover:border-primary/50 hover:bg-muted/50"
            >
              <span class="shrink-0 text-xs font-bold text-muted-foreground">{{ index + 1 }}.</span>
              <span
                class="min-w-0 flex-1 select-all wrap-break-words text-base font-semibold text-foreground"
              >
                {{ word }}
              </span>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>

    <!-- Copy Button -->
    <Button
      variant="outline"
      size="lg"
      class="h-12 w-full gap-2 text-base font-medium"
      :disabled="!isRevealed"
      @click="handleCopySeed"
    >
      <CheckCheck v-if="isCopied" :size="20" class="text-green-500" />
      <Copy v-else :size="20" />
      {{ isCopied ? 'Copied!' : 'Copy to Clipboard' }}
    </Button>
  </div>
</template>
