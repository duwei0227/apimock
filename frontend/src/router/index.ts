import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from '../stores/auth'

declare module 'vue-router' {
  interface RouteMeta {
    title?: string
    public?: boolean
    adminOnly?: boolean
  }
}

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/login', component: () => import('../views/LoginView.vue'), meta: { public: true } },
    { path: '/',          component: () => import('../views/DashboardView.vue'), meta: { title: 'Dashboard' } },
    { path: '/ports',     component: () => import('../views/PortsView.vue'),     meta: { title: 'Ports' } },
    { path: '/mocks',     component: () => import('../views/MocksView.vue'),     meta: { title: 'Mocks' } },
    { path: '/logs',      component: () => import('../views/LogsView.vue'),      meta: { title: 'Logs' } },
    { path: '/functions', component: () => import('../views/FunctionsView.vue'), meta: { title: 'Functions' } },
    { path: '/admin',     component: () => import('../views/AdminView.vue'),     meta: { title: 'Admin', adminOnly: true } },
  ],
})

router.beforeEach((to) => {
  const auth = useAuthStore()
  if (to.meta.public) return true
  if (!auth.isLoggedIn) return '/login'
  if (to.meta.adminOnly && !auth.isAdmin) return '/'
  return true
})

export default router
