<script setup lang="ts">
import { CopyButton } from '@/components/commons'
import { computed } from 'vue'
import { useFormat } from '@/composables'

const props = withDefaults(
  defineProps<{
    open: boolean
    recipientAddress: string
    amount: string
    fee: string
    timelockEnabled?: boolean
    timelockDate?: string
    timelockTime?: string
    loading?: boolean
  }>(),
  {
    timelockEnabled: false,
    timelockDate: '',
    timelockTime: '',
    loading: false,
  },
)

const emit = defineEmits<{
  'update:open': [value: boolean]
  confirm: []
  cancel: []
}>()

const { formatHash, formatXnt } = useFormat()

const isOpen = computed({
  get: () => props.open,
  set: (value) => emit('update:open', value),
})

const handleConfirm = () => {
  emit('confirm')
}

const handleCancel = () => {
  emit('cancel')
}
</script>

<template>
  <Drawer v-model:open="isOpen">
    <DrawerContent class="h-screen flex flex-col">
      <div class="flex-1 overflow-y-auto px-6 pt-6 pb-4">
        <!-- Header -->
        <div>
          <DrawerTitle class="text-2xl font-bold text-foreground">Confirm Transaction</DrawerTitle>
          <DrawerDescription class="text-sm text-muted-foreground">
            Please review the transaction details before sending
          </DrawerDescription>
        </div>

        <div class="space-y-6">
          <!-- Recipient -->
          <div class="space-y-3">
            <Label class="text-sm font-medium text-muted-foreground">Recipient Address</Label>
            <div
              class="break-all rounded-lg border border-border bg-muted/50 p-4 font-mono text-sm"
            >
              {{ formatHash(props.recipientAddress) }}
              <CopyButton :value="props.recipientAddress" label="Address" />
            </div>
          </div>

          <!-- Amount and Fee -->
          <div class="grid grid-cols-2 gap-4">
            <div class="space-y-3">
              <Label class="text-sm font-medium text-muted-foreground">Amount</Label>
              <div class="rounded-lg border border-border bg-muted/50 p-4 font-semibold text-lg">
                {{ formatXnt(props.amount) }} XNT
              </div>
            </div>
            <div class="space-y-3">
              <Label class="text-sm font-medium text-muted-foreground">Priority Fee</Label>
              <div class="rounded-lg border border-border bg-muted/50 p-4 font-semibold text-lg">
                {{ formatXnt(props.fee) }} XNT
              </div>
            </div>
          </div>

          <!-- Timelock Information -->
          <div v-if="props.timelockEnabled && props.timelockDate" class="space-y-3">
            <Label class="text-sm font-medium text-muted-foreground">Time Lock</Label>
            <div class="rounded-lg border border-primary/50 bg-primary/10 p-4">
              <div class="flex items-center gap-2">
                <Badge variant="outline" class="text-xs">Locked Until</Badge>
                <span class="font-semibold text-sm">
                  {{
                    new Date(
                      `${props.timelockDate}T${props.timelockTime || '00:00'}`,
                    ).toLocaleString()
                  }}
                </span>
              </div>
              <p class="mt-2 text-xs text-muted-foreground">
                Outputs will be locked and cannot be spent until the release date.
              </p>
            </div>
          </div>

          <Alert class="mt-4">
            <AlertDescription>
              This action cannot be undone. Make sure all details are correct before proceeding.
            </AlertDescription>
          </Alert>
        </div>
      </div>

      <DrawerFooter class="px-6 py-4 border-t bg-background">
        <Button
          :disabled="props.loading"
          size="lg"
          class="h-12 w-full text-base font-semibold"
          @click="handleConfirm"
        >
          <span
            v-if="props.loading"
            class="mr-2 size-5 animate-spin rounded-full border-2 border-white border-t-transparent"
          />
          {{ props.loading ? 'Sending...' : 'Confirm & Send' }}
        </Button>
      </DrawerFooter>
    </DrawerContent>
  </Drawer>
</template>
