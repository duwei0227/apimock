import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { login as apiLogin, switchTenant as apiSwitchTenant, getMe, type UserInfo, type TenantInfo } from '../api/client'

export type { UserInfo, TenantInfo }

export const useAuthStore = defineStore('auth', () => {
  const token = ref<string | null>(localStorage.getItem('token'))
  const user = ref<UserInfo | null>(null)
  const tenants = ref<TenantInfo[]>([])
  const currentTenant = ref<TenantInfo | null>(null)

  const isLoggedIn = computed(() => !!token.value)
  const isAdmin = computed(() => user.value?.is_admin ?? false)

  function setToken(t: string) {
    token.value = t
    localStorage.setItem('token', t)
  }

  function clearAuth() {
    token.value = null
    user.value = null
    tenants.value = []
    currentTenant.value = null
    localStorage.removeItem('token')
  }

  async function loginAction(username: string, password: string) {
    const res = await apiLogin(username, password)
    setToken(res.token)
    user.value = res.user
    tenants.value = res.tenants
    currentTenant.value = res.current_tenant
  }

  async function switchTenantAction(tenantId: number) {
    const res = await apiSwitchTenant(tenantId)
    setToken(res.token)
    const t = tenants.value.find(t => t.id === tenantId) ?? null
    currentTenant.value = t
  }

  async function fetchMe() {
    if (!token.value) return
    try {
      const res = await getMe()
      user.value = res.user
      tenants.value = res.tenants
      currentTenant.value = res.current_tenant
    } catch {
      clearAuth()
    }
  }

  function logout() {
    clearAuth()
  }

  return {
    token, user, tenants, currentTenant,
    isLoggedIn, isAdmin,
    loginAction, switchTenantAction, fetchMe, logout, setToken, clearAuth,
  }
})
