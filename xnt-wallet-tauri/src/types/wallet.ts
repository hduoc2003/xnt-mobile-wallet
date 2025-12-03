export interface GenerateSeedResult {
  receiver_identifier: string
  seed_phrase: string[]
}

export interface ViewKeyResult {
  receiver_identifier: string
  view_key_hex: string
  spending_key_hex: string
}

export interface PayloadBuildTransaction {
  inputAdditionRecords: string[]
  outputAddresses: string[]
  outputAmounts: string[]
  fee: string
}

export interface WalletState {
  receiverId: string | null
  viewKey: string | null
  spendingKey?: string | null
  address: string | null
  network: 'mainnet' | 'testnet'
  balance?: string | null
  pendingBalance?: string | null
  utxos?: Utxo[]
  minBlockHeight?: number | null
}

export interface Utxo {
  additionRecord: string
  amount: string
  blockHeight: number
  utxoHash: string
}