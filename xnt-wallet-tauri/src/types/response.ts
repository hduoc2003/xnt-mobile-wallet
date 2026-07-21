import type { Utxo } from './wallet'

export interface JsonRpcResponse<T = unknown> {
  jsonrpc?: string
  id?: number | string
  result: T
}

export interface UtxosResult {
  utxos: Utxo[]
}

export interface BalanceResult {
  balance: string
  pendingBalance: string
  totalBalance?: string
}

export interface BlockHeightResult {
  height: number
}

export interface NetworkResult {
  network: string
}

export interface BroadcastResult {
  transactionHex: string
}

export interface RateXntUsdResult {
  rate: number
}
