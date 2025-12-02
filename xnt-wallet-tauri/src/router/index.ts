import { createRouter, createWebHistory, createWebHashHistory } from 'vue-router'
import { routes } from './route'

const router = createRouter({
    history: import.meta.env.DEV ? createWebHistory() : createWebHashHistory(),
    routes,
})

router.beforeEach((to, from, next) => {
    next()
})

export default router
