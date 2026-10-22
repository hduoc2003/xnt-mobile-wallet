<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ArrowLeft } from 'lucide-vue-next'
import { CopyButton, ExplorerLink } from '@/components/commons'
import { useBridgeDeposit, useBridgeFees, useFormat } from '@/composables'
import {
  LEVIATHAN_EXPLORER_URL,
  REFUSAL_REASON,
  RELAYER_STATE_LABEL,
  blocksText,
  blocksUntilMintable,
  fromBaseUnits,
  bridgeDepositBadge,
  bridgeDepositDetail,
} from '@/utils'

const route = useRoute()
const router = useRouter()
const { deposits, refreshError, load, refresh } = useBridgeDeposit()
const { formatAddress, formatXnt } = useFormat()
const { fees, loadFees } = useBridgeFees()

/** WXNT base units as WXNT, or labelled base units while the decimals are unknown. */
const wxntText = (units: number) =>
  fees.value
    ? `${fromBaseUnits(BigInt(units), fees.value.wxntDecimals)} WXNT`
    : `${units} WXNT base units`
const loaded = ref(false)

const commitment = computed(() => String(route.params.commitment))
const record = computed(() => deposits.value.find((d) => d.utxoCommitment === commitment.value))

const mined = computed(() => {
  const status = record.value?.status
  return status && (status.state === 'confirmed' || status.state === 'minted') ? status : undefined
})

const relayerStatus = computed(() => {
  const relayer = record.value?.relayer
  if (!relayer) return undefined
  const label = RELAYER_STATE_LABEL[relayer.status]
  if (relayer.status !== 'refused' || !relayer.reason) return label
  return `${label}: ${REFUSAL_REASON[relayer.reason]}`
})

const mintableText = computed(() => {
  const relayer = record.value?.relayer
  if (!relayer) return undefined
  if (relayer.mintableUpTo === null) return 'not yet (relayer has no mintable block)'
  const behind = record.value ? blocksUntilMintable(record.value) : undefined
  return behind === undefined
    ? `block ${relayer.mintableUpTo}`
    : `block ${relayer.mintableUpTo} (${blocksText(behind)} to go)`
})

const loadError = ref<string>()

onMounted(async () => {
  void loadFees()
  try {
    await load()
  } catch (error: unknown) {
    loadError.value = error instanceof Error ? error.message : String(error)
  }
  loaded.value = true
  await refresh()
})
</script>

