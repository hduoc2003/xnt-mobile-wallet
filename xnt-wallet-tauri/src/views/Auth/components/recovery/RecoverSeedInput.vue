<script setup lang="ts">
import { ref } from 'vue'
import { Input } from '@/components/ui/input'
import { Alert, AlertDescription } from '@/components/ui/alert'

interface Props {
  valid?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  valid: true,
})

const emit = defineEmits<{
  submit: [words: string[]]
  'update:words': [words: string[]]
  'update:valid': [valid: boolean]
}>()

const seedWords = ref<string[]>(Array.from({ length: 18 }, () => ''))

const handleInput = (index: number, value: string) => {
  emit('update:valid', true)
  seedWords.value[index] = value.trim().toLowerCase()
  emit(
    'update:words',
    seedWords.value.filter((w) => w.trim())
  )

  // Auto-focus next input on space or enter
  if (value.endsWith(' ') && index < 17) {
    const nextInput = document.getElementById(`seed-input-${index + 1}`)
    if (nextInput) {
      nextInput.focus()
      seedWords.value[index] = value.trim()
    }
  }
}

const handleKeyDown = (event: KeyboardEvent, index: number) => {
  if (event.key === 'Enter' && index < 17) {
    event.preventDefault()
    const nextInput = document.getElementById(`seed-input-${index + 1}`)
    if (nextInput) {
      nextInput.focus()
    }
  }
}

const handlePaste = (event: ClipboardEvent) => {
  event.preventDefault()
  emit('update:valid', true)
  
  const pastedData = event.clipboardData?.getData('text') || ''
  const words = pastedData
    .trim()
    .toLowerCase()
    .split(/\s+/)
    .filter((w) => w)

  if (words.length === 0) return

  const filledWords = Array.from({ length: 18 }, (_, i) => words[i] || '')
  seedWords.value = filledWords
  emit(
    'update:words',
    words.filter((w) => w.trim())
  )
}

const handleSubmit = () => {
  const words = seedWords.value.filter((w) => w.trim())
  emit('submit', words)
}

defineExpose({
  handleSubmit,
})
</script>

<template>
  <div class="space-y-4">
    <!-- Info Badge -->
    <Alert class="border-2 border-primary/20 bg-primary/5">
      <AlertDescription class="text-center text-sm font-medium">
        Enter your <span class="font-bold text-primary">18-word</span> recovery phrase
      </AlertDescription>
    </Alert>

    <!-- Seed Input Grid -->
    <div class="grid grid-cols-2 gap-2 sm:grid-cols-3">
      <div v-for="(word, i) in seedWords" :key="i" class="relative">
        <span class="absolute left-3 top-1/2 z-10 -translate-y-1/2 text-xs font-bold text-muted-foreground">
          {{ i + 1 }}.
        </span>
        <Input
          :id="`seed-input-${i}`"
          v-model="seedWords[i]"
          type="text"
          inputmode="text"
          autocapitalize="off"
          autocomplete="off"
          spellcheck="false"
          :placeholder="`Word ${i + 1}`"
          class="h-11 pl-9 text-base font-medium"
          :class="{ 
            'border-destructive focus-visible:ring-destructive': !props.valid,
            'border-green-500/50 bg-green-500/5': word && props.valid
          }"
          maxlength="24"
          @input="handleInput(i, ($event.target as HTMLInputElement).value)"
          @keydown="handleKeyDown($event, i)"
          @paste="handlePaste($event)"
        />
      </div>
    </div>

    <!-- Paste Hint -->
    <div class="rounded-lg border border-border/50 bg-muted/30 p-3">
      <p class="text-center text-xs text-muted-foreground">
        💡 Tip: You can paste all 18 words at once into any field
      </p>
    </div>

    <!-- Error Message -->
    <Alert v-if="!props.valid" variant="destructive" class="border-2">
      <AlertDescription class="text-sm font-medium">
        ❌ Invalid recovery phrase. Please check your words and try again.
      </AlertDescription>
    </Alert>
  </div>
</template>
