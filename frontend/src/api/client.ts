import axios from 'axios'

const http = axios.create({ baseURL: '/api/v1' })

// Auth interceptors
http.interceptors.request.use((config) => {
  const token = localStorage.getItem('token')
  if (token) config.headers.Authorization = `Bearer ${token}`
  return config
})

http.interceptors.response.use(
  (res) => res,
  (err) => {
    if (err.response?.status === 401 && !err.config?.url?.includes('/auth/login')) {
      localStorage.removeItem('token')
      window.location.href = '/login'
    }
    return Promise.reject(err)
  }
)

// ---------- Types ----------

export interface PortConfig {
  id: number
  port: number
  label: string
  enabled: boolean
  created_at: string
}

export interface MockApi {
  id: number
  port_id: number
  tenant_id: number
  name: string
  description: string
  method: string
  path: string
  request_schema: unknown | null
  request_params: Record<string, string>
  response_status: number
  response_headers: Record<string, string>
  response_body: string
  response_delay_ms: number
  enabled: boolean
  pagination_enabled: boolean
  pagination_page_size: number
  pagination_page_param: string
  pagination_size_param: string
  pagination_data_field: string
  pagination_total_field: string
  created_at: string
  updated_at: string
}

export interface RequestLog {
  id: number
  mock_api_id: number | null
  port: number
  method: string
  path: string
  query_string: string | null
  request_headers: Record<string, string>
  request_body: string | null
  response_status: number
  response_headers: Record<string, string>
  response_body: string | null
  duration_ms: number
  client_ip: string | null
  tenant_id: number | null
  created_at: string
}

export interface SystemLog {
  id: number
  level: string
  target: string
  message: string
  fields: unknown | null
  created_at: string
}

export interface ServerInfo {
  ip: string
}

export interface UserInfo {
  id: number
  username: string
  display_name: string
  is_admin: boolean
  enabled: boolean
  created_at: string
}

export interface TenantInfo {
  id: number
  name: string
  slug: string
  enabled: boolean
  mock_count: number
  created_at: string
}

export interface UserTenant {
  id: number
  user_id: number
  tenant_id: number
  is_default: boolean
  created_at: string
}

export interface LoginResponse {
  token: string
  user: UserInfo
  tenants: TenantInfo[]
  current_tenant: TenantInfo | null
}

export interface MeResponse {
  user: UserInfo
  tenants: TenantInfo[]
  current_tenant: TenantInfo | null
}

export interface LogPage<T> {
  items: T[]
  total: number
  page: number
  page_size: number
}

export interface TestMockPayload {
  query_params: Record<string, string>
  headers: Record<string, string>
  body?: string
}

export interface TestMockResult {
  status: number
  headers: Record<string, string>
  body: string
  elapsed_ms: number
}

// ---------- Info API ----------

export const InfoApi = {
  get: () => http.get<ServerInfo>('/info'),
}

// ---------- Port API ----------

export const PortsApi = {
  list: ()                              => http.get<PortConfig[]>('/ports'),
  get:  (id: number)                   => http.get<PortConfig>(`/ports/${id}`),
  create: (port: number, label = '')   => http.post<PortConfig>('/ports', { port, label }),
  update: (id: number, label: string, enabled: boolean) =>
    http.put<PortConfig>(`/ports/${id}`, { label, enabled }),
  remove: (id: number)                 => http.delete(`/ports/${id}`),
  start:  (id: number)                 => http.post(`/ports/${id}/start`),
  stop:   (id: number)                 => http.post(`/ports/${id}/stop`),
  status: (id: number)                 => http.get<{ running: boolean }>(`/ports/${id}/status`),
}

// ---------- Mock API ----------

