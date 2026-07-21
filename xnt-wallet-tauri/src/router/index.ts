import { createRouter, createWebHashHistory, createWebHistory } from 'vue-router'
import { routes } from './route'

const router = createRouter({
  history: import.meta.env.DEV ? createWebHistory() : createWebHashHistory(),
  routes,
})

// Navigation guards
router.beforeEach((_to, _from, next) => {
  next()
})

// Log navigation/lazy-load failures (e.g. a route chunk that fails to mount) so
// they aren't swallowed as bare unhandled rejections.
router.onError((error: any) => {
  const stack = error?.stack ? `\n${error.stack}` : ''
  ;(window as any).logger?.error(`[router] navigation error: ${error?.message || error}${stack}`)
})

export default router
