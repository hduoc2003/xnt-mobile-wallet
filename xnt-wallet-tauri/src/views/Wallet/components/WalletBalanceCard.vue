<script setup lang="ts">
import { computed, ref } from 'vue'
import { Wallet, Copy, Check } from 'lucide-vue-next'
import { useNeptuneStore } from '@/stores'
import { toast } from 'vue-sonner'
import { useFormat } from '@/composables'

interface Props {
  availableBalance: string
  pendingBalance: string
  loading?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  loading: false,
})

const { formatAmount, formatHash } = useFormat()
const neptuneStore = useNeptuneStore()
const isCopied = ref(false)

const receiveAddress = computed(() => neptuneStore.getWallet?.address || '')

const handleCopyAddress = async () => {
  try {
    await navigator.clipboard.writeText(receiveAddress.value)
    isCopied.value = true
    toast.success('Address copied to clipboard')
    setTimeout(() => {
      isCopied.value = false
    }, 2000)
  } catch (error) {
    toast.error('Failed to copy address')
  }
}
</script>

<template>
  <Card class="border-2 border-border/50 shadow-xl">
    <CardHeader class="space-y-1 pb-4">
      <CardTitle class="flex items-center gap-2 text-sm font-medium text-muted-foreground">
        <Wallet class="size-4" />
        Available Balance
      </CardTitle>
    </CardHeader>
    <CardContent class="space-y-6">
      <!-- Balance Display -->
      <div class="space-y-2">
        <div v-if="loading" class="flex items-center justify-center py-8">
          <div
            class="size-8 animate-spin rounded-full border-4 border-primary border-t-transparent"
          />
        </div>
        <div v-else class="space-y-1">
          <div class="flex items-baseline gap-2">
            <span class="text-4xl font-bold text-foreground">{{ formatAmount(+availableBalance) }}</span>
            <span class="text-xl font-medium text-muted-foreground">XNT</span>
            <span class="text-lg font-medium text-muted-foreground"> = $5.54</span>
          </div>
          <div v-if="pendingBalance" class="flex items-center gap-2 text-sm">
            <span class="text-muted-foreground">Pending:</span>
            <span class="font-medium text-foreground">{{ formatAmount(+pendingBalance) }} XNT</span>
          </div>
        </div>
      </div>

      <Separator />

      <!-- Receiving Address -->
      <div class="space-y-3">
        <Label class="text-sm font-medium">Receiving Address</Label>
        <div class="flex items-center gap-2 rounded-lg border border-border bg-muted/50 p-3 transition-colors hover:bg-muted">
          <code class="flex-1 overflow-hidden text-ellipsis whitespace-nowrap text-xs text-foreground md:text-sm">
            {{formatHash(receiveAddress) }}
          </code>
          <Button
            variant="ghost"
            size="icon"
            class="size-8 shrink-0"
            @click="handleCopyAddress"
          >
            <Check v-if="isCopied" class="size-4 text-green-500" />
            <Copy v-else class="size-4" />
          </Button>
        </div>
      </div>
    </CardContent>
  </Card>
</template>