export const MocksApi = {
  list:   (port_id?: number, tenant_id?: number) => http.get<MockApi[]>('/mocks', { params: { port_id, tenant_id } }),
  get:    (id: number)                 => http.get<MockApi>(`/mocks/${id}`),
  create: (body: Partial<MockApi>)     => http.post<MockApi>('/mocks', body),
  update: (id: number, body: Partial<MockApi>) => http.put<MockApi>(`/mocks/${id}`, body),
  remove: (id: number)                 => http.delete(`/mocks/${id}`),
  setEnabled: (id: number, enabled: boolean) =>
    http.patch(`/mocks/${id}/enabled`, { enabled }),
  test: (id: number, payload: TestMockPayload) =>
    http.post<TestMockResult>(`/mocks/${id}/test`, payload),
}

// ---------- Log API ----------

export const LogsApi = {
  listRequests: (params = {})         => http.get<LogPage<RequestLog>>('/logs/requests', { params }),
  getRequest:   (id: number)          => http.get<RequestLog>(`/logs/requests/${id}`),
  clearRequests: ()                   => http.delete('/logs/requests'),
  listSystem:   (params = {})         => http.get<LogPage<SystemLog>>('/logs/system', { params }),
  clearSystem:  ()                    => http.delete('/logs/system'),
}

// ---------- Auth API ----------

export const login = (username: string, password: string): Promise<LoginResponse> =>
  http.post('/auth/login', { username, password }).then(r => r.data)

export const logout = (): Promise<void> =>
  http.post('/auth/logout').then(() => undefined)

export const switchTenant = (tenant_id: number): Promise<{ token: string }> =>
  http.post('/auth/switch-tenant', { tenant_id }).then(r => r.data)

export const getMe = (): Promise<MeResponse> =>
  http.get('/auth/me').then(r => r.data)

export const setDefaultTenant = (tenant_id: number): Promise<void> =>
  http.put('/auth/me/default-tenant', { tenant_id }).then(() => undefined)

// ---------- Admin — Tenants ----------

export const adminListTenants = (): Promise<TenantInfo[]> =>
  http.get('/admin/tenants').then(r => r.data)

export const adminCreateTenant = (data: { name: string; slug: string }): Promise<TenantInfo> =>
  http.post('/admin/tenants', data).then(r => r.data)

export const adminUpdateTenant = (id: number, data: { name: string; slug: string; enabled: boolean }): Promise<TenantInfo> =>
  http.put(`/admin/tenants/${id}`, data).then(r => r.data)

export const adminDeleteTenant = (id: number): Promise<void> =>
  http.delete(`/admin/tenants/${id}`).then(() => undefined)

// ---------- Admin — Users ----------

export const adminListUsers = (): Promise<UserInfo[]> =>
  http.get('/admin/users').then(r => r.data)

export const adminCreateUser = (data: { username: string; display_name: string; password: string; is_admin: boolean }): Promise<UserInfo> =>
  http.post('/admin/users', data).then(r => r.data)

export const adminUpdateUser = (id: number, data: { display_name: string; is_admin: boolean }): Promise<UserInfo> =>
  http.put(`/admin/users/${id}`, data).then(r => r.data)

export const adminDeleteUser = (id: number): Promise<void> =>
  http.delete(`/admin/users/${id}`).then(() => undefined)

export const adminDisableUser = (id: number): Promise<void> =>
  http.post(`/admin/users/${id}/disable`).then(() => undefined)

export const adminEnableUser = (id: number): Promise<void> =>
  http.post(`/admin/users/${id}/enable`).then(() => undefined)

export const adminResetPassword = (id: number, password: string): Promise<void> =>
  http.post(`/admin/users/${id}/reset-password`, { password }).then(() => undefined)

export const adminListUserTenants = (userId: number): Promise<UserTenant[]> =>
  http.get(`/admin/users/${userId}/tenants`).then(r => r.data)

export const adminAssignTenant = (userId: number, tenant_id: number): Promise<UserTenant> =>
  http.post(`/admin/users/${userId}/tenants`, { tenant_id }).then(r => r.data)

export const adminRemoveTenant = (userId: number, tenantId: number): Promise<void> =>
  http.delete(`/admin/users/${userId}/tenants/${tenantId}`).then(() => undefined)
