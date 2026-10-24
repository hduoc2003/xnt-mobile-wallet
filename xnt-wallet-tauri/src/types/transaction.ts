/** Status of a history entry, from our node. */
export type HistoryStatus =
  | { state: 'pending' }
  | { state: 'confirmed'; blockHeight: number | null }
  | { state: 'dropped' }

/** Statuses written by builds that asked the mainnet explorer; re-queried on load. */
export type LegacyTxStatus = 'confirmed' | 'pending' | 'failed' | 'unknown'

export interface TransactionDetail {
  /** Canonical commitment of the tx's first output. */
  id: string
  txId?: string
  /** All output commitments, so our own change isn't listed as received. */
  outputCommitments?: string[]
  toAddress: string
  amount: number
  fee: number
  time: string
  status?: HistoryStatus | LegacyTxStatus
}

export interface ReceivedEntry {
  /** Absent on entries logged before identical payments were told apart. */
  utxoId?: string
  commitment: string
  amount: string
  blockHeight: number
  /** ISO time of the block that created the output. */
  time: string
  /** Leviathan burn serial when a bridge settlement paid this output. */
  bridgeSerial?: string
}
