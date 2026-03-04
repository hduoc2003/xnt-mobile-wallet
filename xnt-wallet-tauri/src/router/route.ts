import type { RouteRecordRaw } from 'vue-router'
import { Layout } from '@/components/commons'
import * as Pages from '@/views'
import { useNeptuneStore } from '@/stores'

const ifAuthenticated = (_to: any, _from: any, next: any) => {
  const neptuneStore = useNeptuneStore()
  const hasWallet = neptuneStore.hasWallet
  const hasKeystore = !!neptuneStore.getKeystoreFileName

  if (hasWallet || hasKeystore) {
    next()
    return
  }

  next('/auth')
}

export const routes: RouteRecordRaw[] = [
  {
    path: '/auth',
    name: 'auth',
    component: Pages.AuthPage,
  },
  {
    path: '/',
    component: Layout,
    beforeEnter: ifAuthenticated,
    children: [
      { path: '', name: 'wallet', component: Pages.WalletPage },
      { path: '/wallet/send', name: 'send-transaction', component: Pages.SendTransactionPage },
      { path: '/wallet/backup-seed', name: 'backup-seed', component: Pages.BackupSeedPage },
      { path: '/utxo', name: 'utxo', component: Pages.UTXOPage },
      { path: '/wallet/merge', name: 'merge-utxos', component: Pages.MergeUtxosPage },
      { path: '/network', name: 'network', component: Pages.NetworkPage },
      {
        path: '/transaction-history',
        name: 'transaction-history',
        component: Pages.TransactionHistoryPage,
      },
    ],
  },
  {
    path: '/:pathMatch(.*)*',
    name: 'not-found',
    component: Pages.NotFoundPage,
  },
]
