import { getXntPrice } from '@/api/neptuneApi'
import { useTickerStore } from '@/stores'
import { PRICE_URL } from '@/utils'

export const useTickers = () => {
  const tickerStore = useTickerStore()

  const fetchXntPrice = async () => {
    try {
      const { data } = await getXntPrice(PRICE_URL)
      const price = data?.statistics?.price
      const change24h = data?.statistics?.priceChangePercentage24h ?? null
      tickerStore.setXntPrice(price)
      tickerStore.setXntChange24h(change24h)
      localStorage.setItem('xntPriceChange24h', change24h === null ? '' : String(change24h))
      localStorage.setItem('xntPriceLastFetchTime', Date.now().toString())
      localStorage.setItem('xntPrice', price?.toString() || '')
    } catch (err: any) {
      console.log(err)
    }
  }

  return {
    fetchXntPrice,
  }
}
