<script setup lang="ts">
import { SlidersHorizontal } from 'lucide-vue-next'
import { toast } from 'vue-sonner'
import { useSettingsStore } from '@/stores'
import { DEFAULT_MAX_INPUTS, MAX_INPUTS_LIMIT } from '@/utils'

const settingsStore = useSettingsStore()
const choices = Array.from({ length: MAX_INPUTS_LIMIT }, (_, i) => i + 1)

const save = (apply: () => void) => {
  try {
    apply()
  } catch (error: unknown) {
    toast.error('Could not save the setting', {
      description: error instanceof Error ? error.message : String(error),
    })
  }
}
</script>

<template>
  <Card class="border-2 border-border/50 shadow-xl">
    <CardHeader class="space-y-1">
      <CardTitle class="flex items-center gap-2 text-lg">
        <SlidersHorizontal class="size-5 text-primary" />
        Transactions
      </CardTitle>
    </CardHeader>
    <CardContent class="space-y-5">
      <p v-for="error in settingsStore.loadErrors" :key="error" class="text-xs text-destructive">
        {{ error }}
      </p>
      <div class="space-y-2">
        <Label class="text-xs text-muted-foreground">Max inputs per transaction</Label>
        <div class="flex overflow-hidden rounded-md border text-sm" role="radiogroup">
          <button
            v-for="n in choices"
            :key="n"
            type="button"
            role="radio"
            :aria-checked="settingsStore.maxInputsPerTx === n"
            :class="[
              'flex-1 py-1.5 transition-colors',
              settingsStore.maxInputsPerTx === n
                ? 'bg-primary text-primary-foreground'
                : 'bg-transparent text-muted-foreground hover:bg-muted',
            ]"
            @click="save(() => settingsStore.setMaxInputsPerTx(n))"
          >
            {{ n }}
          </button>
        </div>
        <p class="text-xs text-muted-foreground">
          Used by Send and Bridge. Default on this device: {{ DEFAULT_MAX_INPUTS }}.
        </p>
        <p v-if="settingsStore.maxInputsPerTx > DEFAULT_MAX_INPUTS" class="text-xs text-amber-600">
          More inputs need more memory to prove; on phones this can crash the app.
        </p>
      </div>

      <div class="flex items-center justify-between gap-4">
        <div class="space-y-0.5">
          <Label for="low-memory-proving">Low-memory proving</Label>
          <p class="text-xs text-muted-foreground">Uses less memory, but proving is slower.</p>
        </div>
        <Switch
          id="low-memory-proving"
          :model-value="settingsStore.lowMemoryProving"
          @update:model-value="(on: boolean) => save(() => settingsStore.setLowMemoryProving(on))"
        />
      </div>
    </CardContent>
  </Card>
</template>
