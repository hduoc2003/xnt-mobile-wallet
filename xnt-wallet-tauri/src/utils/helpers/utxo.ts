import { toFiniteNumber } from './helpers'

/** Canonical UTXO shape used throughout the wallet UI and coin selection. */
export interface NormalizedUtxo {
  utxoId: string
  utxoDigest: string
  amount: string
  blockHeight: number
  additionRecord: string
  isTimelocked: boolean
  releaseDateMs: number | null
  keyType: string
  isPending: boolean
  /** At a pre-v0.3 dCTIDH address: can only be moved by a migration, never spent. */
  isLegacyDctidh: boolean
  /** Set when the scan was asked per account. */
  accountId?: string
}

/**
 * Normalizes a raw UTXO record into {@link NormalizedUtxo}. Accepts both the
 * snake_case shape returned by the backend scan and the camelCase shape read
 * back from the local cache, preferring whichever field is present.
 */
export function normalizeUtxo(u: any): NormalizedUtxo {
  const utxoId = u.utxo_id || u.utxoId
  if (!utxoId) throw new Error(`UTXO ${u.utxo_digest || u.utxoDigest} has no utxo id`)
  return {
    utxoId,
    utxoDigest: u.utxo_digest || u.utxoDigest || '',
    amount: u.amount?.toString?.() ?? '0',
    blockHeight: toFiniteNumber(u.block_height || u.blockHeight || 0),
    additionRecord: u.addition_record_hex || u.additionRecord || '',
    isTimelocked: !!(u.is_timelocked ?? u.isTimelocked),
    releaseDateMs: u.release_date_ms ?? u.releaseDateMs ?? null,
    keyType: u.key_type || u.keyType || '',
    isPending: !!(u.is_pending ?? u.isPending),
    isLegacyDctidh: !!(u.is_legacy_dctidh ?? u.isLegacyDctidh),
    ...(u.account_id || u.accountId ? { accountId: u.account_id || u.accountId } : {}),
  }
}
