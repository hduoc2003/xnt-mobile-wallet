import { DEFAULT_MIN_BLOCK_HEIGHT } from '@/utils'

export function toFiniteNumber(
    value: number,
    defaultValue: number = DEFAULT_MIN_BLOCK_HEIGHT
): number {
    return Number.isFinite(value) ? value : defaultValue
}

