<template>
  <div class="space-y-4">
    <h2 class="text-xl font-semibold">Admin</h2>

    <Tabs v-model:value="activeTab">
      <TabList>
        <Tab value="tenants">Tenants</Tab>
        <Tab value="users">Users</Tab>
      </TabList>
      <TabPanels>

        <!-- ===== TENANTS TAB ===== -->
        <TabPanel value="tenants">
          <div class="flex justify-end mb-3">
            <Button label="New Tenant" icon="pi pi-plus" size="small" @click="openCreateTenant" />
          </div>
          <DataTable :value="tenants" :loading="tenantsLoading" stripedRows class="shadow-sm rounded-xl overflow-hidden">
            <Column field="name" header="Name" />
            <Column field="slug" header="Slug" class="font-mono" />
            <Column header="Status" style="width:100px">
              <template #body="{ data }">
                <Badge :value="data.enabled ? 'Enabled' : 'Disabled'" :severity="data.enabled ? 'success' : 'secondary'" />
              </template>
            </Column>
            <Column header="Actions" style="width:160px">
              <template #body="{ data }">
                <div class="flex gap-1">
                  <Button icon="pi pi-pencil" size="small" text rounded @click="openEditTenant(data)" />
                  <Button icon="pi pi-trash" severity="danger" size="small" text rounded @click="confirmDeleteTenant(data)" />
                </div>
              </template>
            </Column>
          </DataTable>
        </TabPanel>

        <!-- ===== USERS TAB ===== -->
        <TabPanel value="users">
          <div class="flex justify-end mb-3">
            <Button label="New User" icon="pi pi-plus" size="small" @click="openCreateUser" />
          </div>
          <DataTable :value="users" :loading="usersLoading" stripedRows class="shadow-sm rounded-xl overflow-hidden">
            <Column field="username" header="Username" />
            <Column field="display_name" header="Display Name" />
            <Column header="Role" style="width:90px">
              <template #body="{ data }">
                <Badge :value="data.is_admin ? 'Admin' : 'User'" :severity="data.is_admin ? 'warn' : 'info'" />
              </template>
            </Column>
            <Column header="Status" style="width:100px">
              <template #body="{ data }">
                <Badge :value="data.enabled ? 'Active' : 'Disabled'" :severity="data.enabled ? 'success' : 'secondary'" />
              </template>
            </Column>
            <Column header="Tenants" style="width:180px">
              <template #body="{ data }">
                <Badge v-if="data.is_admin" value="All Tenants" severity="warn" />
                <Button v-else icon="pi pi-users" label="Tenants" size="small" text @click="openUserTenants(data)" />
              </template>
            </Column>
            <Column header="Actions" style="width:220px">
              <template #body="{ data }">
                <div class="flex gap-1">
                  <Button v-if="!data.is_admin" icon="pi pi-pencil" size="small" text rounded @click="openEditUser(data)" />
                  <Button
                    v-if="!data.is_admin"
                    :icon="data.enabled ? 'pi pi-ban' : 'pi pi-check-circle'"
                    :severity="data.enabled ? 'warn' : 'success'"
                    size="small" text rounded
                    :title="data.enabled ? 'Disable' : 'Enable'"
                    @click="toggleUserEnabled(data)"
                  />
                  <Button icon="pi pi-key" size="small" text rounded title="Reset password" @click="openResetPwd(data)" />
                  <Button v-if="!data.is_admin" icon="pi pi-trash" severity="danger" size="small" text rounded @click="confirmDeleteUser(data)" />
                </div>
              </template>
            </Column>
          </DataTable>
        </TabPanel>
      </TabPanels>
    </Tabs>

    <!-- ===== Tenant Dialog ===== -->
    <Dialog v-model:visible="tenantDialogVisible" :header="editingTenant ? 'Edit Tenant' : 'New Tenant'" modal :style="{ width: '28rem' }">
      <div class="flex flex-col gap-3 pt-2">
        <div class="flex flex-col gap-1">
          <label class="text-sm font-medium">Name</label>
          <InputText v-model="tenantForm.name" placeholder="Tenant name" />
        </div>
        <div class="flex flex-col gap-1">
          <label class="text-sm font-medium">Slug</label>
          <InputText v-model="tenantForm.slug" placeholder="url-slug" class="font-mono" :disabled="!!editingTenant" />
          <small v-if="!editingTenant" class="text-surface-400">
            Slug 是租户的唯一 URL 标识符，创建后不可修改。<br>
            仅允许小写字母、数字和连字符（例如 <code class="font-mono">my-team</code>）。
          </small>
        </div>
        <div v-if="editingTenant" class="flex items-center gap-2">
          <ToggleSwitch v-model="tenantForm.enabled" />
          <label class="text-sm">Enabled</label>
        </div>
      </div>
      <template #footer>
        <Button label="Cancel" text @click="tenantDialogVisible = false" />
        <Button label="Save" @click="saveTenant" />
      </template>
    </Dialog>

    <!-- ===== User Dialog ===== -->
    <Dialog v-model:visible="userDialogVisible" :header="editingUser ? 'Edit User' : 'New User'" modal :style="{ width: '28rem' }">
      <div class="flex flex-col gap-3 pt-2">
        <div v-if="!editingUser" class="flex flex-col gap-1">
          <label class="text-sm font-medium">Username</label>
          <InputText v-model="userForm.username" placeholder="username" />
        </div>
        <div class="flex flex-col gap-1">
          <label class="text-sm font-medium">Display Name</label>
          <InputText v-model="userForm.display_name" placeholder="Display name" />
        </div>
        <div v-if="!editingUser" class="flex flex-col gap-1">
          <label class="text-sm font-medium">Password</label>
          <Password v-model="userForm.password" :feedback="false" toggleMask />
        </div>
        <div v-if="!editingUser" class="flex flex-col gap-1">
          <label class="text-sm font-medium">Tenants</label>
          <MultiSelect
            v-model="userForm.tenant_ids"
            :options="tenants"
            optionLabel="name"
            optionValue="id"
            placeholder="Select tenants (optional)"
            display="chip"
            class="w-full"
          />
        </div>
        <div class="flex items-center gap-2">
          <ToggleSwitch v-model="userForm.is_admin" />
          <label class="text-sm">Admin</label>
        </div>
      </div>
      <template #footer>
        <Button label="Cancel" text @click="userDialogVisible = false" />
        <Button label="Save" @click="saveUser" />
      </template>
    </Dialog>

    <!-- ===== Reset Password Dialog ===== -->
    <Dialog v-model:visible="resetPwdVisible" header="Reset Password" modal :style="{ width: '24rem' }">
      <div class="flex flex-col gap-3 pt-2">
        <p class="text-sm text-surface-500">Set new password for <strong>{{ resetPwdTarget?.username }}</strong></p>
        <Password v-model="newPassword" :feedback="false" toggleMask />
      </div>
      <template #footer>
        <Button label="Cancel" text @click="resetPwdVisible = false" />
        <Button label="Reset" severity="danger" @click="doResetPassword" />
      </template>
    </Dialog>

    <!-- ===== User Tenants Dialog ===== -->
    <Dialog v-model:visible="userTenantsVisible" :header="`Tenants for ${userTenantsTarget?.username}`" modal :style="{ width: '32rem' }">
      <div class="flex flex-col gap-3 pt-2">
        <DataTable :value="userTenantList" size="small" stripedRows>
          <Column header="Tenant">
            <template #body="{ data }">{{ tenantNameById(data.tenant_id) }}</template>
          </Column>
          <Column header="Default" style="width:80px">
            <template #body="{ data }">
              <Badge v-if="data.is_default" value="default" severity="success" />
            </template>
          </Column>
          <Column header="" style="width:80px">
            <template #body="{ data }">
              <Button
                icon="pi pi-trash"
                severity="danger"
                size="small"
                text
                rounded
                :disabled="data.is_default"
                :title="data.is_default ? 'Cannot remove the default tenant' : 'Remove'"
                @click="removeUserTenant(data.tenant_id)"
              />
            </template>
          </Column>
        </DataTable>
        <div class="flex gap-2 mt-2">
          <Select
            v-model="assignTenantId"
            :options="assignableTenants"
            optionLabel="name"
            optionValue="id"
            placeholder="Select tenant"
            class="flex-1"
          />
          <Button label="Add" icon="pi pi-plus" @click="doAssignTenant" :disabled="!assignTenantId" />
        </div>
      </div>
    </Dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import DataTable from 'primevue/datatable'
