<script setup lang="ts">
import { onUnmounted, ref } from 'vue'
import { toast } from 'vue-sonner'
import { Check, Copy } from 'lucide-vue-next'

const props = defineProps<{ value: string; label?: string }>()
const copied = ref(false)
let resetTimer: ReturnType<typeof setTimeout> | undefined

const copy = async () => {
  try {
    await navigator.clipboard.writeText(props.value)
    copied.value = true
    toast.success(`${props.label ?? 'Value'} copied to clipboard`)
    clearTimeout(resetTimer)
    resetTimer = setTimeout(() => (copied.value = false), 1500)
  } catch {
    toast.error(`Failed to copy ${props.label?.toLowerCase() ?? 'value'}`)
  }
}
onUnmounted(() => clearTimeout(resetTimer))
</script>

<template>
  <button
    type="button"
    class="inline-flex shrink-0 items-center rounded p-0.5 align-middle text-muted-foreground hover:text-foreground focus-visible:outline focus-visible:outline-2 focus-visible:outline-ring"
    :title="`Copy ${label?.toLowerCase() ?? 'value'}`"
    :aria-label="`Copy ${label?.toLowerCase() ?? 'value'}`"
    @click.stop="copy"
  >
    <Check v-if="copied" class="size-3.5 text-green-600" />
    <Copy v-else class="size-3.5" />
  </button>
</template>