<template>
  <div class="p-4">
    <div class="mx-auto max-w-2xl space-y-6">
      <div class="flex items-center gap-3">
        <Button variant="ghost" size="icon" @click="router.push('/transaction-history')">
          <ArrowLeft class="size-5" />
        </Button>
        <div>
          <h1 class="text-2xl font-bold text-foreground">Bridge to Leviathan</h1>
          <p class="text-sm text-muted-foreground">Deposit details</p>
        </div>
      </div>

      <p v-if="loadError" class="text-xs text-destructive">
        Could not read saved deposits: {{ loadError }}
      </p>
      <p v-if="refreshError" class="text-xs text-destructive">
        Status refresh failed: {{ refreshError }}
      </p>

      <p v-if="!record" class="text-sm text-muted-foreground">
        {{
          !loaded
            ? 'Loading…'
            : refreshError || loadError
              ? 'Could not load deposits; go back and try again'
              : 'No deposit with this commitment on this device'
        }}
      </p>

      <Card v-else>
        <CardHeader>
          <CardTitle class="flex items-center justify-between gap-2">
            <span>{{ formatXnt(record.amount) }} XNT</span>
            <Badge :variant="bridgeDepositBadge(record).variant">
              {{ bridgeDepositBadge(record).label }}
            </Badge>
          </CardTitle>
          <p class="text-sm text-muted-foreground">{{ bridgeDepositDetail(record) }}</p>
        </CardHeader>
        <CardContent class="space-y-3 text-sm">
          <div>
            <Label class="text-xs text-muted-foreground">Leviathan recipient</Label>
            <p
              v-if="record.leviathanAddress"
              class="font-mono text-xs"
              :title="record.leviathanAddress"
            >
              {{ formatAddress(record.leviathanAddress) }}
              <CopyButton :value="record.leviathanAddress" label="Recipient" />
            </p>
            <p v-else class="text-xs text-muted-foreground">
              Not recorded (deposit made before recipients were stored)
            </p>
          </div>
          <div class="grid grid-cols-2 gap-3">
            <div>
              <Label class="text-xs text-muted-foreground">Fee</Label>
              <p>{{ record.fee === null ? 'Not recorded' : `${formatXnt(record.fee)} XNT` }}</p>
            </div>
            <div>
              <Label class="text-xs text-muted-foreground">Time</Label>
              <p>{{ new Date(record.createdMs).toLocaleString() }}</p>
            </div>
            <div>
              <Label class="text-xs text-muted-foreground">XNT block</Label>
              <p v-if="!mined">Not mined yet</p>
              <p v-else-if="mined.blockHeight === null">Mined, block unknown</p>
              <p v-else>
                {{ mined.blockHeight }}
                <ExplorerLink :path="`/block/${mined.blockHeight}`" />
              </p>
            </div>
            <div>
              <Label class="text-xs text-muted-foreground">Relayer</Label>
              <p>{{ relayerStatus ?? (record.relayerError ? "can't check" : 'not asked yet') }}</p>
            </div>
            <div v-if="mintableText">
              <Label class="text-xs text-muted-foreground">Mintable up to</Label>
              <p>{{ mintableText }}</p>
            </div>
            <div v-if="record.relayer">
              <Label class="text-xs text-muted-foreground">Bridge paused</Label>
              <p>{{ record.relayer.paused ? 'Yes' : 'No' }}</p>
            </div>
          </div>
          <p v-if="record.relayerError" class="text-xs text-destructive">
            Last relayer check failed: {{ record.relayerError }}
          </p>
          <div v-if="!mined">
            <Label class="text-xs text-muted-foreground">XNT tx id</Label>
            <p class="break-all font-mono text-xs">
              {{ record.txId }}
              <CopyButton :value="record.txId" label="Tx id" />
              <ExplorerLink :path="`/tx?id=${record.txId}`" />
            </p>
          </div>
          <div>
            <Label class="text-xs text-muted-foreground">Deposit commitment</Label>
            <p class="break-all font-mono text-xs">
              {{ record.utxoCommitment }}
              <CopyButton :value="record.utxoCommitment" label="Commitment" />
              <ExplorerLink :path="`/utxo/${record.utxoCommitment}`" />
            </p>
          </div>
          <div
            v-if="
              record.relayer &&
              (record.relayer.mintFee !== null || record.relayer.mintedWxnt !== null)
            "
            class="grid grid-cols-2 gap-3"
          >
            <div v-if="record.relayer.mintFee !== null">
              <Label class="text-xs text-muted-foreground">Mint fee</Label>
              <p>{{ wxntText(record.relayer.mintFee) }}</p>
            </div>
            <div v-if="record.relayer.mintedWxnt !== null">
              <Label class="text-xs text-muted-foreground">WXNT received</Label>
              <p>{{ wxntText(record.relayer.mintedWxnt) }}</p>
            </div>
          </div>
          <div v-if="record.relayer?.mintTxId">
            <Label class="text-xs text-muted-foreground">Leviathan mint tx</Label>
            <p class="break-all font-mono text-xs">
              {{ record.relayer.mintTxId }}
              <CopyButton :value="record.relayer.mintTxId" label="Mint tx id" />
              <ExplorerLink
                :base="LEVIATHAN_EXPLORER_URL"
                :path="`/tx/${record.relayer.mintTxId}`"
              />
            </p>
          </div>
          <div v-if="record.relayer?.mintNoteId">
            <Label class="text-xs text-muted-foreground">WXNT note</Label>
            <p class="break-all font-mono text-xs">
              {{ record.relayer.mintNoteId }}
              <CopyButton :value="record.relayer.mintNoteId" label="Note id" />
              <ExplorerLink
                :base="LEVIATHAN_EXPLORER_URL"
                :path="`/note/${record.relayer.mintNoteId}`"
              />
            </p>
          </div>
        </CardContent>
      </Card>
    </div>
  </div>
</template>
