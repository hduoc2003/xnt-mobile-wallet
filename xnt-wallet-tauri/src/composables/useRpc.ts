import { useNeptuneStore } from '@/stores'

export function useRpc() {
  const store = useNeptuneStore()

  const loadRpcEndpoint = async () => {
    try {
      if (typeof (window as any).walletApi?.getCustomRpcEndpoint === 'function') {
        const endpoint = await (window as any).walletApi.getCustomRpcEndpoint()
        store.setRpcUrl(endpoint || null)
        return endpoint || null
      }
      return null
    } catch (err) {
      await (window as any).logger?.error('Error loading RPC endpoint')
      store.setRpcUrl(null)
      return null
    }
  }

  const saveRpcEndpoint = async (endpoint: string) => {
    try {
      if (typeof (window as any).walletApi?.setRpcEndpoint === 'function') {
        const result = await (window as any).walletApi.setRpcEndpoint(endpoint)
        if (!result.success) {
          throw new Error(result.error || 'Failed to save endpoint')
        }
      }

      store.setRpcUrl(endpoint || null)
    } catch (err: any) {
      await (window as any).logger?.error('Error saving RPC endpoint')
      throw new Error(err.message)
    }
  }

  return {
    loadRpcEndpoint,
    saveRpcEndpoint,
  }
}
