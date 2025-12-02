// Network Status Interface
export interface NetworkStatus {
    network: string
    daaScore: number
    dagHeader: number
    dagBlocks: number
    difficulty: number
    medianOffset: string
    medianTimeUTC: string
}

// Wallet Tab Props
export interface WalletTabProps {
    network: string
}

export interface DebugTabProps {}

export interface TransactionsTabProps {}
