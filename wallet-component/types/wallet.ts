export interface GenerateSeedResult {
  receiver_identifier: string
  seed_phrase: string[]
}

export interface ViewKeyResult {
  receiver_identifier: string
  view_key_hex: string
  spending_key_hex: string
}

export interface BalanceResult {
  balance: string
  pendingBalance: string
}

export interface PayloadBuildTransaction {
  inputAdditionRecords: string[]
  outputAddresses: string[]
  outputAmounts: string[]
  fee: string
}

export interface WalletState {
  network: 'mainnet' | 'testnet'
  receiverId: string
  viewKey: string
  spendingKey: string
  address: string
}

