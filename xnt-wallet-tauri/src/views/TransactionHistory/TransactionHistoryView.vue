<script setup lang="ts">
import { computed, ref, onActivated } from 'vue'
import { useRouter } from 'vue-router'
import { CopyButton, ExplorerLink } from '@/components/commons'
import { History, ArrowUpRight, ArrowDownLeft, ArrowRightLeft } from 'lucide-vue-next'
import { useBridgeDeposit, useFormat, useTransaction } from '@/composables'
import { useTransactionStore } from '@/stores'
import type { BridgeDepositRecord, ReceivedEntry, TransactionDetail } from '@/types'
import { openUrl } from '@tauri-apps/plugin-opener'
import {
  EXPLORER_URL,
  LEVIATHAN_EXPLORER_URL,
  bridgeDepositBadge,
  bridgeDepositDetail,
  normalizeUtxo,
} from '@/utils'

type Kind = 'sent' | 'received' | 'bridge'
type Tone = 'pending' | 'confirmed' | 'dropped' | 'neutral'

interface HistoryRow {
  key: string
  /** Output commitment the explorer page is keyed by. */
  commitment: string
  kind: Kind
  /** Who received it: XNT address for sends, Leviathan address for deposits. */
  recipient?: string
  amount: number
  fee?: number
  timeMs: number
  status: string
  tone: Tone
  detail?: string
  txId?: string
  /** Leviathan mint tx, once the relayer submitted the mint. */
  mintTxId?: string
}

const KIND_LABEL: Record<Kind, string> = {
  sent: 'Sent',
  received: 'Received',
  bridge: 'Bridge to Leviathan',
}

const router = useRouter()
const { formatAddress, formatXnt } = useFormat()
const { loadTransactionsFromStorage, refreshTransactionStatuses, loadReceived, syncReceived } =
  useTransaction()
const {
  deposits,
  refreshError: depositError,
  load: loadDeposits,
  refresh: refreshDeposits,
} = useBridgeDeposit()
const transactionStore = useTransactionStore()
const received = ref<ReceivedEntry[]>([])
const loading = ref(true)
const errors = ref<string[]>([])

const sentStatus = (tx: TransactionDetail): Pick<HistoryRow, 'status' | 'tone'> => {
  const status = tx.status
  if (typeof status !== 'object') return { status: 'Checking…', tone: 'neutral' }
  switch (status.state) {
    case 'pending':
      return { status: 'Pending', tone: 'pending' }
    case 'dropped':
      return { status: 'Dropped', tone: 'dropped' }
    case 'confirmed':
      return {
        status:
          status.blockHeight === null ? 'Confirmed' : `Confirmed in block ${status.blockHeight}`,
        tone: 'confirmed',
      }
  }
}

const BRIDGE_TONE = {
  submitted: 'pending',
  confirmed: 'pending',
  minted: 'confirmed',
  dropped: 'dropped',
} as const

const rows = computed<HistoryRow[]>(() => {
  const sent: HistoryRow[] = transactionStore.getTransactions.map((tx) => ({
    key: `sent-${tx.id}`,
    commitment: tx.id,
    recipient: tx.toAddress,
    kind: 'sent',
    amount: tx.amount,
    fee: tx.fee,
    timeMs: new Date(tx.time).getTime(),
    txId: tx.txId,
    ...sentStatus(tx),
  }))
  const incoming: HistoryRow[] = received.value.map((r) => ({
    key: `received-${r.commitment}`,
    commitment: r.commitment,
    kind: 'received',
    amount: Number(r.amount),
    timeMs: new Date(r.time).getTime(),
    status: `Confirmed in block ${r.blockHeight}`,
    tone: 'confirmed',
  }))
  const bridge: HistoryRow[] = deposits.value.map((d) => ({
    key: `bridge-${d.utxoCommitment}`,
    commitment: d.utxoCommitment,
    recipient: d.midenAddress ?? undefined,
    kind: 'bridge',
    amount: Number(d.amount),
    fee: d.fee === null ? undefined : Number(d.fee),
    timeMs: d.createdMs,
    status: bridgeDepositBadge(d).label,
    tone: BRIDGE_TONE[d.status.state],
    detail: bridgeDepositDetail(d),
    txId: d.txId,
    mintTxId: d.relayer?.mintTxId ?? undefined,
  }))
  return [...sent, ...incoming, ...bridge].sort((a, b) => b.timeMs - a.timeMs)
})

const describe = (error: unknown) => (error instanceof Error ? error.message : String(error))

const syncReceivedFromScan = async () => {
  const cached = await (window as any).walletApi.getCachedUtxosLocal()
  if (!cached?.success || !Array.isArray(cached.utxos)) {
    throw new Error(`Could not read scanned UTXOs: ${cached?.error ?? 'no data'}`)
  }
  // From the local records file, so a node or relayer outage can't hide our deposit change.
  const depositRecords: BridgeDepositRecord[] = await (window as any).walletApi.listBridgeDeposits()
  const ownOutputs = new Set<string>([
    ...transactionStore.getTransactions.flatMap((tx) => [tx.id, ...(tx.outputCommitments ?? [])]),
    ...depositRecords.flatMap((d) => [d.utxoCommitment, ...d.outputCommitments]),
  ])
  const synced = await syncReceived(cached.utxos.map(normalizeUtxo), ownOutputs)
  received.value = synced.received
  if (synced.errors.length) throw new Error(synced.errors.join('; '))
}

