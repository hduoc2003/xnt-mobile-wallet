/**
 * Converts a decimal XNT value to integer atomic units (XNT has 8 decimal
 * places). Selection and balance math must run on these integers: comparing
 * decimal amounts as JS floats (e.g. `0.3 >= 0.1 + 0.2`) yields wrong results.
 */
export const toAtomicUnits = (value: number | string): number => Math.round(Number(value) * 1e8)

/** Active spending keys, keyed by the address type they unlock. */
export interface SpendingKeys {
  genKey: string
  dctidhKey: string
}

export interface CoinSelection {
  selectedUtxos: any[]
  totalAmount: number
  /** Default (non-symmetric) spending key required by the backend. */
  spendingKeyHex: string
  /** Per-input spending keys, positionally aligned with `selectedUtxos`. */
  spendingKeyHexes: string[]
}

/**
 * Greedily selects the largest spendable UTXOs across ALL key types until the
 * target amount (`neededAmount + fee`) is covered or the input cap is reached.
 *
 * Mixed Generation + dCTIDH inputs are supported by the backend via per-input
 * spending keys, so selection is intentionally not restricted to a single key
 * type. This keeps it consistent with the "max sendable" figure, which likewise
 * sums the top `maxInputs` UTXOs regardless of key type.
 */
export function selectSpendableUtxos(
  utxos: any[],
  neededAmount: number,
  fee: number,
  keys: SpendingKeys,
  maxInputs: number,
): CoinSelection {
  const neededAtomic = toAtomicUnits(neededAmount) + toAtomicUnits(fee)
  const now = Date.now()

  // Resolve the spending key for a UTXO by its key type (absent => legacy dCTIDH).
  const keyForUtxo = (u: any): string => (u.keyType === 'Generation' ? keys.genKey : keys.dctidhKey)

  // Keep only spendable UTXOs: not pending, and either not timelocked or past release.
  const spendable = utxos.filter((u) => {
    if (u.is_pending ?? u.isPending) return false
    const isTimelocked = !!(u.is_timelocked ?? u.isTimelocked)
    if (!isTimelocked) return true
    const releaseDateMs = u.release_date_ms ?? u.releaseDateMs ?? null
    return releaseDateMs !== null && releaseDateMs <= now
  })

  const sorted = [...spendable].sort(
    (a, b) => parseFloat(b.amount || '0') - parseFloat(a.amount || '0'),
  )

  let totalAtomic = 0
  const selected: any[] = []
  for (const utxo of sorted) {
    const amtAtomic = toAtomicUnits(utxo.amount || '0')
    if (isNaN(amtAtomic) || amtAtomic <= 0) continue
    // Skip UTXOs whose spending key is unavailable — we couldn't sign them.
    if (!keyForUtxo(utxo)) continue
    selected.push(utxo)
    totalAtomic += amtAtomic
    if (totalAtomic >= neededAtomic || selected.length >= maxInputs) break
  }

  const spendingKeyHexes = selected.map(keyForUtxo)
  // The backend still requires a valid default spendingKeyHex; use the first
  // selected input's key, falling back to any available key.
  const spendingKeyHex = spendingKeyHexes[0] || keys.genKey || keys.dctidhKey

  return {
    selectedUtxos: selected,
    totalAmount: totalAtomic / 1e8,
    spendingKeyHex,
    spendingKeyHexes,
  }
}
