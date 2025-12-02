import * as Page from '@/views'
import { useNeptuneStore } from '@/stores/neptuneStore'
import { useNeptuneWallet } from '@/composables/useNeptuneWallet'
import { useAuthStore } from '@/stores'

const ifAuthenticated = (to: any, from: any, next: any) => {
    const neptuneStore = useNeptuneStore()

    // If we have a wallet, proceed as normal
    if (neptuneStore.hasWallet) {
        next()
        return
    }

    // If no wallet yet, go to auth screen (this is expected on first launch)
    next('/auth')
}

const ifNotAuthenticated = async (to: any, from: any, next: any) => {
    const neptuneWallet = useNeptuneWallet()
    const authStore = useAuthStore()

    const exists = await neptuneWallet.checkKeystore()
    if (exists) authStore.goToLogin()
    next()
}

export const routes: any = [
    {
        path: '/',
        name: 'home',
        component: Page.Home,
        beforeEnter: ifAuthenticated,
    },
    {
        path: '/auth',
        name: 'auth',
        component: Page.Auth,
        beforeEnter: ifNotAuthenticated,
    },
    {
        path: '/:pathMatch(.*)*',
        component: Page.NotFound,
        name: 'page-not-found',
    },
]
