<script setup lang="ts">
import { toast } from 'vue-sonner'
import { ExternalLink } from 'lucide-vue-next'
import { openUrl } from '@tauri-apps/plugin-opener'
import { EXPLORER_URL } from '@/utils'

/** `path` on the explorer at `base` (default: the XNT explorer), e.g. `/tx?id=…`, `/utxo/…`. */
const props = withDefaults(defineProps<{ path: string; base?: string }>(), {
  base: EXPLORER_URL,
})

const open = async () => {
  try {
    await openUrl(`${props.base}${props.path}`)
  } catch (error: unknown) {
    toast.error('Could not open the explorer', {
      description: error instanceof Error ? error.message : String(error),
    })
  }
}
</script>

<template>
  <button
    type="button"
    class="inline-flex shrink-0 items-center rounded p-0.5 align-middle text-muted-foreground hover:text-foreground focus-visible:outline focus-visible:outline-2 focus-visible:outline-ring"
    title="Open in explorer"
    aria-label="Open in explorer"
    @click.stop="open"
  >
    <ExternalLink class="size-3.5" />
  </button>
</template>
