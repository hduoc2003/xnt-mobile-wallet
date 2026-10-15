import { formatXnt } from '@/utils/helpers/amount'

export const useFormat = () => {
  const formatAddress = (address: string) => {
    if (!address || address.length <= 20) return address
    return `${address.slice(0, 10)}…${address.slice(-10)}`
  }

  const formatAmount = (value: number, precision: number = 4) => {
    if (isNaN(value) || !isFinite(value)) return '0'
    return value.toFixed(precision)
  }

  return {
    formatXnt,
    formatAddress,
    formatAmount,
  }
}
