<script setup lang="ts">
import { DEFAULT_MAX_INPUTS, MAX_INPUTS_LIMIT } from '@/utils'

const maxInputs = defineModel<number>({ required: true })
defineProps<{ disabled?: boolean }>()

const choices = Array.from({ length: MAX_INPUTS_LIMIT }, (_, i) => i + 1)
</script>

<template>
  <div class="space-y-2">
    <Label class="text-xs text-muted-foreground">Max inputs (UTXOs) per transaction</Label>
    <div class="flex overflow-hidden rounded-md border text-sm" role="radiogroup">
      <button
        v-for="n in choices"
        :key="n"
        type="button"
        role="radio"
        :aria-checked="maxInputs === n"
        :disabled="disabled"
        :class="[
          'flex-1 py-1.5 transition-colors disabled:opacity-50',
          maxInputs === n
            ? 'bg-primary text-primary-foreground'
            : 'bg-transparent text-muted-foreground hover:bg-muted',
        ]"
        @click="maxInputs = n"
      >
        {{ n }}
      </button>
    </div>
    <p v-if="maxInputs > DEFAULT_MAX_INPUTS" class="text-xs text-amber-600">
      Each input adds memory to the on-device proof; on phones, more than
      {{ DEFAULT_MAX_INPUTS }} can crash the app.
    </p>
  </div>
</template>
