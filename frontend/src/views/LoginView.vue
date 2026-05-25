<template>
  <div class="terminal-bg">
    <div class="scanlines" />
    <div class="terminal-window">
      <div class="term-header">
        <div class="term-dots">
          <span class="dot red" />
          <span class="dot yellow" />
          <span class="dot green" />
        </div>
        <span class="term-title">apimock — auth@localhost</span>
      </div>

      <div class="term-body">
        <pre class="ascii-logo">
 █████╗ ██████╗ ██╗███╗   ███╗ ██████╗  ██████╗██╗  ██╗
██╔══██╗██╔══██╗██║████╗ ████║██╔═══██╗██╔════╝██║ ██╔╝
███████║██████╔╝██║██╔████╔██║██║   ██║██║     █████╔╝
██╔══██║██╔═══╝ ██║██║╚██╔╝██║██║   ██║██║     ██╔═██╗
██║  ██║██║     ██║██║ ╚═╝ ██║╚██████╔╝╚██████╗██║  ██╗
╚═╝  ╚═╝╚═╝     ╚═╝╚═╝     ╚═╝ ╚═════╝  ╚═════╝╚═╝  ╚═╝
        </pre>

        <div class="prompt-line">
          <span class="ps1"><span class="ps1-user">admin</span><span class="ps1-at">@</span><span class="ps1-host">mock</span><span class="ps1-path">:~</span><span class="ps1-dollar">$</span></span>
          <span class="ps1-cmd"> apimock login<span class="cursor-blink">▋</span></span>
        </div>

        <div class="fields">
          <div class="field-row">
            <label class="field-label">username</label>
            <div class="field-input-wrap" :class="{ focused: focusedField === 'user' }">
              <span class="field-arrow">›</span>
              <input
                ref="usernameRef"
                v-model="username"
                type="text"
                autocomplete="username"
                spellcheck="false"
                @focus="focusedField = 'user'"
                @blur="focusedField = ''"
                @keyup.enter="handleLogin"
              />
            </div>
          </div>

          <div class="field-row">
            <label class="field-label">password</label>
            <div class="field-input-wrap" :class="{ focused: focusedField === 'pass' }">
              <span class="field-arrow">›</span>
              <input
                v-model="password"
                :type="showPwd ? 'text' : 'password'"
                autocomplete="current-password"
                @focus="focusedField = 'pass'"
                @blur="focusedField = ''"
                @keyup.enter="handleLogin"
              />
              <button class="toggle-pwd" @click="showPwd = !showPwd" tabindex="-1">
                <i :class="showPwd ? 'pi pi-eye-slash' : 'pi pi-eye'" />
              </button>
            </div>
          </div>
        </div>

        <div v-if="error" class="error-bar">
          <span class="err-icon">✗</span> {{ error }}
        </div>

        <button class="exec-btn" :class="{ loading }" @click="handleLogin" :disabled="loading">
          <template v-if="!loading">
            <span class="exec-arrow">→</span> authenticate
          </template>
          <template v-else>
            <span class="spin">◐</span> authenticating…
          </template>
        </button>

        <div class="footer-hint">press <kbd>↵</kbd> to submit</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '../stores/auth'

const router = useRouter()
const auth = useAuthStore()

const username = ref('')
const password = ref('')
const error = ref('')
const loading = ref(false)
const showPwd = ref(false)
const focusedField = ref('')
const usernameRef = ref<HTMLInputElement | null>(null)

onMounted(() => usernameRef.value?.focus())

async function handleLogin() {
  if (!username.value || !password.value) return
  loading.value = true
  error.value = ''
  try {
    await auth.loginAction(username.value, password.value)
    router.push('/')
  } catch (e: any) {
    const status = e?.response?.status
    const msg = e?.response?.data
    if (status === 403 && msg) {
      error.value = msg
    } else {
      error.value = '用户名或密码错误'
    }
  } finally {
    loading.value = false
  }
}
</script>

<style scoped>
@import url('https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@300;400;500;700&display=swap');

* {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

.terminal-bg {
  position: fixed;
  inset: 0;
  background: #020c06;
  background-image:
    radial-gradient(ellipse 80% 60% at 50% -10%, rgba(0,255,80,0.08) 0%, transparent 60%),
    radial-gradient(ellipse 60% 40% at 80% 90%, rgba(0,200,60,0.05) 0%, transparent 50%);
  display: flex;
  align-items: center;
  justify-content: center;
  font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;
}

.scanlines {
  position: fixed;
  inset: 0;
  pointer-events: none;
  background: repeating-linear-gradient(
    to bottom,
    transparent 0px,
    transparent 3px,
    rgba(0,0,0,0.12) 3px,
    rgba(0,0,0,0.12) 4px
  );
  z-index: 10;
}

.terminal-window {
  width: min(680px, 96vw);
  background: #061008;
  border: 1px solid rgba(0,255,80,0.2);
  border-radius: 8px;
  box-shadow:
    0 0 0 1px rgba(0,255,80,0.05),
    0 0 60px rgba(0,255,80,0.06),
    0 32px 80px rgba(0,0,0,0.7),
    inset 0 1px 0 rgba(0,255,80,0.08);
  overflow: hidden;
  animation: slide-up 0.4s cubic-bezier(0.16, 1, 0.3, 1) both;
}

@keyframes slide-up {
  from { opacity: 0; transform: translateY(24px) scale(0.98); }
  to   { opacity: 1; transform: translateY(0) scale(1); }
}

/* — Header chrome — */
.term-header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 11px 16px;
  background: rgba(0,255,80,0.03);
  border-bottom: 1px solid rgba(0,255,80,0.1);
}

