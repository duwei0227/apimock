<template>
  <Dialog
    :visible="modelValue"
    @update:visible="$emit('update:modelValue', $event)"
    header="Test Mock"
    :style="{ width: '960px', maxWidth: '96vw' }"
    modal
    :draggable="false"
    contentStyle="padding: 0; overflow: hidden;"
  >
    <div v-if="mock" class="flex" style="min-height: 480px; max-height: 75vh;">

      <!-- ===== LEFT — Request ===== -->
      <div class="flex flex-col gap-4 p-5 overflow-y-auto" style="width: 50%; border-right: 1px solid var(--p-surface-200);">

        <!-- Read-only method + URL bar -->
        <div class="flex items-center gap-2">
          <Badge :value="mock.method" severity="info" class="shrink-0" />
          <InputText
            :value="testUrl"
            readonly
            class="flex-1 font-mono text-xs"
            style="background: var(--p-surface-100); color: var(--p-surface-500); cursor: default;"
          />
        </div>

        <!-- Query Params -->
        <div>
          <div class="flex items-center justify-between mb-2">
            <span class="text-xs font-semibold uppercase tracking-wide text-surface-500">Query Params</span>
            <Button label="Add" icon="pi pi-plus" text size="small" @click="addQueryParam" />
          </div>
          <div v-if="queryParams.length === 0" class="text-xs text-surface-400 italic">No params — click Add</div>
          <div
            v-for="(row, i) in queryParams"
            :key="i"
            class="flex gap-2 mb-1 items-center"
          >
            <InputText v-model="row.key" placeholder="key" class="flex-1 text-xs" size="small" />
            <InputText v-model="row.value" placeholder="value" class="flex-1 text-xs" size="small" />
            <Button icon="pi pi-times" text rounded severity="danger" size="small" class="shrink-0" @click="queryParams.splice(i, 1)" />
          </div>
        </div>

        <!-- Request Headers -->
        <div>
          <div class="flex items-center justify-between mb-2">
            <span class="text-xs font-semibold uppercase tracking-wide text-surface-500">Request Headers</span>
            <Button label="Add" icon="pi pi-plus" text size="small" @click="addHeader" />
          </div>
          <div v-if="reqHeaders.length === 0" class="text-xs text-surface-400 italic">No headers — click Add</div>
          <div
            v-for="(row, i) in reqHeaders"
            :key="i"
            class="flex gap-2 mb-1 items-center"
          >
            <InputText v-model="row.key" placeholder="header name" class="flex-1 text-xs" size="small" />
            <InputText v-model="row.value" placeholder="value" class="flex-1 text-xs" size="small" />
            <Button icon="pi pi-times" text rounded severity="danger" size="small" class="shrink-0" @click="reqHeaders.splice(i, 1)" />
          </div>
        </div>

        <!-- Request Body (POST / PUT / PATCH / DELETE only) -->
        <div v-if="showBody">
          <span class="text-xs font-semibold uppercase tracking-wide text-surface-500 block mb-2">Request Body</span>
          <Textarea
            v-model="reqBody"
            rows="6"
            class="w-full font-mono text-xs"
            placeholder='{"key": "value"}'
            autoResize
          />
        </div>

        <!-- Send button -->
        <div class="mt-auto pt-2">
          <Button
            label="Send"
            icon="pi pi-send"
            class="w-full"
            :loading="loading"
            @click="send"
          />
        </div>
      </div>

      <!-- ===== RIGHT — Response ===== -->
      <div class="flex flex-col p-5 overflow-y-auto" style="width: 50%;">

        <!-- Placeholder — nothing sent yet -->
        <div
          v-if="!result && !error"
          class="flex flex-col items-center justify-center flex-1 gap-3 text-surface-300"
        >
          <i class="pi pi-send text-4xl" />
          <span class="text-sm">Send a request to see the response</span>
        </div>

        <!-- Error state -->
        <div v-if="error" class="flex items-start gap-2 text-sm text-red-500">
          <i class="pi pi-exclamation-circle mt-0.5 shrink-0" />
          <span>{{ error }}</span>
        </div>

        <!-- Success state -->
        <div v-if="result" class="flex flex-col gap-4">

          <!-- Status + timing -->
          <div class="flex items-center gap-3">
            <Badge
              :value="String(result.status)"
              :severity="result.status < 400 ? 'success' : 'danger'"
            />
            <span class="text-xs text-surface-400">{{ result.elapsed_ms }} ms</span>
          </div>

          <!-- Response Headers -->
          <div>
            <p class="text-xs font-semibold uppercase tracking-wide text-surface-500 mb-1">Response Headers</p>
            <div v-if="Object.keys(result.headers).length === 0" class="text-xs text-surface-400 italic">None</div>
            <div
              v-for="(v, k) in result.headers"
              :key="k"
              class="text-xs font-mono leading-relaxed"
            >
              <span class="text-primary-500">{{ k }}</span>
              <span class="text-surface-400">: </span>
              <span class="text-surface-600 dark:text-surface-300">{{ v }}</span>
            </div>
          </div>

          <!-- Response Body -->
          <div class="flex flex-col flex-1 min-h-0">
            <p class="text-xs font-semibold uppercase tracking-wide text-surface-500 mb-1">Response Body</p>
            <pre
              class="flex-1 rounded p-3 text-xs font-mono overflow-auto leading-relaxed"
              style="background: var(--p-surface-100); color: var(--p-surface-700); max-height: 340px;"
            >{{ prettyBody }}</pre>
          </div>
        </div>

      </div>
    </div>
  </Dialog>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import Dialog from 'primevue/dialog'
