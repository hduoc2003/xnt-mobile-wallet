import { toFiniteNumber } from './helpers'

/** Canonical UTXO shape used throughout the wallet UI and coin selection. */
export interface NormalizedUtxo {
  utxoDigest: string
  amount: string
  blockHeight: number
  additionRecord: string
  isTimelocked: boolean
  releaseDateMs: number | null
  keyType: string
  isPending: boolean
}

/**
 * Normalizes a raw UTXO record into {@link NormalizedUtxo}. Accepts both the
 * snake_case shape returned by the backend scan and the camelCase shape read
 * back from the local cache, preferring whichever field is present.
 */
export function normalizeUtxo(u: any): NormalizedUtxo {
  return {
    utxoDigest: u.utxo_digest || u.utxoDigest || '',
    amount: u.amount?.toString?.() ?? '0',
    blockHeight: toFiniteNumber(u.block_height || u.blockHeight || 0),
    additionRecord: u.addition_record_hex || u.additionRecord || '',
    isTimelocked: !!(u.is_timelocked ?? u.isTimelocked),
    releaseDateMs: u.release_date_ms ?? u.releaseDateMs ?? null,
    keyType: u.key_type || u.keyType || '',
    isPending: !!(u.is_pending ?? u.isPending),
  }
}
