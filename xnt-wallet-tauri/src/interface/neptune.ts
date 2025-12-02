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

export interface GenerateSeedResult {
    seed_phrase: string[]
    receiver_identifier: string
}

export interface ViewKeyResult {
    receiver_identifier: string
    spending_key_hex: string
    view_key_hex: string
    success?: boolean
}

export interface PayloadBuildTransaction {
    spendingKeyHex?: string
    inputAdditionRecords?: string[]
    outputAddresses?: string[]
    outputAmounts?: string[]
    fee?: string
}

export interface PayloadBroadcastSignedTransaction {
    transactionHex: string
}

export interface Utxo {
    additionRecord: string
    amount: string
    blockHeight: number
    utxoHash: string
}

export interface BalanceResult {
    balance: string | null
    pendingBalance: string | null
}
