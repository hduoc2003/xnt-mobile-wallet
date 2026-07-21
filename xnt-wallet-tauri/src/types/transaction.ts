export type TxStatus = 'confirmed' | 'pending' | 'failed' | 'unknown'

export interface TransactionDetail {
  id: string
  toAddress: string
  amount: number
  fee: number
  time: string
  status?: TxStatus
}
