<template>
  <div class="flex flex-col gap-4 h-full">
    <div class="flex items-center justify-between">
      <h2 class="text-xl font-semibold">Mocks</h2>
      <div class="flex gap-2">
        <Select
          v-if="auth.isAdmin"
          v-model="selectedTenantId"
          :options="tenantOptions"
          optionLabel="label"
          optionValue="value"
          placeholder="All tenants"
          showClear
          class="w-44"
          @change="reload"
        />
        <Select
          v-model="selectedPortId"
          :options="portOptions"
          optionLabel="label"
          optionValue="value"
          placeholder="All ports"
          showClear
          class="w-48"
          @change="reload"
        />
        <Button label="New Mock" icon="pi pi-plus" @click="openCreate" />
      </div>
    </div>

    <div class="flex flex-1 rounded-xl shadow-sm overflow-hidden" style="height: calc(100vh - 160px)">
      <!-- Table panel -->
      <div class="flex-1 overflow-hidden min-w-0 relative">
        <div
          v-if="!mocksStore.loading && mocksStore.mocks.length === 0"
          class="absolute inset-0 flex flex-col items-center justify-center gap-3 text-surface-400 z-10 pointer-events-none"
          style="top: 48px"
        >
          <i class="pi pi-inbox text-4xl" />
          <span class="text-sm">No mocks found</span>
        </div>
        <DataTable
          :value="mocksStore.mocks"
          :loading="mocksStore.loading"
          v-model:selection="selected"
          selectionMode="single"
          dataKey="id"
          stripedRows
          scrollable
          scrollHeight="flex"
          class="h-full"
          @rowSelect="onRowSelect"
        >
          <Column style="min-width:260px">
            <template #header>
              <div class="flex items-center gap-1">
                <span>Address</span>
                <Button
                  icon="pi pi-question-circle"
                  size="small" text rounded
                  class="p-0 w-5 h-5 text-surface-400 hover:text-primary-500"
                  @click.stop="addrHelp.toggle($event)"
                />
              </div>
            </template>
            <template #body="{ data }">
              <div class="flex items-center gap-1 font-mono text-sm">
                <span
                  class="truncate"
                  style="max-width: 220px"
                  v-tooltip.top="{ value: fullUrl(data), showDelay: 300 }"
                >{{ fullUrl(data) }}</span>
                <Button
                  icon="pi pi-copy"
                  size="small" text rounded
                  class="p-0 w-5 h-5 shrink-0 text-surface-400 hover:text-primary-500"
                  @click.stop="copyFullUrl(data)"
                />
              </div>
            </template>
          </Column>
          <Column field="name" header="Name" style="min-width:120px" />
          <Column field="path" header="Path" class="font-mono text-sm" />
          <Column v-if="auth.isAdmin" header="Tenant" style="width:120px">
            <template #body="{ data }">
              <span class="text-xs font-mono text-surface-500">{{ tenantMap[data.tenant_id] ?? '—' }}</span>
            </template>
          </Column>
          <Column field="method" header="Method" style="width:90px">
            <template #body="{ data }">
              <Badge :value="data.method" severity="info" />
            </template>
          </Column>
          <Column header="Status" style="width:80px">
            <template #body="{ data }">
              <ToggleSwitch :modelValue="data.enabled" @update:modelValue="v => mocksStore.toggleEnabled(data.id, v)" />
            </template>
          </Column>
          <Column header="" style="width:136px">
            <template #body="{ data }">
              <div class="flex gap-1">
                <Button icon="pi pi-play" size="small" text rounded title="Test" class="text-green-500 hover:text-green-600" @click.stop="openTest(data)" />
                <Button icon="pi pi-copy" size="small" text rounded title="Duplicate" @click.stop="duplicateMock(data)" />
                <Button icon="pi pi-pencil" size="small" text rounded @click.stop="openEdit(data)" />
                <Button icon="pi pi-trash" severity="danger" size="small" text rounded @click.stop="confirmDelete(data)" />
              </div>
            </template>
          </Column>
        </DataTable>
      </div>

      <!-- Detail panel: only shown when a row is selected -->
      <Transition name="slide">
        <div
          v-if="showDetail && selected"
          class="w-96 shrink-0 bg-surface-50 dark:bg-surface-800 flex flex-col overflow-hidden"
          style="box-shadow: -2px 0 12px rgba(0,0,0,0.06)"
        >
          <div class="flex items-center justify-between px-4 py-2" style="border-bottom: 1px solid var(--p-surface-200)">
            <span class="text-sm font-medium text-surface-600 dark:text-surface-300">{{ selected.name }}</span>
            <Button icon="pi pi-times" size="small" text rounded severity="secondary" @click="closeDetail" />
          </div>
          <div class="flex-1 overflow-auto p-4">
            <MockDetail :mock="selected" />
          </div>
        </div>
      </Transition>
    </div>

    <MockDialog
      v-model="dialogVisible"
      :mock="editingMock"
      :ports="portsStore.ports"
      :tenants="allTenants"
      @save="onSave"
    />

    <MockTestDialog
      v-model="testDialogVisible"
      :mock="testingMock"
      :testUrl="testingMock ? fullUrl(testingMock) : ''"
    />

    <Popover ref="addrHelp">
      <div class="p-1 space-y-3" style="max-width: 340px">
        <p class="text-sm font-semibold text-surface-700 dark:text-surface-200">Address 组成规则</p>
        <code class="block text-xs font-mono bg-surface-100 dark:bg-surface-800 rounded px-3 py-2 leading-relaxed">
          {ip}:{port}/<span class="text-primary-500">{tenant}</span>/<span class="text-green-500">{path}</span>
        </code>
        <div class="space-y-2 text-xs text-surface-600 dark:text-surface-300">
          <div class="flex gap-2">
            <span class="font-mono text-surface-500 shrink-0">ip:port</span>
            <span>服务器 IP 与端口，由管理员在 Ports 页面创建</span>
          </div>
          <div class="flex gap-2">
            <span class="font-mono text-primary-500 shrink-0">tenant</span>
            <span>租户标识（slug），用于隔离不同租户的路由，确保同一端口下不同团队的 Mock 互不干扰</span>
          </div>
          <div class="flex gap-2">
            <span class="font-mono text-green-500 shrink-0">path</span>
            <span>你在 Mock 中定义的接口路径</span>
          </div>
        </div>
        <div class="text-xs text-surface-400 border-t border-surface-200 dark:border-surface-700 pt-2">
          示例：<code class="font-mono">10.0.0.1:8080/acme/api/users</code>
        </div>
      </div>
    </Popover>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import DataTable from 'primevue/datatable'
