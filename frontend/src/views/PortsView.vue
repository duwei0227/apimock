<template>
  <div class="space-y-4">
    <div class="flex items-center justify-between">
      <h2 class="text-xl font-semibold">Ports</h2>
      <Button v-if="auth.isAdmin" label="New Port" icon="pi pi-plus" @click="openCreate" />
    </div>

    <Message v-if="!auth.isAdmin" severity="info" :closable="false" icon="pi pi-info-circle">
      Ports are managed by an administrator. Contact your admin to add or modify ports.
    </Message>

    <DataTable
      :value="portsStore.ports"
      :loading="portsStore.loading"
      stripedRows
      class="shadow-sm rounded-xl overflow-hidden"
    >
      <Column field="port" header="Port" class="font-mono" />
      <Column field="label" header="Label" />
      <Column header="Status">
        <template #body="{ data }">
          <Badge
            :value="portsStore.isRunning(data.id) ? 'Running' : 'Stopped'"
            :severity="portsStore.isRunning(data.id) ? 'success' : 'secondary'"
          />
        </template>
      </Column>
      <Column v-if="auth.isAdmin" header="Actions" style="width: 280px">
        <template #body="{ data }">
          <div class="flex gap-1">
            <Button
              v-tooltip.top="portsStore.isRunning(data.id) ? 'Stop port' : 'Start port'"
              :icon="portsStore.isRunning(data.id) ? 'pi pi-stop-circle' : 'pi pi-play-circle'"
              :label="portsStore.isRunning(data.id) ? 'Stop' : 'Start'"
              :severity="portsStore.isRunning(data.id) ? 'warning' : 'success'"
              size="small" text
              @click="toggleRunning(data)"
            />
            <Button
              v-tooltip.top="'View bindings'"
              icon="pi pi-sitemap"
              label="Bindings"
              size="small" text
              @click="openBindings(data)"
            />
            <Button
              v-tooltip.top="'Edit port'"
              icon="pi pi-pencil"
              label="Edit"
              size="small" text
              @click="openEdit(data)"
            />
            <Button
              v-tooltip.top="'Delete port'"
              icon="pi pi-trash"
              label="Delete"
              severity="danger"
              size="small" text
              @click="confirmDelete(data)"
            />
          </div>
        </template>
      </Column>
    </DataTable>

    <PortDialog
      v-model="dialogVisible"
      :port="editingPort"
      @saved="onSaved"
    />

    <!-- Bindings Dialog -->
    <Dialog
      v-model:visible="bindingsVisible"
      :header="`Port ${bindingsPort?.port} — Mock Bindings`"
      :modal="true"
      :style="{ width: '44rem' }"
      :dismissableMask="true"
    >
      <div v-if="bindingsLoading" class="flex justify-center py-8">
        <i class="pi pi-spin pi-spinner text-2xl text-surface-400" />
      </div>
      <div v-else-if="bindingGroups.length === 0" class="text-center text-surface-400 text-sm py-8">
        该端口下暂无绑定的 Mock
      </div>
      <div v-else class="space-y-4">
        <div v-for="group in bindingGroups" :key="group.tenantId" class="rounded-lg border border-surface-200 dark:border-surface-700 overflow-hidden">
          <!-- Tenant header -->
          <div class="flex items-center gap-2 px-4 py-2 bg-surface-50 dark:bg-surface-800 border-b border-surface-200 dark:border-surface-700">
            <i class="pi pi-tag text-primary-500 text-xs" />
            <span class="text-sm font-semibold text-surface-700 dark:text-surface-200">{{ group.tenantName }}</span>
            <Badge :value="String(group.mocks.length)" severity="secondary" class="ml-auto" />
          </div>
          <!-- Mocks list -->
          <div class="divide-y divide-surface-100 dark:divide-surface-800">
            <div
              v-for="mock in group.mocks"
              :key="mock.id"
              class="flex items-center gap-3 px-4 py-2.5 text-sm"
            >
              <Badge :value="mock.method" severity="info" class="shrink-0" />
              <span class="font-mono text-surface-700 dark:text-surface-200 flex-1 truncate">{{ mock.path }}</span>
              <span class="text-surface-400 truncate max-w-[140px]">{{ mock.name }}</span>
              <Badge
                :value="mock.enabled ? 'Enabled' : 'Disabled'"
                :severity="mock.enabled ? 'success' : 'secondary'"
                class="shrink-0"
              />
            </div>
          </div>
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
import Message from 'primevue/message'
import Dialog from 'primevue/dialog'
import { useConfirm } from 'primevue/useconfirm'
import { useToast } from 'primevue/usetoast'
import { usePortsStore } from '../stores/ports'
import { useAuthStore } from '../stores/auth'
import PortDialog from '../components/ports/PortDialog.vue'
import { MocksApi, adminListTenants } from '../api/client'
import type { PortConfig, MockApi, TenantInfo } from '../api/client'

