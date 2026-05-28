import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { login as apiLogin, switchTenant as apiSwitchTenant, getMe, type MockPermissions, type UserInfo, type TenantInfo, type UserTenant } from '../api/client'

export type { MockPermissions, UserInfo, TenantInfo, UserTenant }

const defaultMockPermissions: MockPermissions = {
  can_create_mock: true,
  can_edit_mock: true,
  can_delete_mock: true,
  can_test_mock: true,
}

export const useAuthStore = defineStore('auth', () => {
  const token = ref<string | null>(localStorage.getItem('token'))
  const user = ref<UserInfo | null>(null)
  const tenants = ref<TenantInfo[]>([])
  const userTenants = ref<UserTenant[]>([])
  const currentTenant = ref<TenantInfo | null>(null)

  const isLoggedIn = computed(() => !!token.value)
  const isAdmin = computed(() => user.value?.is_admin ?? false)
  const currentMockPermissions = computed<MockPermissions>(() => {
    if (isAdmin.value) return defaultMockPermissions
    const tenantId = currentTenant.value?.id
    if (!tenantId) return defaultMockPermissions
    const userTenant = userTenants.value.find(ut => ut.tenant_id === tenantId)
    return userTenant
      ? {
          can_create_mock: userTenant.can_create_mock,
          can_edit_mock: userTenant.can_edit_mock,
          can_delete_mock: userTenant.can_delete_mock,
          can_test_mock: userTenant.can_test_mock,
        }
      : defaultMockPermissions
  })

  function setToken(t: string) {
    token.value = t
    localStorage.setItem('token', t)
  }

  function clearAuth() {
    token.value = null
    user.value = null
    tenants.value = []
    userTenants.value = []
    currentTenant.value = null
    localStorage.removeItem('token')
  }

  async function loginAction(username: string, password: string) {
    const res = await apiLogin(username, password)
    setToken(res.token)
    user.value = res.user
    tenants.value = res.tenants
    userTenants.value = res.user_tenants ?? []
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
      userTenants.value = res.user_tenants ?? []
      currentTenant.value = res.current_tenant
    } catch {
      clearAuth()
    }
  }

  function logout() {
    clearAuth()
  }

  return {
    token, user, tenants, userTenants, currentTenant,
    isLoggedIn, isAdmin, currentMockPermissions,
    loginAction, switchTenantAction, fetchMe, logout, setToken, clearAuth,
  }
})