// Quick tab switches overlap activations; only the latest one reports errors.
let activation = 0

onActivated(async () => {
  const current = ++activation
  errors.value = []
  // Local data first so every row renders at once; statuses refresh in the background.
  try {
    loadTransactionsFromStorage()
    received.value = loadReceived()
    await loadDeposits()
  } catch (error: unknown) {
    errors.value = [describe(error)]
  }
  loading.value = false
  const results = await Promise.allSettled([
    refreshDeposits(),
    refreshTransactionStatuses(),
    syncReceivedFromScan(),
  ])
  if (current !== activation) return
  errors.value = [
    ...errors.value,
    ...results
      .filter((r): r is PromiseRejectedResult => r.status === 'rejected')
      .map((r) => describe(r.reason)),
  ]
})

const toneClass = (tone: Tone) => {
  switch (tone) {
    case 'confirmed':
      return 'bg-green-500/15 text-green-600 border-green-500/30'
    case 'pending':
      return 'bg-amber-500/15 text-amber-600 border-amber-500/30'
    case 'dropped':
      return 'bg-red-500/15 text-red-600 border-red-500/30'
    case 'neutral':
      return 'bg-muted text-muted-foreground border-border'
  }
}

const openInExplorer = async (path: string) => {
  try {
    await openUrl(`${EXPLORER_URL}${path}`)
  } catch (error: unknown) {
    errors.value = [...errors.value, `Could not open the explorer: ${describe(error)}`]
  }
}

const shortId = (hex: string) => `${hex.slice(0, 10)}…${hex.slice(-6)}`
</script>

<template>
  <div class="p-4 h-full">
    <div class="mx-auto max-w-2xl space-y-6">
      <div class="flex items-center gap-3">
        <div class="rounded-full bg-primary/10 p-3">
          <History class="size-6 text-primary" />
        </div>
        <div>
          <h1 class="text-2xl font-bold text-foreground">Transaction History</h1>
          <p class="text-sm text-muted-foreground">Sent, received and bridged XNT</p>
        </div>
      </div>

      <p v-for="error in errors" :key="error" class="text-xs text-destructive">{{ error }}</p>
      <p v-if="depositError" class="text-xs text-destructive">
        Deposit status refresh failed: {{ depositError }}
      </p>

      <div
        v-if="rows.length === 0"
        class="flex flex-col items-center justify-center py-16 text-center"
      >
        <History class="mb-4 size-12 text-muted-foreground opacity-50" />
        <p class="text-sm text-muted-foreground">
          {{ loading ? 'Loading…' : 'No transactions yet' }}
        </p>
      </div>

      <div v-else class="space-y-3">
        <Card
          v-for="row in rows"
          :key="row.key"
          class="border border-border/70 bg-card cursor-pointer transition-all hover:bg-muted/50"
          @click="
            row.kind === 'bridge'
              ? router.push({
                  name: 'bridge-deposit-detail',
                  params: { commitment: row.commitment },
                })
              : openInExplorer(`/utxo/${row.commitment}`)
          "
        >
          <CardContent class="flex items-start justify-between gap-3 py-3">
            <div class="flex items-start gap-3">
              <div class="mt-1 rounded-full bg-primary/10 p-2">
                <ArrowUpRight v-if="row.kind === 'sent'" class="size-4 text-primary" />
                <ArrowDownLeft v-else-if="row.kind === 'received'" class="size-4 text-primary" />
                <ArrowRightLeft v-else class="size-4 text-primary" />
              </div>
              <div class="space-y-1">
                <div class="flex flex-wrap items-center gap-2">
                  <span class="text-sm font-medium text-foreground">{{
                    KIND_LABEL[row.kind]
                  }}</span>
                  <span
                    class="inline-flex items-center rounded-full border px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide"
                    :class="toneClass(row.tone)"
                  >
                    {{ row.status }}
                  </span>
                </div>
                <p
                  v-if="row.recipient"
                  class="font-mono text-xs text-muted-foreground"
                  :title="row.recipient"
                >
                  To {{ formatAddress(row.recipient) }}
                  <CopyButton :value="row.recipient" label="Address" />
                </p>
                <p v-if="row.detail" class="text-xs text-muted-foreground">{{ row.detail }}</p>
                <p class="text-xs text-muted-foreground">
                  {{ new Date(row.timeMs).toLocaleString() }}
                </p>
                <p
                  v-if="row.txId"
                  class="font-mono text-[10px] text-muted-foreground"
                  :title="row.txId"
                >
                  tx {{ shortId(row.txId) }}
                  <CopyButton :value="row.txId" label="Tx id" />
                  <ExplorerLink :path="`/tx?id=${row.txId}`" />
                </p>
                <p
                  v-if="row.mintTxId"
                  class="font-mono text-[10px] text-muted-foreground"
                  :title="row.mintTxId"
                >
                  mint {{ shortId(row.mintTxId) }}
                  <CopyButton :value="row.mintTxId" label="Mint tx id" />
                  <ExplorerLink :base="LEVIATHAN_EXPLORER_URL" :path="`/tx/${row.mintTxId}`" />
                </p>
              </div>
            </div>

            <div class="text-right space-y-1">
              <div class="text-sm font-semibold text-foreground">
                {{ formatXnt(row.amount) }}
                <span class="text-xs text-muted-foreground">XNT</span>
              </div>
              <div v-if="row.fee !== undefined" class="text-[11px] text-muted-foreground">
                Fee: {{ formatXnt(row.fee) }} XNT
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  </div>
</template>
