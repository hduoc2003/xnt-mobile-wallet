<script setup lang="ts">
import { computed } from 'vue'
import { Loader2 } from 'lucide-vue-next'

const props = withDefaults(defineProps<{
  open: boolean
}>(), {
  open: false,
})

const emit = defineEmits<{
  'update:open': [value: boolean]
}>()

const isOpen = computed({
  get: () => props.open,
  set: (value) => emit('update:open', value),
})

// Prevent closing when clicking outside or pressing escape
const handlePointerDownOutside = (event: any) => {
  event.preventDefault()
}

const handleEscapeKeyDown = (event: any) => {
  event.preventDefault()
}
</script>

<template>
  <Dialog v-model:open="isOpen">
    <DialogContent
      class="sm:max-w-md [&>button]:hidden"
      @pointer-down-outside="handlePointerDownOutside"
      @escape-key-down="handleEscapeKeyDown"
    >
      <DialogHeader>
        <DialogTitle class="text-center">Generating Proof</DialogTitle>
        <DialogDescription class="text-center">
          Please wait while the system is generating proof for your transaction...
        </DialogDescription>
      </DialogHeader>
      
      <div class="flex flex-col items-center justify-center py-6">
        <Loader2 class="size-12 animate-spin text-primary mb-4" />
        <p class="text-sm text-muted-foreground text-center">
          This process may take a few minutes
        </p>
      </div>
    </DialogContent>
  </Dialog>
</template>

