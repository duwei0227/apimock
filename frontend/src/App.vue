<template>
  <template v-if="isLoginPage">
    <router-view />
    <Toast position="bottom-right" />
  </template>
  <template v-else>
    <div class="flex h-screen bg-surface-100 dark:bg-surface-950">
      <AppSidebar />
      <div class="flex flex-col flex-1 overflow-hidden">
        <AppTopBar />
        <main class="flex-1 overflow-auto p-5">
          <router-view />
        </main>
      </div>
    </div>
    <Toast position="bottom-right" />
    <ConfirmDialog />
  </template>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, computed } from 'vue'
import { useRoute } from 'vue-router'
import Toast from 'primevue/toast'
import ConfirmDialog from 'primevue/confirmdialog'
import AppSidebar from './components/layout/AppSidebar.vue'
import AppTopBar from './components/layout/AppTopBar.vue'
import { useLogsStore } from './stores/logs'
import { useAuthStore } from './stores/auth'

const route = useRoute()
const isLoginPage = computed(() => route.path === '/login')

const auth = useAuthStore()
const logsStore = useLogsStore()

onMounted(async () => {
  if (auth.isLoggedIn) {
    await auth.fetchMe()
  }
  logsStore.connectLive()
})
onUnmounted(() => logsStore.disconnectLive())
</script>
