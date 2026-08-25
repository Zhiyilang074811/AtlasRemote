<template>
  <div class="remote-view" :class="{ fullscreen, pointerLock }">
    <div class="top-bar" v-show="!fullscreen || showControls">
      <div class="bar-left">
        <router-link to="/" class="back-btn">Back</router-link>
        <div class="connection-status" :class="statusColor">
          <span class="dot"></span>
          <span>{{ statusText }}</span>
        </div>
        <div v-if="appStore.deviceId" class="device-id">ID: {{ appStore.deviceId }}</div>
      </div>
      <div class="bar-right">
        <button @click="toggleFullscreen" class="bar-btn">{{ fullscreen ? "Exit" : "FS" }}</button>
        <button @click="toggleDebug" class="bar-btn">Debug</button>
        <button @click="sendCtrlAltDel" class="bar-btn danger">Ctrl+Alt+Del</button>
      </div>
    </div>
    <div class="video-area" ref="videoAreaRef">
      <canvas ref="canvasRef" class="canvas" />
      <div v-if="!connected" class="connecting-overlay">
        <div class="spinner"></div>
        <p>Connecting...</p>
      </div>
      <div v-if="showDebug" class="debug-panel">
        <div class="debug-item"><span class="label">Res</span><span class="value">{{ width }}x{{ height }}</span></div>
        <div class="debug-item"><span class="label">FPS</span><span class="value">{{ fps }}</span></div>
        <div class="debug-item"><span class="label">Latency</span><span class="value">{{ latency }}ms</span></div>
        <div class="debug-item mouse-pos"><span class="label">Mouse</span><span class="value">{{ mouseX }}, {{ mouseY }}</span></div>
      </div>
    </div>
    <div class="bottom-bar" v-show="showControls || !fullscreen">
      <div class="input-hints">
        <span>Mouse: Move</span>
        <span>Keyboard: Capture</span>
        <span>Right-click: Menu</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from "vue"
import { useRoute, useRouter } from "vue-router"
import { useAppStore } from "@/store/app"
import { useLogStore } from "@/store/log"
import { client } from "@/utils/client"

const route = useRoute()
const router = useRouter()
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

const statusColor = computed(() => {
  const s = appStore.connectionStatus
  return s === "connected" ? "online" : s === "connecting" ? "connecting" : s === "error" ? "error" : "offline"
})
const statusText = computed(() => {
  const s = appStore.connectionStatus
  return s === "connected" ? "Connected" : s === "connecting" ? "Connecting..." : s === "error" ? "Failed" : "Disconnected"
})

let ctx: CanvasRenderingContext2D | null = null
let animFrame: number
let frameCount = 0
let lastFpsTime = Date.now()
let lastFrameTime = 0

onMounted(async () => {
  if (canvasRef.value) ctx = canvasRef.value.getContext("2d")
  const deviceId = route.params.deviceId as string || "local"
  const pairCode = appStore.pairCode || "000000"

  try {
    await client.connect(deviceId, pairCode)
    appStore.deviceId = deviceId
    appStore.connectionStatus = "connected"
    connected.value = true
    logStore.add("info", "Remote desktop connected")
    client.onFrame(handleVideoFrame)
    animFrame = requestAnimationFrame(updateFps)
    videoAreaRef.value?.addEventListener("click", () => videoAreaRef.value?.requestPointerLock())
    document.addEventListener("pointerlockchange", () => {
      pointerLock.value = !!document.pointerLockElement
    })
    document.addEventListener("mousemove", onMouseMove)
    document.addEventListener("keydown", onKeyDown)
    document.addEventListener("keyup", onKeyUp)
    document.addEventListener("contextmenu", onContextMenu)
    document.addEventListener("wheel", onWheel, { passive: false })
  } catch (e: any) {
    appStore.connectionStatus = "error"
    logStore.add("error", "Failed: " + e.message)
  }
})

onUnmounted(() => {
  cancelAnimationFrame(animFrame)
  client.disconnect()
  document.removeEventListener("pointerlockchange", () => {})
  document.removeEventListener("mousemove", onMouseMove)
  document.removeEventListener("keydown", onKeyDown)
  document.removeEventListener("keyup", onKeyUp)
  document.removeEventListener("contextmenu", onContextMenu)
  document.removeEventListener("wheel", onWheel)
})

function handleVideoFrame(blob: Blob) {
  if (!ctx || !canvasRef.value) return
  const img = new Image()
  const url = URL.createObjectURL(blob)
  img.onload = () => {
    const area = videoAreaRef.value
    if (!area) { URL.revokeObjectURL(url); return }
    const r = area.getBoundingClientRect()
    const imgAspect = img.width / img.height
    const areaAspect = r.width / r.height
    let dw = r.width, dh = r.height
    if (imgAspect > areaAspect) dh = r.width / imgAspect
    else dw = r.height * imgAspect
    const dx = (r.width - dw) / 2, dy = (r.height - dh) / 2
    canvasRef.value!.width = dw
    canvasRef.value!.height = dh
    if (ctx) ctx.drawImage(img, dx, dy, dw, dh)
    width.value = img.width
    height.value = img.height
    bitrate.value = blob.size * 8 / ((Date.now() - lastFrameTime) / 1000)
    lastFrameTime = Date.now()
    URL.revokeObjectURL(url)
  }
  img.src = url
}