import Column from 'primevue/column'
import Badge from 'primevue/badge'
import Button from 'primevue/button'
import Select from 'primevue/select'
import ToggleSwitch from 'primevue/toggleswitch'
import Popover from 'primevue/popover'
import { useConfirm } from 'primevue/useconfirm'
import { useToast } from 'primevue/usetoast'
import { usePortsStore } from '../stores/ports'
import { useMocksStore } from '../stores/mocks'
import { useAuthStore } from '../stores/auth'
import MockDialog from '../components/mocks/MockDialog.vue'
import MockDetail from '../components/mocks/MockDetail.vue'
import MockTestDialog from '../components/mocks/MockTestDialog.vue'
import type { MockApi, TenantInfo } from '../api/client'
import { InfoApi, adminListTenants } from '../api/client'
import { copyText } from '../utils/clipboard'

const portsStore = usePortsStore()
const mocksStore = useMocksStore()
const auth = useAuthStore()
const confirm = useConfirm()
const toast = useToast()

const addrHelp = ref()
const serverIp = ref('')
const selected = ref<MockApi | null>(null)
const showDetail = ref(false)
const dialogVisible = ref(false)
const editingMock = ref<MockApi | undefined>()
const testDialogVisible = ref(false)
const testingMock = ref<MockApi | undefined>()
const selectedPortId = ref<number | null>(null)
const selectedTenantId = ref<number | null>(null)
const allTenants = ref<TenantInfo[]>([])

const tenantMap = computed<Record<number, string>>(() =>
  Object.fromEntries(allTenants.value.map(t => [t.id, t.name]))
)

const tenantSlugMap = computed<Record<number, string>>(() =>
  Object.fromEntries(allTenants.value.map(t => [t.id, t.slug]))
)

const tenantOptions = computed(() =>
  allTenants.value.map(t => ({ label: t.name, value: t.id }))
)

