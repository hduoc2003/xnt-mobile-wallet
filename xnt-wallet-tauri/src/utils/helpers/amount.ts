/** XNT has 8 decimals; numbers are rendered at that precision before trimming. */
const XNT_DECIMALS = 8

/**
 * XNT amount without trailing zeros or a dangling dot ("1.00000000" -> "1", "0.50" -> "0.5").
 * Strings are trimmed as given, so their precision is kept exactly.
 */
export const formatXnt = (value: string | number): string => {
  if (typeof value === 'number' && !Number.isFinite(value)) {
    throw new Error(`Not an XNT amount: ${value}`)
  }
  const text = typeof value === 'number' ? value.toFixed(XNT_DECIMALS) : value.trim()
  if (!text.includes('.')) return text
  return text.replace(/0+$/, '').replace(/\.$/, '')
}

/** Base units of a non-negative decimal string at `decimals` places; extra digits are cut off. */
export const toBaseUnits = (decimal: string, decimals: number): bigint => {
  const match = /^(\d+)(?:\.(\d*))?$/.exec(decimal.trim())
  if (!match) throw new Error(`Not a decimal amount: ${decimal}`)
  const fraction = (match[2] ?? '').slice(0, decimals).padEnd(decimals, '0')
  return BigInt(match[1] + fraction)
}

/** Decimal string of `units` base units at `decimals` places, without trailing zeros. */
export const fromBaseUnits = (units: bigint, decimals: number): string => {
  const negative = units < 0n
  const digits = (negative ? -units : units).toString().padStart(decimals + 1, '0')
  const whole = digits.slice(0, digits.length - decimals)
  const fraction = digits.slice(digits.length - decimals)
  return `${negative ? '-' : ''}${formatXnt(decimals ? `${whole}.${fraction}` : whole)}`
}
