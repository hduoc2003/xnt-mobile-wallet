<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { Settings, Check, X } from 'lucide-vue-next'
import { toast } from 'vue-sonner'
import { useNeptuneStore } from '@/stores'
import { useNeptuneWallet, useRpc } from '@/composables'
import type { ChainCheck } from '@/types'

const emit = defineEmits<{
  endpointUpdated: []
}>()

const store = useNeptuneStore()
const { saveRpcEndpoint } = useRpc()
const { getUtxos } = useNeptuneWallet()

const rpcUrl = computed(() => store.getRpcUrl)
const rpcEndpointInput = ref('')
const isEditingEndpoint = ref(false)
const savingEndpoint = ref(false)

watch(
  rpcUrl,
  (newUrl) => {
    if (!isEditingEndpoint.value) {
      rpcEndpointInput.value = newUrl || ''
    }
  },
  { immediate: true },
)

const startEditEndpoint = () => {
  rpcEndpointInput.value = rpcUrl.value || ''
  isEditingEndpoint.value = true
}

const cancelEditEndpoint = () => {
  rpcEndpointInput.value = rpcUrl.value || ''
  isEditingEndpoint.value = false
}

const handleSaveRpcEndpoint = async () => {
  try {
    savingEndpoint.value = true
    const trimmedEndpoint = rpcEndpointInput.value.trim()

    if (
      !trimmedEndpoint?.startsWith('http://') &&
      !trimmedEndpoint?.startsWith('https://') &&
      trimmedEndpoint !== ''
    ) {
      return toast.error('Endpoint must start with http:// or https://')
    }

    await saveRpcEndpoint(rpcEndpointInput.value)

    isEditingEndpoint.value = false
    emit('endpointUpdated')

    // On another chain the scan rescans from 0 and shows the reset toast.
    let check: ChainCheck
    try {
      check = await (window as any).walletApi.checkChain()
    } catch (error: unknown) {
      return toast.error('Endpoint saved, but the node could not be checked', {
        description: error instanceof Error ? error.message : String(error),
      })
    }
    if (check.state === 'reset' || check.state === 'reorg') {
      getUtxos().catch((error: unknown) =>
        toast.error('Failed to re-sync the wallet', {
          description: error instanceof Error ? error.message : String(error),
        }),
      )
    }

    savingEndpoint.value = false
  } catch (error: unknown) {
    toast.error('Failed to save RPC endpoint')
  } finally {
    savingEndpoint.value = false
  }
}
</script>

<template>
  <Card class="border-2 border-border/50 shadow-xl">
    <CardHeader class="space-y-1">
      <CardTitle class="flex items-center gap-2 text-lg">
        <Settings class="size-5 text-primary" />
        RPC Endpoint Configuration
      </CardTitle>
    </CardHeader>
    <CardContent class="space-y-4">
      <div class="space-y-2">
        <Label class="text-xs text-muted-foreground">Custom RPC Endpoint</Label>
        <div v-if="!isEditingEndpoint" class="flex items-center gap-2">
          <code
            class="flex-1 rounded-lg border border-border bg-muted/50 px-3 py-2 text-sm text-foreground"
          >
            {{ rpcUrl }}
          </code>
          <Button variant="outline" size="icon" @click="startEditEndpoint">
            <Settings class="size-4" />
          </Button>
        </div>
        <div v-else class="space-y-2">
          <Input
            v-model="rpcEndpointInput"
            placeholder="https://"
            class="font-mono"
            :disabled="savingEndpoint"
          />
          <div class="flex items-center gap-2">
            <Button
              size="sm"
              :disabled="savingEndpoint"
              class="flex-1"
              @click="handleSaveRpcEndpoint"
            >
              <Check class="mr-2 size-4" />
              {{ savingEndpoint ? 'Saving...' : 'Save' }}
            </Button>
            <Button
              size="sm"
              variant="outline"
              :disabled="savingEndpoint"
              @click="cancelEditEndpoint"
            >
              <X class="size-4" />
            </Button>
          </div>
        </div>
      </div>
      <p class="text-xs text-muted-foreground">
        Configure a custom RPC endpoint to connect to a different node.
      </p>
    </CardContent>
  </Card>
</template>