const portOptions = computed(() =>
  portsStore.ports.map(p => ({ label: p.label ? `${p.port} — ${p.label}` : `${p.port}`, value: p.id }))
)

const portMap = computed(() =>
  Object.fromEntries(portsStore.ports.map(p => [p.id, p.port]))
)

function fullUrl(mock: MockApi & { tenant_slug?: string }) {
  const port = portMap.value[mock.port_id] ?? mock.port_id
  const slug = mock.tenant_slug
    ?? tenantSlugMap.value[mock.tenant_id]
    ?? auth.currentTenant?.slug
    ?? ''
  const path = mock.path.replace(/^\//, '')
  // "default" tenant is accessed without a slug prefix: ip:port/path
  const effectiveSlug = slug === 'default' ? '' : slug
  return effectiveSlug ? `${serverIp.value}:${port}/${effectiveSlug}/${path}` : `${serverIp.value}:${port}/${path}`
}

async function reload() {
  await mocksStore.fetchMocks(
    selectedPortId.value ?? undefined,
    selectedTenantId.value ?? undefined,
  )
}

onMounted(async () => {
  const promises: Promise<unknown>[] = [portsStore.fetchPorts(), InfoApi.get()]
  if (auth.isAdmin) promises.push(adminListTenants())
  const results = await Promise.allSettled(promises)
  const infoResult = results[1]
  if (infoResult.status === 'fulfilled') serverIp.value = (infoResult.value as { data: { ip: string } }).data.ip
  if (auth.isAdmin && results[2]?.status === 'fulfilled')
    allTenants.value = (results[2] as PromiseFulfilledResult<TenantInfo[]>).value
  await mocksStore.fetchMocks()
})

function onRowSelect(e: { data: MockApi }) {
  selected.value = e.data
  showDetail.value = true
}

function closeDetail() {
  showDetail.value = false
}

function openCreate() {
  editingMock.value = undefined
  dialogVisible.value = true
}

function openEdit(mock: MockApi) {
  editingMock.value = mock
  dialogVisible.value = true
}

function openTest(mock: MockApi) {
  testingMock.value = mock
  testDialogVisible.value = true
}

function confirmDelete(mock: MockApi) {
  confirm.require({
    message: `Delete mock "${mock.name}"?`,
    header: 'Confirm',
    icon: 'pi pi-exclamation-triangle',
    accept: async () => {
      await mocksStore.deleteMock(mock.id)
      if (selected.value?.id === mock.id) {
        selected.value = null
        showDetail.value = false
      }
      toast.add({ severity: 'success', summary: 'Deleted', life: 2000 })
    },
  })
}

function duplicateMock(mock: MockApi) {
  const usedPaths = new Set(
    mocksStore.mocks
      .filter(m => m.port_id === mock.port_id && m.method === mock.method)
      .map(m => m.path)
  )
  let path = `${mock.path}-copy`
  let n = 2
  while (usedPaths.has(path)) path = `${mock.path}-copy${n++}`

  editingMock.value = {
    ...mock,
    id: undefined as unknown as number,
    name: `${mock.name} (copy)`,
    path,
    enabled: false,
  }
  dialogVisible.value = true
}

function copyFullUrl(mock: MockApi & { tenant_slug?: string }) {
  const url = fullUrl(mock)
  copyText(url)
  toast.add({ severity: 'success', summary: 'Copied', detail: url, life: 2000 })
}

async function onSave(form: Partial<MockApi>) {
  try {
    if (editingMock.value?.id) {
      await mocksStore.updateMock(editingMock.value.id, form)
    } else {
      await mocksStore.createMock(form)
    }
    toast.add({ severity: 'success', summary: 'Saved', life: 2000 })
  } catch (e: any) {
    toast.add({ severity: 'error', summary: 'Error', detail: e.response?.data ?? e.message, life: 4000 })
  }
}
</script>

<style scoped>
.slide-enter-active,
.slide-leave-active {
  transition: width 0.22s ease, opacity 0.22s ease;
  overflow: hidden;
}
.slide-enter-from,
.slide-leave-to {
  width: 0 !important;
  opacity: 0;
}
.slide-enter-to,
.slide-leave-from {
  width: 24rem;
  opacity: 1;
}
</style>
