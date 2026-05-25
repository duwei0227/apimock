<template>
  <Toolbar class="rounded-none px-5 py-2" style="border:none; border-bottom: 1px solid var(--p-surface-100); box-shadow: 0 1px 6px 0 rgba(0,0,0,.04)">
    <template #start>
      <div class="flex items-center gap-2">
        <i class="pi pi-angle-right text-surface-300" />
        <span class="text-sm font-semibold text-surface-700 dark:text-surface-200">{{ pageTitle }}</span>
      </div>
    </template>
    <template #end>
      <div class="flex items-center gap-2">
        <Select
          v-if="auth.tenants.length > 1"
          v-model="selectedTenantId"
          :options="auth.tenants"
          optionLabel="name"
          optionValue="id"
          class="text-sm h-8"
          style="min-width:140px"
          @change="onTenantChange"
        />
        <Button
          :icon="isDark ? 'pi pi-sun' : 'pi pi-moon'"
          text rounded size="small" severity="secondary"
          @click="toggleDark"
          v-tooltip.bottom="isDark ? 'Light mode' : 'Dark mode'"
        />
        <Chip :label="auth.user?.display_name ?? auth.user?.username ?? 'user'" icon="pi pi-user" class="text-xs" />
        <Button icon="pi pi-sign-out" text rounded size="small" severity="secondary" v-tooltip.bottom="'Logout'" @click="handleLogout" />
      </div>
    </template>
  </Toolbar>
</template>

<script setup lang="ts">
import { computed, ref, onMounted, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import Toolbar from 'primevue/toolbar'
import Button from 'primevue/button'
import Chip from 'primevue/chip'
import Select from 'primevue/select'
import { useAuthStore } from '../../stores/auth'

const route = useRoute()
const router = useRouter()
const auth = useAuthStore()

const pageTitle = computed(() => (route.meta.title as string | undefined) ?? 'Mock APIs')
const selectedTenantId = ref<number | null>(auth.currentTenant?.id ?? null)

watch(() => auth.currentTenant, (t) => {
  selectedTenantId.value = t?.id ?? null
})

const isDark = ref(false)

onMounted(() => {
  isDark.value = document.documentElement.classList.contains('dark')
})

function toggleDark() {
  isDark.value = !isDark.value
  document.documentElement.classList.toggle('dark', isDark.value)
}

async function onTenantChange() {
  if (selectedTenantId.value == null) return
  await auth.switchTenantAction(selectedTenantId.value)
  window.location.reload()
}

function handleLogout() {
  auth.logout()
  router.push('/login')
}
</script>
