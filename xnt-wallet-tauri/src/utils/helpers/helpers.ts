import { DEFAULT_MIN_BLOCK_HEIGHT } from '@/utils'

export function toFiniteNumber(
  value: number,
  defaultValue: number = DEFAULT_MIN_BLOCK_HEIGHT,
): number {
  return Number.isFinite(value) ? value : defaultValue
}

/** Text of a JSON-RPC error object (`{ code, message, data? }`) or any other thrown value. */
export const describeError = (error: unknown): string => {
  if (typeof error !== 'object' || error === null) return String(error)
  const message = 'message' in error ? String(error.message) : JSON.stringify(error)
  return 'data' in error && error.data !== undefined
    ? `${message}: ${JSON.stringify(error.data)}`
    : message
}