.term-dots {
  display: flex;
  gap: 6px;
}

.dot {
  width: 12px;
  height: 12px;
  border-radius: 50%;
}
.dot.red    { background: #ff5f57; }
.dot.yellow { background: #ffbd2e; }
.dot.green  { background: #28c840; }

.term-title {
  flex: 1;
  text-align: center;
  font-size: 11px;
  color: rgba(0,255,80,0.35);
  letter-spacing: 0.05em;
  margin-right: 30px;
}

/* — Body — */
.term-body {
  padding: 28px 32px 32px;
  display: flex;
  flex-direction: column;
  gap: 0;
}

/* — ASCII Logo — */
.ascii-logo {
  font-size: clamp(4px, 1.1vw, 8.5px);
  line-height: 1.2;
  color: rgba(0,255,80,0.55);
  white-space: pre;
  text-align: center;
  margin-bottom: 24px;
  letter-spacing: 0;
  text-shadow: 0 0 12px rgba(0,255,80,0.4);
  overflow: hidden;
}

/* — Prompt line — */
.prompt-line {
  display: flex;
  align-items: center;
  gap: 0;
  margin-bottom: 22px;
  font-size: 13px;
}

.ps1-user   { color: #28c840; }
.ps1-at     { color: rgba(0,255,80,0.4); }
.ps1-host   { color: #00aaff; }
.ps1-path   { color: rgba(0,255,80,0.4); }
.ps1-dollar { color: #ffd700; }
.ps1-cmd    { color: rgba(0,255,80,0.6); }

.cursor-blink {
  animation: blink 1s step-end infinite;
  color: #00ff50;
}
@keyframes blink {
  50% { opacity: 0; }
}

/* — Fields — */
.fields {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-bottom: 20px;
}

.field-row {
  display: flex;
  align-items: center;
  gap: 0;
}

.field-label {
  width: 90px;
  font-size: 12px;
  color: rgba(0,255,80,0.45);
  flex-shrink: 0;
}

.field-input-wrap {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 8px;
  border: 1px solid rgba(0,255,80,0.12);
  border-radius: 4px;
  padding: 8px 12px;
  background: rgba(0,255,80,0.03);
  transition: border-color 0.15s, box-shadow 0.15s, background 0.15s;
}

.field-input-wrap.focused {
  border-color: rgba(0,255,80,0.5);
  background: rgba(0,255,80,0.05);
  box-shadow: 0 0 0 3px rgba(0,255,80,0.08), 0 0 16px rgba(0,255,80,0.06);
}

.field-arrow {
  color: #00ff50;
  font-size: 16px;
  line-height: 1;
  flex-shrink: 0;
  text-shadow: 0 0 8px rgba(0,255,80,0.8);
}

.field-input-wrap input {
  flex: 1;
  background: transparent;
  border: none;
  outline: none;
  color: #c0ffd0;
  font-family: inherit;
  font-size: 13px;
  caret-color: #00ff50;
  letter-spacing: 0.02em;
}

.field-input-wrap input::placeholder {
  color: rgba(0,255,80,0.2);
}

.toggle-pwd {
  background: transparent;
  border: none;
  cursor: pointer;
  color: rgba(0,255,80,0.3);
  font-size: 13px;
  padding: 0 2px;
  display: flex;
  align-items: center;
  transition: color 0.15s;
}
.toggle-pwd:hover { color: rgba(0,255,80,0.7); }

/* — Error — */
.error-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: #ff6b6b;
  background: rgba(255,60,60,0.08);
  border: 1px solid rgba(255,60,60,0.2);
  border-radius: 4px;
  padding: 8px 12px;
  margin-bottom: 16px;
}

.err-icon { font-size: 14px; }

/* — Execute button — */
.exec-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  width: 100%;
  padding: 11px 20px;
  background: rgba(0,255,80,0.08);
  border: 1px solid rgba(0,255,80,0.3);
  border-radius: 4px;
  color: #00ff50;
  font-family: inherit;
  font-size: 13px;
  font-weight: 500;
  letter-spacing: 0.08em;
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s, box-shadow 0.15s, transform 0.1s;
  text-shadow: 0 0 8px rgba(0,255,80,0.5);
  margin-bottom: 16px;
}

.exec-btn:hover:not(:disabled) {
  background: rgba(0,255,80,0.14);
  border-color: rgba(0,255,80,0.6);
  box-shadow: 0 0 24px rgba(0,255,80,0.12), inset 0 0 12px rgba(0,255,80,0.04);
  transform: translateY(-1px);
}

.exec-btn:active:not(:disabled) {
  transform: translateY(0);
}

.exec-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.exec-arrow {
  font-size: 16px;
}

.spin {
  display: inline-block;
  animation: spin-char 0.6s linear infinite;
  font-size: 14px;
}
@keyframes spin-char {
  0%   { content: '◐'; }
  25%  { opacity: 0.5; }
  100% { opacity: 1; transform: rotate(360deg); }
}

/* — Footer — */
.footer-hint {
  text-align: center;
  font-size: 11px;
  color: rgba(0,255,80,0.2);
  letter-spacing: 0.04em;
}

kbd {
  font-family: inherit;
  font-size: 10px;
  background: rgba(0,255,80,0.08);
  border: 1px solid rgba(0,255,80,0.15);
  border-radius: 3px;
  padding: 1px 5px;
  color: rgba(0,255,80,0.35);
}
</style>
