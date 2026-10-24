export interface GenerateSeedResult {
  receiver_identifier: string
  seed_phrase: string[]
}

export interface SpendingKeyResult {
  receiver_identifier: string
  spending_key_hex: string
}

export interface PayloadBuildTransaction {
  inputUtxoIds: string[]
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
  utxoId: string
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
  inputUtxoIds: string[]
  /** Leviathan address (`mtst1...`, optional `miden:` prefix) or 30-hex account id. */
  leviathanAddress: string
  amount: string
  fee: string
  spendingKeyHex: string
  spendingKeyHexes: string[]
  lowMemoryProving: boolean
  /** The wallet account spending, captured with its UTXOs. */
  walletAccount: string
}

export interface PayloadBuildLegacyDctidhMigration {
  dctidhKeyHex: string
  lowMemoryProving: boolean
}

export interface LegacyDctidhMigration {
  success: true
  transaction: unknown
  outputUtxoDigests: string[]
  txId: string
  toAddress: string
  numInputs: number
  amount: string
  fee: string
  /** Legacy UTXOs left for a later migration, `numDust` included. */
  numWaiting: number
  /** Legacy UTXOs worth less than their fee share; never moved. */
  numDust: number
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
  leviathanAddress: string | null
  amount: string
  fee: string | null
  outputCommitments: string[]
  createdMs: number
  status: BridgeDepositStatus
  relayer: RelayerDepositStatus | null
  relayerError: string | null
  walletAccount: string
}

export type ChainCheck =
  | { state: 'same' }
  | { state: 'firstRecord' }
  | { state: 'reset'; previous: { network: string; block1Digest: string } }
  | { state: 'reorg'; height: number }

export type SeedSource = 'created' | 'phrase' | 'backupFile' | 'nodeWalletDat'

export interface SeedEntry {
  id: string
  keystoreFile: string
  source: SeedSource
  backedUp: boolean
}

export interface AccountEntry {
  id: string
  seedId: string
  index: number
  /** null shows the default "Account N". */
  name: string | null
  hidden: boolean
}

export interface Registry {
  seeds: SeedEntry[]
  accounts: AccountEntry[]
  active: string
}

/** File sources carry the file's text: Android's picker gives `content://` URIs, not paths. */
export type ImportSource =
  | { kind: 'phrase'; words: string[] }
  | { kind: 'backupFile'; content: string; filePassword: string }
  | { kind: 'nodeWalletDat'; content: string }

export interface AccountKeys {
  index: number
  generationKeyHex: string
  dctidhKeyHex: string
  generationAddress: string
  dctidhAddress: string
}