import Column from 'primevue/column'
import Badge from 'primevue/badge'
import Button from 'primevue/button'
import Dialog from 'primevue/dialog'
import InputText from 'primevue/inputtext'
import Password from 'primevue/password'
import ToggleSwitch from 'primevue/toggleswitch'
import Select from 'primevue/select'
import MultiSelect from 'primevue/multiselect'
import Tabs from 'primevue/tabs'
import Tab from 'primevue/tab'
import TabList from 'primevue/tablist'
import TabPanels from 'primevue/tabpanels'
import TabPanel from 'primevue/tabpanel'
import { useConfirm } from 'primevue/useconfirm'
import { useToast } from 'primevue/usetoast'
import {
  adminListTenants, adminCreateTenant, adminUpdateTenant, adminDeleteTenant,
  adminListUsers, adminCreateUser, adminUpdateUser, adminDeleteUser,
  adminDisableUser, adminEnableUser, adminResetPassword,
  adminListUserTenants, adminAssignTenant, adminRemoveTenant,
  type TenantInfo, type UserInfo, type UserTenant,
} from '../api/client'

const confirm = useConfirm()
const toast = useToast()
const activeTab = ref('tenants')

// ---- Tenants ----
const tenants = ref<TenantInfo[]>([])
const tenantsLoading = ref(false)
const tenantDialogVisible = ref(false)
const editingTenant = ref<TenantInfo | null>(null)
const tenantForm = ref({ name: '', slug: '', enabled: true })