function updateFps() {
  frameCount++
  const now = Date.now()
  if (now - lastFpsTime >= 1000) {
    fps.value = frameCount
    frameCount = 0
    lastFpsTime = now
  }
  animFrame = requestAnimationFrame(updateFps)
}

function onMouseMove(e: MouseEvent) {
  if (!pointerLock.value) return
  const area = videoAreaRef.value
  if (!area) return
  const r = area.getBoundingClientRect()
  const scaleX = width.value / r.width
  const scaleY = height.value / r.height
  mouseX.value = e.clientX - r.left
  mouseY.value = e.clientY - r.top
  const relX = (e.movementX / r.width) * scaleX
  const relY = (e.movementY / r.height) * scaleY
  if (relX !== 0 || relY !== 0) client.sendMouse(relX, relY)
}

function onKeyDown(e: KeyboardEvent) {
  if (e.ctrlKey && e.altKey && e.key === "Delete") { sendCtrlAltDel(); e.preventDefault(); return }
  client.sendKey(e.code, true)
}

function onKeyUp(e: KeyboardEvent) {
  client.sendKey(e.code, false)
}

function onContextMenu(e: MouseEvent) {
  e.preventDefault()
  client.sendClick("right", true)
  setTimeout(() => client.sendClick("right", false), 100)
}

function onWheel(e: WheelEvent) {
  e.preventDefault()
  client.sendWheel(e.deltaY > 0 ? -1 : 1)
}

function sendCtrlAltDel() {
  client.sendKey("ControlLeft", true)
  client.sendKey("AltLeft", true)
  client.sendKey("Delete", true)
  setTimeout(() => {
    client.sendKey("ControlLeft", false)
    client.sendKey("AltLeft", false)
    client.sendKey("Delete", false)
  }, 100)
}

function toggleFullscreen() {
  if (!document.fullscreenElement) {
    document.documentElement.requestFullscreen()
    fullscreen.value = true
  } else {
    document.exitFullscreen()
    fullscreen.value = false
  }
}

function toggleDebug() { showDebug.value = !showDebug.value }

watch(fullscreen, (v) => { if (!v) showControls.value = true })
</script>

<style lang="scss" scoped>
.remote-view {
  position: relative;
  width: 100vw;
  height: 100vh;
  background: #000;
  overflow: hidden;
  &.pointerLock { cursor: none; }
}
.top-bar {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 16px;
  background: linear-gradient(to bottom, rgba(0,0,0,0.8), transparent);
  z-index: 100;
  opacity: 0;
  transition: opacity 0.3s;
}
.remote-view:hover & { opacity: 1; }
.bar-left, .bar-right { display: flex; align-items: center; gap: 16px; }
.back-btn { color: #8888aa; text-decoration: none; font-size: 14px; &:hover { color: #e8e8f0; } }
.connection-status {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  .dot { width: 8px; height: 8px; border-radius: 50%; &.online { background: #34d399; } &.connecting { background: #fbbf24; animation: blink 1s infinite; } &.error { background: #f87171; } &.offline { background: #666; } }
}
@keyframes blink { 0%,100%{opacity:1} 50%{opacity:0.3} }
.device-id { font-size: 12px; color: #34d399; font-family: monospace; }
.bar-btn {
  padding: 6px 14px;
  background: rgba(255,255,255,0.1);
  border-radius: 4px;
  font-size: 13px;
  color: #e8e8f0;
  cursor: pointer;
  &:hover { background: rgba(255,255,255,0.2); }
  &.danger { color: #f87171; &:hover { background: rgba(248,113,113,0.2); } }
}
.video-area {
  position: absolute;
  top: 48px;
  left: 0;
  right: 0;
  bottom: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
}
.canvas { display: block; max-width: 100%; max-height: 100%; }
.connecting-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 16px;
  background: rgba(10,10,15,0.9);
}
.spinner {
  width: 40px;
  height: 40px;
  border: 3px solid #2a2a45;
  border-top-color: #4f8ef7;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}
@keyframes spin { to { transform: rotate(360deg); } }
.debug-panel {
  position: absolute;
  top: 60px;
  right: 16px;
  background: rgba(0,0,0,0.8);
  border: 1px solid #2a2a45;
  border-radius: 6px;
  padding: 12px;
  font-size: 12px;
  z-index: 50;
}
.debug-item { display: flex; justify-content: space-between; gap: 16px; padding: 4px 0; .label { color: #8888aa; } .value { color: #e8e8f0; font-family: monospace; } &.mouse-pos .value { color: #4f8ef7; } }
.bottom-bar {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(to top, rgba(0,0,0,0.8), transparent);
  z-index: 100;
  opacity: 0;
  transition: opacity 0.3s;
}
.remote-view:hover & { opacity: 1; }
.input-hints { display: flex; gap: 24px; font-size: 12px; color: #8888aa; }
</style>
