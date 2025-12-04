import { getXntPrice } from '@/api/neptuneApi'
import { useTickerStore } from '@/stores'

export const useTickers = () => {
  const tickerStore = useTickerStore()

  const fetchXntPrice = async () => {
    try {
      const response = await getXntPrice()
      tickerStore.setXntPrice(response?.result?.price)
    } catch (err: any) {
      await (window as any).logger?.error('Error fetching XNT price: ', err.message)
      throw err
    }
  }

  return {
    fetchXntPrice
  }
}
