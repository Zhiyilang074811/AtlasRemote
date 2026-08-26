<template>
  <div class="connect-view">
    <div class="logo-section">
      <div class="logo"><div class="logo-icon">⬡</div><span class="logo-text">AtlasRemote</span></div>
      <p class="tagline">Open Source · Unlimited · Cross-Platform</p>
    </div>
    <div class="connect-card">
      <h2>Remote Desktop</h2>
      <div class="input-group">
        <label>Pair Code<input id="pair-code-input" v-model="pairCode" type="text" placeholder="Enter 6-digit code" :disabled="connecting" maxlength="6" pattern="[0-9]*" inputmode="numeric" /></label>
      </div>
      <div class="relay-info">
        <span>Relay: <code>127.0.0.1:8080</code></span>
      </div>
      <div v-if="pairInfo.deviceId" class="device-info">
        Device: <code>{{ pairInfo.deviceId }}</code>
      </div>
      <div v-if="pairInfo.pairCode" class="pair-display">
        <span class="pair-code">{{ pairInfo.pairCode }}</span>
        <button @click="copyPairCode" class="copy-btn">Copy</button>
      </div>
      <button class="connect-btn" :class="{ connecting, error }" :disabled="connecting || !pairCode" @click="connect">
        {{ connecting ? "Connecting..." : "Connect" }}
      </button>
      <div v-if="error" class="error-msg">{{ error }}</div>
    </div>
    <div class="quick-links">
      <a href="#/remote/local" v-if="pairInfo.deviceId">Quick Connect →</a>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from "vue"
import { useRouter } from "vue-router"
import { useAppStore } from "@/store/app"
import { useLogStore } from "@/store/log"
import { client } from "@/utils/client"

const router = useRouter()
const appStore = useAppStore()
const logStore = useLogStore()
const pairCode = ref("")
const connecting = ref(false)
const error = ref("")
const pairInfo = reactive({ deviceId: "", pairCode: "" })

function copyPairCode() {
  navigator.clipboard.writeText(pairInfo.pairCode)
  logStore.add("info", "Pair code copied")
}

async function connect() {
  connecting.value = true
  error.value = ""
  pairInfo.deviceId = ""
  pairInfo.pairCode = ""
  appStore.connectionStatus = "connecting"

  client.onPairFail((reason) => {
    error.value = reason || "Invalid pair code"
    connecting.value = false
    logStore.add("error", error.value)
  })

  client.onPair((deviceId, code) => {
    pairInfo.deviceId = deviceId
    pairInfo.pairCode = code
    logStore.add("info", `Paired: ${deviceId}`)
    setTimeout(() => router.push(`/remote/${deviceId}`), 500)
  })

  try {
    await client.connect("local", pairCode.value)
    logStore.add("info", "Connected to relay, waiting for pair response...")
  } catch (e: any) {
    error.value = e.message || "Connection failed"
    logStore.add("error", e.message)
    appStore.connectionStatus = "error"
  } finally {
    connecting.value = false
  }
}
</script>

<style lang="scss" scoped>
.connect-view {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100vh;
  gap: 32px;
  padding: 20px;
}
.logo {
  display: flex;
  align-items: center;
  gap: 12px;
  font-size: 28px;
  font-weight: 700;
}
.logo-icon { font-size: 36px; color: #4f8ef7; }
.tagline { color: #8888aa; font-size: 14px; }
.connect-card {
  background: #1a1a2e;
  border: 1px solid #2a2a45;
  border-radius: 12px;
  padding: 32px;
  width: 100%;
  max-width: 420px;
  h2 { margin-bottom: 24px; font-size: 20px; }
}
.input-group { margin-bottom: 16px; label { display: block; margin-bottom: 6px; font-size: 13px; color: #8888aa; } }
.input-group input {
  width: 100%;
  padding: 10px 14px;
  background: #12121a;
  border: 1px solid #2a2a45;
  border-radius: 6px;
  color: #e8e8f0;
  font-size: 22px;
  letter-spacing: 8px;
  text-align: center;
  &:focus { border-color: #4f8ef7; outline: none; }
  &:disabled { opacity: 0.5; }
}
.relay-info {
  font-size: 12px;
  color: #8888aa;
  margin-bottom: 16px;
  code { color: #4f8ef7; }
}
.device-info {
  font-size: 13px;
  color: #8888aa;
  margin-bottom: 8px;
  code { color: #34d399; }
}
.pair-display {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  margin-bottom: 16px;
}
.pair-code {
  font-size: 28px;
  font-weight: 700;
  letter-spacing: 6px;
  color: #4f8ef7;
}
.copy-btn {
  padding: 4px 12px;
  background: rgba(79, 142, 247, 0.2);
  border: 1px solid #4f8ef7;
  border-radius: 4px;
  color: #4f8ef7;
  font-size: 12px;
  cursor: pointer;
  &:hover { background: rgba(79, 142, 247, 0.3); }
}
.connect-btn {
  width: 100%;
  padding: 14px;
  background: #4f8ef7;
  border-radius: 6px;
  font-size: 16px;
  font-weight: 600;
  color: white;
  cursor: pointer;
  &:hover:not(:disabled) { background: #6ba0ff; }
  &:disabled { opacity: 0.5; cursor: not-allowed; }
  &.connecting { background: #fbbf24; color: #0a0a0f; }
  &.error { background: #f87171; }
}
.error-msg { color: #f87171; font-size: 13px; text-align: center; margin-top: 12px; }
.quick-links a { color: #4f8ef7; font-size: 13px; text-decoration: none; &:hover { text-decoration: underline; } }
</style>


