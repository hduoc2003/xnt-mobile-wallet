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
  isPending?: boolean
}

export interface PayloadBuildBridgeDeposit {
  inputAdditionRecords: string[]
  /** Miden address (`mtst1...`, optional `miden:` prefix) or 30-hex account id. */
  midenAddress: string
  amount: string
  fee: string
  spendingKeyHex: string
  spendingKeyHexes: string[]
  lowMemoryProving: boolean
}

export interface RelayerDepositStatus {
  minted: boolean
  status: 'minted' | 'submitted' | 'refused' | 'waiting'
  reason: 'invalid' | 'below_mint_fee' | 'no_snapshot' | null
  mintFee: number | null
  /** WXNT in the depositor's note, once minted. */
  mintedWxnt: number | null
  mintableUpTo: number | null
  paused: boolean
  /** Leviathan mint tx / WXNT note. */
  mintTxId: string | null
  mintNoteId: string | null
}

export type BridgeDepositStatus =
  | { state: 'submitted'; misses: number }
  | { state: 'confirmed'; blockHeight: number | null }
  | { state: 'minted'; blockHeight: number | null }
  | { state: 'dropped' }

export interface BridgeDepositRecord {
  utxoCommitment: string
  txId: string
  /** Recipient as entered; null for deposits recorded before it was stored. */
  midenAddress: string | null
  amount: string
  fee: string | null
  outputCommitments: string[]
  createdMs: number
  status: BridgeDepositStatus
  relayer: RelayerDepositStatus | null
  relayerError: string | null
}
