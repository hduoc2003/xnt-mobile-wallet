import { ref } from 'vue'
import { RPC_URL } from '@/utils'

export interface BridgeFees {
  /** Current WXNT mint fee in WXNT base units. */
  mintFeeWxnt: bigint
  wxntDecimals: number
}

/** Mint fee and WXNT decimals from the relayer's `/bridge/capacity`, on the RPC endpoint's origin. */
export const useBridgeFees = () => {
  const fees = ref<BridgeFees>()
  const feesError = ref<string>()

  const loadFees = async () => {
    try {
      const walletApi = (window as any).walletApi
      const custom: string | null = await walletApi.getCustomRpcEndpoint()
      const origin = new URL(custom ?? RPC_URL).origin
      const body = await walletApi.proxyHttpGet(`${origin}/bridge/capacity`)
      if (body?.success === false) throw new Error(String(body.error))
      if (typeof body?.mintFeeWxnt !== 'number' || typeof body?.wxntDecimals !== 'number') {
        throw new Error('the relayer does not publish the mint fee yet')
      }
      fees.value = { mintFeeWxnt: BigInt(body.mintFeeWxnt), wxntDecimals: body.wxntDecimals }
      feesError.value = undefined
    } catch (error: unknown) {
      fees.value = undefined
      feesError.value = error instanceof Error ? error.message : String(error)
    }
  }

  return { fees, feesError, loadFees }
}
