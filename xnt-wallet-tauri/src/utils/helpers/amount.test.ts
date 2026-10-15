import { describe, expect, it } from 'vitest'
import { formatXnt, fromBaseUnits, toBaseUnits } from './amount'

describe('formatXnt', () => {
  it('drops trailing zeros and a dangling dot from decimal strings', () => {
    expect(formatXnt('1.00000000')).toBe('1')
    expect(formatXnt('0.01000000')).toBe('0.01')
    expect(formatXnt('3.48000000')).toBe('3.48')
    expect(formatXnt('0.50')).toBe('0.5')
  })

  it('keeps full precision and integers untouched', () => {
    expect(formatXnt('0.00000001')).toBe('0.00000001')
    expect(formatXnt('12345678.12345678')).toBe('12345678.12345678')
    expect(formatXnt('100')).toBe('100')
    expect(formatXnt('10.0')).toBe('10')
  })

  it('rejects numbers that are not finite', () => {
    expect(() => formatXnt(Number.NaN)).toThrow()
    expect(() => formatXnt(Number.POSITIVE_INFINITY)).toThrow()
  })

  it('renders numbers at 8 decimals before trimming', () => {
    expect(formatXnt(1)).toBe('1')
    expect(formatXnt(4.99)).toBe('4.99')
    expect(formatXnt(0.1 + 0.2)).toBe('0.3')
    expect(formatXnt(1e-8)).toBe('0.00000001')
  })
})

describe('toBaseUnits / fromBaseUnits', () => {
  it('converts exactly at the given decimals', () => {
    expect(toBaseUnits('1', 6)).toBe(1_000_000n)
    expect(toBaseUnits('0.0005', 6)).toBe(500n)
    expect(toBaseUnits('15.4', 6)).toBe(15_400_000n)
    expect(fromBaseUnits(999_500n, 6)).toBe('0.9995')
    expect(fromBaseUnits(15_399_500n, 6)).toBe('15.3995')
    expect(fromBaseUnits(2_000_000n, 6)).toBe('2')
  })

  it('cuts precision beyond the decimals and rejects non-decimals', () => {
    expect(toBaseUnits('1.12345678', 6)).toBe(1_123_456n)
    expect(() => toBaseUnits('1e-8', 6)).toThrow()
    expect(() => toBaseUnits('-1', 6)).toThrow()
  })

  it('keeps the sign of negative unit counts', () => {
    expect(fromBaseUnits(-500n, 6)).toBe('-0.0005')
  })
})
