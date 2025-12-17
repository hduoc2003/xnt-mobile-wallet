import { getXntPrice } from '@/api/neptuneApi'
import { useTickerStore } from '@/stores'

export const useTickers = () => {
  const tickerStore = useTickerStore()

  const fetchXntPrice = async () => {
    try {
      const { data } = await getXntPrice('https://api.coinmarketcap.com/data-api/v3/cryptocurrency/detail/lite?id=39109')
      const price = data?.statistics?.price
      tickerStore.setXntPrice(price)
      localStorage.setItem('xntPriceLastFetchTime', Date.now().toString())
      localStorage.setItem('xntPrice', price?.toString() || '')
    } catch (err: any) {
      console.log(err)
    }
  }

  return {
    fetchXntPrice
  }
}