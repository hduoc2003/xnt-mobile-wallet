<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { toast } from 'vue-sonner'
import { useLegacyDctidh } from '@/composables'
import { formatXnt } from '@/utils'
import ProofGenerationModal from './ProofGenerationModal.vue'

const emit = defineEmits<{ moved: [] }>()

const { legacyUtxos, legacyAmount, pendingCount, movableCount, loadDustThreshold, migrate } =
  useLegacyDctidh()
const moving = ref(false)

onMounted(async () => {
  try {
    await loadDustThreshold()
  } catch (error: unknown) {
    toast.error('Could not load the legacy dust threshold', {
      description: error instanceof Error ? error.message : String(error),
    })
  }
})

const move = async () => {
  moving.value = true
  try {
    const result = await migrate()
    toast.success(`Moving ${formatXnt(result.amount)} XNT to your current address`, {
      description: [
        `Fee ${formatXnt(result.fee)} XNT.`,
        result.numWaiting > result.numDust &&
          `${result.numWaiting - result.numDust} more UTXO(s) left: move them after this one confirms.`,
        result.numDust > 0 && `${result.numDust} UTXO(s) are worth less than their fee.`,
      ]
        .filter(Boolean)
        .join(' '),
    })
    emit('moved')
  } catch (error: unknown) {
    toast.error('Could not move the funds', {
      description: error instanceof Error ? error.message : String(error),
    })
  } finally {
    moving.value = false
  }
}
</script>

<template>
  <Card v-if="legacyUtxos.length > 0" class="border-2 border-amber-500/50">
    <CardContent class="space-y-3 pt-6">
      <p class="text-sm">
        {{ formatXnt(legacyAmount) }} XNT is at this account's old dCTIDH address, from before the
        v0.3 upgrade. It isn't in your balance until you move it to your current address.
      </p>
      <p v-if="movableCount === 0 && pendingCount === 0" class="text-xs text-muted-foreground">
        What is left is worth less than the fee to move it.
      </p>
      <Button
        v-else
        class="w-full"
        :disabled="moving || movableCount === undefined || movableCount === 0"
        @click="move"
      >
        {{
          moving
            ? 'Moving...'
            : movableCount === 0
              ? 'Waiting for the move to confirm'
              : 'Move to current address'
        }}
      </Button>
    </CardContent>
  </Card>
  <ProofGenerationModal :open="moving" />
</template>
