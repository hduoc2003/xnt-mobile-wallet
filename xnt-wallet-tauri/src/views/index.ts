// New UI page entrypoints used by the router
export const AuthPage = () => import('@/views/Auth/AuthView.vue')
export const WalletPage = () => import('@/views/Wallet/WalletView.vue')
export const SendTransactionPage = () => import('@/views/Wallet/SendTransactionView.vue')
export const BackupSeedPage = () => import('@/views/Wallet/BackupSeedView.vue')
export const UTXOPage = () => import('@/views/UTXO/UTXOView.vue')
export const NetworkPage = () => import('@/views/Network/NetworkView.vue')
export const TransactionHistoryPage = () =>
  import('@/views/TransactionHistory/TransactionHistoryView.vue')
export const BridgeDepositDetailPage = () =>
  import('@/views/TransactionHistory/BridgeDepositDetailView.vue')
export const BridgeDepositPage = () => import('@/views/Wallet/BridgeDepositView.vue')
export const MergeUtxosPage = () => import('@/views/Wallet/MergeUtxosView.vue')
export const NotFoundPage = () => import('@/views/NotFound/NotFoundView.vue')