async function loadTenants() {
  tenantsLoading.value = true
  try { tenants.value = await adminListTenants() } finally { tenantsLoading.value = false }
}

function openCreateTenant() {
  editingTenant.value = null
  tenantForm.value = { name: '', slug: '', enabled: true }
  tenantDialogVisible.value = true
}

function openEditTenant(t: TenantInfo) {
  editingTenant.value = t
  tenantForm.value = { name: t.name, slug: t.slug, enabled: t.enabled }
  tenantDialogVisible.value = true
}

async function saveTenant() {
  try {
    if (editingTenant.value) {
      await adminUpdateTenant(editingTenant.value.id, tenantForm.value)
    } else {
      await adminCreateTenant({ name: tenantForm.value.name, slug: tenantForm.value.slug })
    }
    tenantDialogVisible.value = false
    await loadTenants()
    toast.add({ severity: 'success', summary: 'Saved', life: 2000 })
  } catch (e: any) {
    toast.add({ severity: 'error', summary: 'Error', detail: e.response?.data ?? e.message, life: 4000 })
  }
}

function confirmDeleteTenant(t: TenantInfo) {
  if (t.mock_count > 0) {
    toast.add({
      severity: 'warn',
      summary: 'Cannot delete tenant',
      detail: `"${t.name}" has ${t.mock_count} mock${t.mock_count > 1 ? 's' : ''}. Please delete all mocks first.`,
      life: 5000,
    })
    return
  }
  confirm.require({
    message: `Delete tenant "${t.name}"?`,
    header: 'Confirm',
    icon: 'pi pi-question-circle',
    accept: async () => {
      try {
        await adminDeleteTenant(t.id)
        await loadTenants()
        toast.add({ severity: 'success', summary: 'Deleted', life: 2000 })
      } catch (e: any) {
        toast.add({ severity: 'error', summary: 'Error', detail: e.response?.data ?? e.message, life: 4000 })
      }
    },
  })
}

// ---- Users ----
const users = ref<UserInfo[]>([])
const usersLoading = ref(false)
const userDialogVisible = ref(false)
const editingUser = ref<UserInfo | null>(null)
const userForm = ref({ username: '', display_name: '', password: '', is_admin: false, tenant_ids: [] as number[] })

async function loadUsers() {
  usersLoading.value = true
  try { users.value = await adminListUsers() } finally { usersLoading.value = false }
}

function openCreateUser() {
  editingUser.value = null
  userForm.value = { username: '', display_name: '', password: '', is_admin: false, tenant_ids: [] }
  userDialogVisible.value = true
}

function openEditUser(u: UserInfo) {
  editingUser.value = u
  userForm.value = { username: u.username, display_name: u.display_name, password: '', is_admin: u.is_admin, tenant_ids: [] }
  userDialogVisible.value = true
}

