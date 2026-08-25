import os

connect_view = """<template>
  <div class="connect-view">
    <div class="logo-section">
      <div class="logo"><div class="logo-icon">⬡</div><span class="logo-text">AtlasRemote</span></div>
      <p class="tagline">Open Source · Unlimited · Cross-Platform</p>
    </div>
    <div class="connect-card">
      <h2>Remote Desktop</h2>
      <div class="input-group"><label>Host</label><input v-model="host" type="text" placeholder="127.0.0.1" :disabled="connecting" /></div>
      <div class="input-group"><label>Port</label><input v-model.number="port" type="number" placeholder="9090" :disabled="connecting" /></div>
      <div class="input-group" v-if="pairCode"><label>Pair Code</label><div class="pair-display"><span class="pair-code">{{ pairCode }}</span><button @click="copyPairCode" class="copy-btn">Copy</button></div></div>
      <div v-if="deviceId" class="device-info">Device: <code>{{ deviceId }}</code></div>
      <button class="connect-btn" :class="{ connecting, error }" :disabled="connecting || !host" @click="connect">{{ connecting ? 'Connecting...' : 'Connect' }}</button>
      <div v-if="error" class="error-msg">{{ error }}</div>
    </div>
    <div class="quick-links"><a href="#/remote/local">Quick Connect Local</a></div>
  </div>
</template>
<script setup lang="ts">
import { ref } from 'vue'
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
function copyPairCode() { navigator.clipboard.writeText(pairCode.value); logStore.add('info', 'Pair code copied') }
async function connect() {
  connecting.value = true; error.value = ''; pairCode.value = ''; deviceId.value = ''
  appStore.host = host.value; appStore.port = port.value
  try { await client.connect(host.value, port.value); logStore.add('info', 'Connected'); appStore.connectionStatus = 'connected'; router.push('/remote/local') }
  catch (e: any) { error.value = e.message || 'Connection failed'; logStore.add('error', e.message) }
  finally { connecting.value = false }
}
</script>
<style lang="scss" scoped>
.connect-view { display:flex; flex-direction:column; align-items:center; justify-content:center; height:100vh; gap:32px; padding:20px; }
.logo { display:flex; align-items:center; gap:12px; font-size:28px; font-weight:700; }
.logo-icon { font-size:36px; color:#4f8ef7; }
.tagline { color:#8888aa; font-size:14px; }
.connect-card { background:#1a1a2e; border:1px solid #2a2a45; border-radius:12px; padding:32px; width:100%; max-width:420px; h2 { margin-bottom:24px; font-size:20px; } }
.input-group { margin-bottom:16px; label { display:block; margin-bottom:6px; font-size:13px; color:#8888aa; } }
.input-group input { width:100%; padding:10px14px; background:#12121a; border:1px solid #2a2a45; border-radius:6px; color:#e8e8f0; font-size:15px; &:focus { border-color:#4f8ef7; } &:disabled { opacity:0.5; } }
.pair-code { font-size:22px; font-weight:700; letter-spacing:4px; color:#4f8ef7; text-align:center; }
.connect-btn { width:100%; padding:14px; background:#4f8ef7; border-radius:6px; font-size:16px; font-weight:600; color:white; &:hover:not(:disabled) { background:#6ba0ff; } &:disabled { opacity:0.5; } &.connecting { background:#fbbf24; color:#0a0a0f; } &.error { background:#f87171; } }
.error-msg { color:#f87171; font-size:13px; text-align:center; margin-top:12px; }
.quick-links a { color:#4f8ef7; font-size:13px; text-decoration:none; &:hover { text-decoration:underline; } }
</style>
"""

