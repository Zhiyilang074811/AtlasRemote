import sys

connect_view = """<template>
  <div class="connect-view">
    <div class="logo-section">
      <div class="logo">
        <div class="logo-icon">⬡</div>
        <span class="logo-text">AtlasRemote</span>
      </div>
      <p class="tagline">Open Source · Unlimited · Cross-Platform</p>
    </div>

    <div class="connect-card">
      <h2>Remote Desktop</h2>

      <div class="input-group">
        <label>Host</label>
        <input v-model="host" type="text" placeholder="127.0.0.1" :disabled="connecting" />
      </div>

      <div class="input-group">
        <label>Port</label>
        <input v-model.number="port" type="number" placeholder="9090" :disabled="connecting" />
      </div>

      <div class="input-group" v-if="pairCode">
        <label>Pair Code</label>
        <div class="pair-display">
          <span class="pair-code">{{ pairCode }}</span>
          <button @click="copyPairCode" class="copy-btn">Copy</button>
        </div>
      </div>

      <div v-if="deviceId" class="device-info">
        Device: <code>{{ deviceId }}</code>
      </div>

      <button class="connect-btn" :class="{ connecting, error }" :disabled="connecting || !host" @click="connect">
        {{ connecting ? 'Connecting...' : 'Connect' }}
      </button>

      <div v-if="error" class="error-msg">{{ error }}</div>
    </div>

    <div class="quick-links">
      <a href="#/remote/local">Quick Connect Local</a>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useAppStore } from '@/store/app'
import { useLogStore } from '@/store/log'
import { client } from '@/utils/client'

const router = useRouter()
const appStore = useAppStore()
const logStore = useLogStore()

const host = ref(appStore.host)
const port = ref(appStore.port)
const connecting = ref(false)
const pairCode = ref('')
const deviceId = ref('')
const error = ref('')

function copyPairCode() {
  navigator.clipboard.writeText(pairCode.value)
  logStore.add('info', 'Pair code copied')
}

async function connect() {
  connecting.value = true
  error.value = ''
  pairCode.value = ''
  deviceId.value = ''
  appStore.host = host.value
  appStore.port = port.value

  try {
    await client.connect(host.value, port.value)
    logStore.add('info', 'Connected to ' + host.value + ':' + port.value)
    appStore.connectionStatus = 'connected'
    router.push('/remote/local')
  } catch (e: any) {
    error.value = e.message || 'Connection failed'
    logStore.add('error', 'Connect failed: ' + e.message)
  } finally {
    connecting.value = false
  }
}
</script>

<style lang="scss" scoped>
.connect-view {
  display: flex; flex-direction: column; align-items: center; justify-content: center;
  height: 100vh; gap: 32px; padding: 20px;
}
.logo-section { text-align: center; }
.logo { display: flex; align-items: center; justify-content: center; gap: 12px; font-size: 28px; font-weight: 700; }
.logo-icon { font-size: 36px; color: #4f8ef7; }
.tagline { margin-top: 8px; color: #8888aa; font-size: 14px; }
.connect-card {
  background: #1a1a2e; border: 1px solid #2a2a45; border-radius: 12px; padding: 32px;
  width: 100%; max-width: 420px; h2 { margin-bottom: 24px; font-size: 20px; font-weight: 600; }
}
.input-group { margin-bottom: 16px; label { display: block; margin-bottom: 6px; font-size: 13px; color: #8888aa; } }
.input-group input {
  width: 100%; padding: 10px 14px; background: #12121a; border: 1px solid #2a2a45;
  border-radius: 6px; color: #e8e8f0; font-size: 15px; outline: none; transition: border-color 0.2s;
  &:focus { border-color: #4f8ef7; } &:disabled { opacity: 0.5; }
}
.pair-display { display: flex; align-items: center; gap: 8px; }
.pair-code { flex: 1; font-size: 22px; font-weight: 700; letter-spacing: 4px; color: #4f8ef7; text-align: center; padding: 8px; }
.copy-btn { padding: 6px 12px; background: #252540; border-radius: 4px; font-size: 13px; color: #8888aa; &:hover { background: #2a2a45; color: #e8e8f0; } }
.device-info { margin-top: 12px; font-size: 13px; color: #8888aa; code { color: #34d399; font-family: monospace; } }
.connect-btn {
  width: 100%; padding: 14px; margin-top: 8px; background: #4f8ef7; border-radius: 6px;
  font-size: 16px; font-weight: 600; color: white; transition: background 0.2s;
  &:hover:not(:disabled) { background: #6ba0ff; } &:disabled { opacity: 0.5; cursor: not-allowed; }
  &.connecting { background: #fbbf24; color: #0a0a0f; } &.error { background: #f87171; }
}
.error-msg { margin-top: 12px; color: #f87171; font-size: 13px; text-align: center; }
.quick-links a { color: #4f8ef7; font-size: 13px; text-decoration: none; &:hover { text-decoration: underline; } }
</style>
"""

with open(r'D:\Android\AtlasRemote\apps\web\src\views\ConnectView.vue', 'w', encoding='utf-8') as f:
    f.write(connect_view)
print('OK: ConnectView.vue written')