async function saveUser() {
  try {
    if (editingUser.value) {
      await adminUpdateUser(editingUser.value.id, { display_name: userForm.value.display_name, is_admin: userForm.value.is_admin })
    } else {
      const created = await adminCreateUser({ username: userForm.value.username, display_name: userForm.value.display_name, password: userForm.value.password, is_admin: userForm.value.is_admin })
      await Promise.all(userForm.value.tenant_ids.map(tid => adminAssignTenant(created.id, tid)))
    }
    userDialogVisible.value = false
    await loadUsers()
    toast.add({ severity: 'success', summary: 'Saved', life: 2000 })
  } catch (e: any) {
    toast.add({ severity: 'error', summary: 'Error', detail: e.response?.data ?? e.message, life: 4000 })
  }
}

async function toggleUserEnabled(u: UserInfo) {
  try {
    if (u.enabled) await adminDisableUser(u.id)
    else await adminEnableUser(u.id)
    await loadUsers()
  } catch (e: any) {
    toast.add({ severity: 'error', summary: 'Error', detail: e.response?.data ?? e.message, life: 4000 })
  }
}

function confirmDeleteUser(u: UserInfo) {
  confirm.require({
    message: `Delete user "${u.username}"?`,
    header: 'Confirm',
    icon: 'pi pi-exclamation-triangle',
    accept: async () => {
      try {
        await adminDeleteUser(u.id)
        await loadUsers()
        toast.add({ severity: 'success', summary: 'Deleted', life: 2000 })
      } catch (e: any) {
        toast.add({ severity: 'error', summary: 'Error', detail: e.response?.data ?? e.message, life: 4000 })
      }
    },
  })
}

// ---- Reset Password ----
const resetPwdVisible = ref(false)
const resetPwdTarget = ref<UserInfo | null>(null)
const newPassword = ref('')

function openResetPwd(u: UserInfo) {
  resetPwdTarget.value = u
  newPassword.value = ''
  resetPwdVisible.value = true
}

async function doResetPassword() {
  if (!resetPwdTarget.value || !newPassword.value) return
  try {
    await adminResetPassword(resetPwdTarget.value.id, newPassword.value)
    resetPwdVisible.value = false
    toast.add({ severity: 'success', summary: 'Password reset', life: 2000 })
  } catch (e: any) {
    toast.add({ severity: 'error', summary: 'Error', detail: e.response?.data ?? e.message, life: 4000 })
  }
}

// ---- User Tenants ----
const userTenantsVisible = ref(false)
const userTenantsTarget = ref<UserInfo | null>(null)
const userTenantList = ref<UserTenant[]>([])
const assignTenantId = ref<number | null>(null)

const assignableTenants = computed(() => {
  const assigned = new Set(userTenantList.value.map(ut => ut.tenant_id))
  return tenants.value.filter(t => !assigned.has(t.id))
})

function tenantNameById(id: number) {
  return tenants.value.find(t => t.id === id)?.name ?? String(id)
}

async function openUserTenants(u: UserInfo) {
  userTenantsTarget.value = u
  assignTenantId.value = null
  userTenantList.value = await adminListUserTenants(u.id)
  userTenantsVisible.value = true
}

async function doAssignTenant() {
  if (!userTenantsTarget.value || !assignTenantId.value) return
  try {
    await adminAssignTenant(userTenantsTarget.value.id, assignTenantId.value)
    userTenantList.value = await adminListUserTenants(userTenantsTarget.value.id)
    assignTenantId.value = null
    toast.add({ severity: 'success', summary: 'Tenant assigned', life: 2000 })
  } catch (e: any) {
    toast.add({ severity: 'error', summary: 'Error', detail: e.response?.data ?? e.message, life: 4000 })
  }
}

async function removeUserTenant(tenantId: number) {
  if (!userTenantsTarget.value) return
  try {
    await adminRemoveTenant(userTenantsTarget.value.id, tenantId)
    userTenantList.value = await adminListUserTenants(userTenantsTarget.value.id)
  } catch (e: any) {
    toast.add({ severity: 'error', summary: 'Error', detail: e.response?.data ?? e.message, life: 4000 })
  }
}

onMounted(() => {
  loadTenants()
  loadUsers()
})
</script>
