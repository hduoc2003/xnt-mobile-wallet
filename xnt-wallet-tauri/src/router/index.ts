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

export default router