const auth = useAuthStore()
const portsStore = usePortsStore()
const confirm = useConfirm()
const toast = useToast()

const dialogVisible = ref(false)
const editingPort = ref<PortConfig | undefined>()

onMounted(() => portsStore.fetchPorts())

function openCreate() {
  editingPort.value = undefined
  dialogVisible.value = true
}

function openEdit(port: PortConfig) {
  editingPort.value = port
  dialogVisible.value = true
}

async function toggleRunning(port: PortConfig) {
  try {
    if (portsStore.isRunning(port.id)) {
      await portsStore.stopPort(port.id)
    } else {
      await portsStore.startPort(port.id)
    }
  } catch (e: any) {
    toast.add({ severity: 'error', summary: 'Error', detail: e.response?.data ?? e.message, life: 3000 })
  }
}

function confirmDelete(port: PortConfig) {
  confirm.require({
    message: `Delete port ${port.port}?`,
    header: 'Confirm',
    icon: 'pi pi-exclamation-triangle',
    accept: async () => {
      try {
        await portsStore.deletePort(port.id)
        toast.add({ severity: 'success', summary: 'Deleted', life: 2000 })
      } catch (e: any) {
        const detail = e.response?.data ?? e.message
        toast.add({ severity: 'error', summary: `无法删除端口 ${port.port}`, detail, life: 5000 })
      }
    },
  })
}

async function onSaved() {
  await portsStore.fetchPorts()
}

// ---- Bindings ----
const bindingsVisible = ref(false)
const bindingsPort = ref<PortConfig | null>(null)
const bindingsLoading = ref(false)
const bindingMocks = ref<MockApi[]>([])
const tenantMap = ref<Record<number, string>>({})

interface BindingGroup {
  tenantId: number
  tenantName: string
  mocks: MockApi[]
}

const bindingGroups = computed<BindingGroup[]>(() => {
  const groups: Record<number, MockApi[]> = {}
  for (const mock of bindingMocks.value) {
    const tid = mock.tenant_id ?? 0
    if (!groups[tid]) groups[tid] = []
    groups[tid].push(mock)
  }
  return Object.entries(groups).map(([tid, mocks]) => ({
    tenantId: Number(tid),
    tenantName: tenantMap.value[Number(tid)] ?? `Tenant #${tid}`,
    mocks,
  }))
})

async function openBindings(port: PortConfig) {
  bindingsPort.value = port
  bindingsVisible.value = true
  bindingsLoading.value = true
  try {
    const [mocksRes, tenants] = await Promise.all([
      MocksApi.list(port.id),
      adminListTenants(),
    ])
    bindingMocks.value = mocksRes.data
    tenantMap.value = Object.fromEntries((tenants as TenantInfo[]).map(t => [t.id, t.name]))
  } catch (e: any) {
    toast.add({ severity: 'error', summary: 'Error', detail: e.response?.data ?? e.message, life: 4000 })
  } finally {
    bindingsLoading.value = false
  }
}
</script>