remote_view = """<template>
  <div class="remote-view" :class="{ fullscreen, pointerLock }">
    <div class="top-bar" v-show="!fullscreen || showControls">
      <div class="bar-left">
        <router-link to="/" class="back-btn">Back</router-link>
        <div class="connection-status" :class="statusColor"><span class="dot"></span><span>{{ statusText }}</span></div>
        <div v-if="appStore.deviceId" class="device-id">ID: {{ appStore.deviceId }}</div>
      </div>
      <div class="bar-right">
        <button @click="toggleFullscreen" class="bar-btn">{{ fullscreen ? 'Exit' : 'FS' }}</button>
        <button @click="toggleDebug" class="bar-btn">Debug</button>
        <button @click="sendCtrlAltDel" class="bar-btn danger">Ctrl+Alt+Del</button>
      </div>
    </div>
    <div class="video-area" ref="videoAreaRef">
      <canvas ref="canvasRef" class="canvas" />
      <div v-if="!connected" class="connecting-overlay"><div class="spinner"></div><p>Connecting...</p></div>
      <div v-if="showDebug" class="debug-panel">
        <div class="debug-item"><span class="label">Res</span><span class="value">{{ width }}x{{ height }}</span></div>
        <div class="debug-item"><span class="label">FPS</span><span class="value">{{ fps }}</span></div>
        <div class="debug-item"><span class="label">Latency</span><span class="value">{{ latency }}ms</span></div>
        <div class="debug-item mouse-pos"><span class="label">Mouse</span><span class="value">{{ mouseX }}, {{ mouseY }}</span></div>
      </div>
    </div>
    <div class="bottom-bar" v-show="showControls || !fullscreen">
      <div class="input-hints"><span>Mouse: Move</span><span>Keyboard: Capture</span><span>Right-click: Menu</span></div>
    </div>
  </div>
</template>
<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { useAppStore } from '@/store/app'
import { useLogStore } from '@/store/log'
import { client } from '@/utils/client'
defineProps<{ deviceId?: string }>()
const appStore = useAppStore()
const logStore = useLogStore()
const canvasRef = ref<HTMLCanvasElement>()
const videoAreaRef = ref<HTMLDivElement>()
const connected = ref(false)
const showDebug = ref(false)
const showControls = ref(true)
const fullscreen = ref(false)
const pointerLock = ref(false)
const width = ref(0)
const height = ref(0)
const latency = ref(0)
const fps = ref(0)
const bitrate = ref(0)
const mouseX = ref(0)
const mouseY = ref(0)
const statusColor = computed(() => ({ connected:'online', connecting:'connecting', error:'error' }[appStore.connectionStatus] || 'offline'))
const statusText = computed(() => ({ connected:'Connected', connecting:'Connecting...', error:'Failed' }[appStore.connectionStatus] || 'Disconnected'))
let ctx: CanvasRenderingContext2D | null = null
let animFrame: number
let frameCount = 0
let lastFpsTime = Date.now()
let lastFrameTime = 0
onMounted(async () => {
  if (canvasRef.value) ctx = canvasRef.value.getContext('2d')
  const host = appStore.host || '127.0.0.1'
  const port = appStore.port || 9090
  try {
    await client.connect(host, port)
    appStore.connectionStatus = 'connected'
    connected.value = true
    logStore.add('info', 'Remote desktop connected')
    client.onFrame = (blob: Blob) => handleVideoFrame(blob)
    animFrame = requestAnimationFrame(updateFps)
    videoAreaRef.value?.addEventListener('click', () => videoAreaRef.value?.requestPointerLock())
    document.addEventListener('pointerlockchange', () => { pointerLock.value = !!document.pointerLockElement })
    document.addEventListener('mousemove', onMouseMove)
    document.addEventListener('keydown', onKeyDown)
    document.addEventListener('keyup', onKeyUp)
    document.addEventListener('contextmenu', onContextMenu)
    document.addEventListener('wheel', onWheel, { passive: false })
  } catch (e) { appStore.connectionStatus = 'error'; logStore.add('error', 'Failed: ' + e) }
})
onUnmounted(() => { cancelAnimationFrame(animFrame); client.disconnect(); document.removeEventListener('mousemove', onMouseMove); document.removeEventListener('keydown', onKeyDown); document.removeEventListener('keyup', onKeyUp); document.removeEventListener('contextmenu', onContextMenu); document.removeEventListener('wheel', onWheel) })
function handleVideoFrame(blob: Blob) {
  if (!ctx || !canvasRef.value) return
  const img = new Image(); const url = URL.createObjectURL(blob)
  img.onload = () => {
    const area = videoAreaRef.value; if (!area) { URL.revokeObjectURL(url); return }
    const r = area.getBoundingClientRect()
    const scaleX = width.value / r.width || 1
    const scaleY = height.value / r.height || 1
    const imgAspect = img.width / img.height
    const areaAspect = r.width / r.height
    let dw = r.width, dh = r.height
    if (imgAspect > areaAspect) dh = r.width / imgAspect
    else dw = r.height * imgAspect
    const dx = (r.width - dw) / 2, dy = (r.height - dh) / 2
    canvasRef.value.width = dw; canvasRef.value.height = dh
    ctx.drawImage(img, dx, dy, dw, dh)
    width.value = img.width; height.value = img.height
    bitrate.value = blob.size * 8 / ((Date.now() - lastFrameTime) / 1000)
    lastFrameTime = Date.now(); URL.revokeObjectURL(url)
  }
  img.src = url
}
function updateFps() { frameCount++; const now = Date.now(); if (now - lastFpsTime >= 1000) { fps.value = frameCount; frameCount = 0; lastFpsTime = now } animFrame = requestAnimationFrame(updateFps) }
function onMouseMove(e: MouseEvent) { if (!pointerLock.value) return; const area = videoAreaRef.value; if (!area) return; const r = area.getBoundingClientRect(); const scaleX = width.value / r.width; const scaleY = height.value / r.height; mouseX.value = e.clientX - r.left; mouseY.value = e.clientY - r.top; const relX = (e.movementX / r.width) * scaleX; const relY = (e.movementY / r.height) * scaleY; if (relX !== 0 || relY !== 0) client.sendMouse(relX, relY) }
function onKeyDown(e: KeyboardEvent) { if (e.ctrlKey && e.altKey && e.key === 'Delete') { sendCtrlAltDel(); e.preventDefault(); return }; client.sendKey(e.code, true) }
function onKeyUp(e: KeyboardEvent) { client.sendKey(e.code, false) }
function onContextMenu(e: MouseEvent) { e.preventDefault(); client.sendClick('right', true); setTimeout(() => client.sendClick('right', false), 100) }
function onWheel(e: WheelEvent) { e.preventDefault(); client.sendWheel(e.deltaY > 0 ? -1 : 1) }
function sendCtrlAltDel() { client.sendKey('ControlLeft', true); client.sendKey('AltLeft', true); client.sendKey('Delete', true); setTimeout(() => { client.sendKey('ControlLeft', false); client.sendKey('AltLeft', false); client.sendKey('Delete', false) }, 100) }
function toggleFullscreen() { if (!document.fullscreenElement) { document.documentElement.requestFullscreen(); fullscreen.value = true } else { document.exitFullscreen(); fullscreen.value = false } }
function toggleDebug() { showDebug.value = !showDebug.value }
watch(fullscreen, (v) => { if (!v) showControls.value = true })
</script>
<style lang="scss" scoped>
.remote-view { position:relative; width:100vw; height:100vh; background:#000; overflow:hidden; &.pointerLock { cursor:none; } }
.top-bar { position:absolute; top:0; left:0; right:0; height:48px; display:flex; align-items:center; justify-content:space-between; padding:0 16px; background:linear-gradient(to bottom,rgba(0,0,0,0.8),transparent); z-index:100; opacity:0; transition:opacity 0.3s; }
.remote-view:hover & { opacity:1; }
.bar-left, .bar-right { display:flex; align-items:center; gap:16px; }
.back-btn { color:#8888aa; text-decoration:none; font-size:14px; &:hover { color:#e8e8f0; } }
.connection-status { display:flex; align-items:center; gap:6px; font-size:13px; .dot { width:8px; height:8px; border-radius:50%; &.online { background:#34d399; } &.connecting { background:#fbbf24; animation:blink 1s infinite; } &.error { background:#f87171; } &.offline { background:#666; } } }
@keyframes blink { 0%,100%{opacity:1} 50%{opacity:0.3} }
.device-id { font-size:12px; color:#34d399; font-family:monospace; }
.bar-btn { padding:6px 14px; background:rgba(255,255,255,0.1); border-radius:4px; font-size:13px; color:#e8e8f0; &:hover { background:rgba(255,255,255,0.2); } &.danger { color:#f87171; } &.danger:hover { background:rgba(248,113,113,0.2); } }
.video-area { position:absolute; top:48px; left:0; right:0; bottom:0; display:flex; align-items:center; justify-content:center; overflow:hidden; }
.canvas { display:block; max-width:100%; max-height:100%; }
.connecting-overlay { position:absolute; inset:0; display:flex; flex-direction:column; align-items:center; justify-content:center; gap:16px; background:rgba(10,10,15,0.9); }
.spinner { width:40px; height:40px; border:3px solid #2a2a45; border-top-color:#4f8ef7; border-radius:50%; animation:spin 0.8s linear infinite; }
@keyframes spin { to { transform:rotate(360deg); } }
.debug-panel { position:absolute; top:60px; right:16px; background:rgba(0,0,0,0.8); border:1px solid #2a2a45; border-radius:6px; padding:12px; font-size:12px; z-index:50; }
.debug-item { display:flex; justify-content:space-between; gap:16px; padding:4px 0; .label { color:#8888aa; } .value { color:#e8e8f0; font-family:monospace; } &.mouse-pos .value { color:#4f8ef7; } }
.bottom-bar { position:absolute; bottom:0; left:0; right:0; height:40px; display:flex; align-items:center; justify-content:center; background:linear-gradient(to top,rgba(0,0,0,0.8),transparent); z-index:100; opacity:0; transition:opacity 0.3s; }
.remote-view:hover & { opacity:1; }
.input-hints { display:flex; gap:24px; font-size:12px; color:#8888aa; }
</style>
"""

with open(r'D:\Android\AtlasRemote\apps\web\src\views\ConnectView.vue', 'w', encoding='utf-8') as f:
    f.write(connect_view)
with open(r'D:\Android\AtlasRemote\apps\web\src\views\RemoteView.vue', 'w', encoding='utf-8') as f:
    f.write(remote_view)
print('Both files written successfully')
