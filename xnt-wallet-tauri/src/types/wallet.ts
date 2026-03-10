export interface GenerateSeedResult {
  receiver_identifier: string
  seed_phrase: string[]
}

export interface SpendingKeyResult {
  receiver_identifier: string
  spending_key_hex: string
}

export interface PayloadBuildTransaction {
  inputAdditionRecords: string[]
  outputAddresses: string[]
  outputAmounts: string[]
  fee: string
  timelockReleaseDate?: number | string
  spendingKeyHex?: string
  /** Per-input spending keys for mixed-key transactions (e.g. merging Generation + dCTIDH UTXOs). */
  spendingKeyHexes?: string[]
}

export interface WalletState {
  receiverId: string | null
  spendingKey?: string | null
  generationSpendingKey?: string | null
  address: string | null
  dctidh_address?: string | null
  generation_address?: string | null
  network: 'mainnet' | 'testnet'
  balance?: string | null
  pendingBalance?: string | null
  utxos?: Utxo[]
  minBlockHeight?: number | null
}

export interface Utxo {
  additionRecord: string
  utxoDigest?: string
  amount: string
  blockHeight: number
  isTimelocked?: boolean
  releaseDateMs?: number | null
  keyType?: string
}