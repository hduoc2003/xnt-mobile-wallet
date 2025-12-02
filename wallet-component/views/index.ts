export const AuthPage = () => import('@/views/Auth/AuthView.vue')
export const WalletPage = () => import('@/views/Wallet/WalletView.vue')
export const SendTransactionPage = () => import('@/views/Wallet/SendTransactionView.vue')
export const BackupSeedPage = () => import('@/views/Wallet/BackupSeedView.vue')
export const UTXOPage = () => import('@/views/UTXO/UTXOView.vue')
export const TransactionHistoryPage = () =>
  import('@/views/TransactionHistory/TransactionHistoryView.vue')
export const NetworkPage = () => import('@/views/Network/NetworkView.vue')
export const NotFoundPage = () => import('@/views/NotFoundView.vue')