import Badge from 'primevue/badge'
import Button from 'primevue/button'
import InputText from 'primevue/inputtext'
import Textarea from 'primevue/textarea'
import { MocksApi } from '../../api/client'
import type { MockApi, TestMockResult } from '../../api/client'

interface KV { key: string; value: string }

const props = defineProps<{
  modelValue: boolean
  mock: MockApi | undefined
  testUrl: string
}>()

defineEmits<{ (e: 'update:modelValue', v: boolean): void }>()

const queryParams = ref<KV[]>([])
const reqHeaders  = ref<KV[]>([])
const reqBody     = ref('')
const loading     = ref(false)
const result      = ref<TestMockResult | null>(null)
const error       = ref<string | null>(null)

const BODY_METHODS = ['POST', 'PUT', 'PATCH', 'DELETE']

const showBody = computed(() =>
  props.mock ? BODY_METHODS.includes(props.mock.method) : false
)

const prettyBody = computed(() => {
  if (!result.value) return ''
  try { return JSON.stringify(JSON.parse(result.value.body), null, 2) }
  catch { return result.value.body }
})

function addQueryParam() { queryParams.value.push({ key: '', value: '' }) }
function addHeader()     { reqHeaders.value.push({ key: '', value: '' }) }

function reset() {
  result.value  = null
  error.value   = null
  reqBody.value = ''
  reqHeaders.value = []
}

function initFromMock(mock: MockApi | undefined) {
  reset()
  queryParams.value = mock ? Object.keys(mock.request_params).map(k => ({ key: k, value: '' })) : []
}

watch(() => props.mock,        (m)    => initFromMock(m),         { immediate: true })
watch(() => props.modelValue,  (open) => { if (open) initFromMock(props.mock) })

async function send() {
  if (!props.mock) return
  loading.value = true
  result.value  = null
  error.value   = null
  try {
    const payload = {
      query_params: Object.fromEntries(
        queryParams.value.filter(r => r.key.trim()).map(r => [r.key.trim(), r.value])
      ),
      headers: Object.fromEntries(
        reqHeaders.value.filter(r => r.key.trim()).map(r => [r.key.trim(), r.value])
      ),
      body: showBody.value && reqBody.value ? reqBody.value : undefined,
    }
    const res = await MocksApi.test(props.mock.id, payload)
    result.value = res.data
  } catch (e: any) {
    error.value = e.response?.data ?? e.message
  } finally {
    loading.value = false
  }
}
</script>
