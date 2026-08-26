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
        <div class="debug-item"><span class="label">Codec</span><span class="value">{{ codecName }}</span></div>
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
const codecName = ref("BGRA")
const codecInitialized = ref(false)

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

// WebCodecs H.264 decoder
let videoDecoder: VideoDecoder | null = null
const decodedFrames: EncodedVideoChunk[] = []
let decoderConfigured = false
let pendingFrames: ImageData[] = []
const SPS_BUFFER: Uint8Array[] = []
const PPS_BUFFER: Uint8Array[] = []

function extractNalu(data: Uint8Array, offset: number, size: number): Uint8Array | null {
  if (offset + size > data.length) return null
  return data.subarray(offset, offset + size)
}

function findStartCode(data: Uint8Array, offset: number): number {
  if (offset + 4 <= data.length && data[offset] === 0 && data[offset + 1] === 0 && data[offset + 2] === 0 && data[offset + 3] === 1) return 4
  if (offset + 3 <= data.length && data[offset] === 0 && data[offset + 1] === 0 && data[offset + 2] === 1) return 3
  return 0
}

function parseH264SpsPps(nalu: Uint8Array): { width?: number; height?: number } {
  if (nalu.length < 4) return {}
  const rbsp = new Uint8Array(nalu.length - 4)
  for (let i = 4; i < nalu.length; i++) rbsp[i - 4] = nalu[i]
  const view = new DataView(rbsp.buffer, rbsp.byteOffset, rbsp.byteLength)
  const firstByte = rbsp[0]
  const sliceType = (firstByte >> 5) & 0x1f
  const picType = firstByte & 0x1f
  if (sliceType === 5) {
    const log2_max_frame_num = 4 + ((picType >> 4) & 0x03)
    const vui_parameters = rbsp[1]
    if (vui_parameters & 0x40) {
      let pos = 2
      const aspect_ratio = rbsp[pos++]
      if (aspect_ratio === 255) {
        const sar_width = view.getUint16(pos); pos += 2
        const sar_height = view.getUint16(pos); pos += 2
      }
    }
    const log2_max_pic_order = 4 + ((rbsp[pos++] & 0x03))
    const frame_mbs_only = ((rbsp[pos++] >> 7) & 0x01)
    const mb_address = (16 + (rbsp[pos] & 0x3f)) << 1
    const ref_frames = rbsp[pos++] & 0x1f
    const num_ref_idx = rbsp[pos++] & 0x1f
    const pic_width = (view.getUint16(pos) & 0x3ff) * 2 - 1 + mb_address
    pos += 2
    const pic_height = (2 - frame_mbs_only) * ((view.getUint16(pos) & 0x3ff) * 2 - 1 + mb_address)
    return { width: pic_width, height: pic_height }
  }
  return {}
}

function initVideoDecoder(width: number, height: number) {
  if (typeof VideoDecoder === "undefined") {
    logStore.add("warn", "VideoDecoder not supported in this browser")
    return
  }
  if (videoDecoder) {
    videoDecoder.close()
    videoDecoder = null
  }
  decoderConfigured = false
  SPS_BUFFER.length = 0
  PPS_BUFFER.length = 0

  videoDecoder = new VideoDecoder({
    output: (frame) => {
      if (!ctx || !canvasRef.value) { frame.close(); return }
      const area = videoAreaRef.value
      if (!area) { frame.close(); return }
      const r = area.getBoundingClientRect()
      const imgAspect = frame.displayWidth / frame.displayHeight
      const areaAspect = r.width / r.height
      let dw = r.width, dh = r.height
      if (imgAspect > areaAspect) dh = r.width / imgAspect
      else dw = r.height * imgAspect
      const dx = (r.width - dw) / 2, dy = (r.height - dh) / 2
      canvasRef.value.width = frame.displayWidth
      canvasRef.value.height = frame.displayHeight
      ctx.drawImage(frame, 0, 0)
      // Scale to fit
      ctx.save()
      ctx.drawImage(canvasRef.value, 0, 0, frame.displayWidth, frame.displayHeight, dx, dy, dw, dh)
      ctx.restore()
      width.value = frame.displayWidth
      height.value = frame.displayHeight
      codecName.value = "H264"
      frame.close()
    },
    error: (e) => {
      logStore.add("error", `VideoDecoder error: ${e.message}`)
    },
  })

  videoDecoder.configure({
    codec: "avc1.42001e",
    width,
    height,
    description: createAVCC(SPS_BUFFER, PPS_BUFFER),
  })
  decoderConfigured = true
  codecInitialized.value = true
  logStore.add("info", `H264 decoder configured: ${width}x${height}`)
}

function createAVCC(sps: Uint8Array[], pps: Uint8Array[]): Uint8Array {
  const spsData = sps[0]
  const ppsData = pps[0]
  if (!spsData || !ppsData) return new Uint8Array(0)

  const spsLength = spsData.length
  const ppsLength = ppsData.length

  const avcc = new Uint8Array(7 + 2 + spsLength + 1 + 2 + ppsLength)
  let offset = 0
  avcc[offset++] = 1 // version
  avcc[offset++] = spsData[1] // profile
  avcc[offset++] = spsData[2] // compatibility
  avcc[offset++] = spsData[3] // level
  avcc[offset++] = 0xFF // 6 bits reserved + 2 bits numSps
  avcc[offset++] = 0xE1 // 3 bits reserved + numPPS
  // SPS
  avcc[offset++] = (spsLength >> 8) & 0xFF
  avcc[offset++] = spsLength & 0xFF
  avcc.set(spsData, offset)
  offset += spsLength
  // PPS
  avcc[offset++] = (ppsLength >> 8) & 0xFF
  avcc[offset++] = ppsLength & 0xFF
  avcc.set(ppsData, offset)
  return avcc
}

function parseATLSHeader(data: Uint8Array): { codec: number; width: number; height: number; payloadOffset: number; payloadLength: number } | null {
  if (data.length < 36) return null
  const magic = String.fromCharCode(data[0], data[1], data[2], data[3])
  if (magic !== "ATLS") return null
  const codec = data[20] | (data[21] << 8)
  const width = data[12] | (data[13] << 8) | (data[14] << 16) | (data[15] << 24)
  const height = data[16] | (data[17] << 8) | (data[18] << 16) | (data[19] << 24)
  const payloadLength = data[8] | (data[9] << 8) | (data[10] << 16) | (data[11] << 24)
  return { codec, width, height, payloadOffset: 36, payloadLength }
}

function handleVideoFrame(blob: Blob) {
  const reader = new FileReader()
  reader.onload = () => {
    const data = new Uint8Array(reader.result as ArrayBuffer)
    const header = parseATLSHeader(data)
    if (!header) {
      logStore.add("warn", "Invalid ATLS frame header")
      return
    }

    const { codec, width: w, height: h, payloadOffset, payloadLength } = header
    const payload = data.subarray(payloadOffset, payloadOffset + payloadLength)

    if (codec === 2) {
      // H264
      codecName.value = "H264"
      if (!videoDecoder) {
        initVideoDecoder(w, h)
        return
      }
      if (!decoderConfigured) {
        initVideoDecoder(w, h)
        return
      }

      // Parse NAL units to extract SPS/PPS
      let offset = 0
      while (offset < payload.length) {
        const scLen = findStartCode(payload, offset)
        if (scLen === 0) { offset++; continue }
        const naluType = payload[offset + scLen] & 0x1f
        let naluEnd = payload.length
        for (let i = offset + scLen; i < payload.length - 3; i++) {
          if (payload[i] === 0 && payload[i + 1] === 0 && (payload[i + 2] === 0 || payload[i + 3] === 1)) {
            naluEnd = i
            break
          }
        }
        const naluSize = naluEnd - offset
        const nalu = payload.subarray(offset, naluEnd)

        if (naluType === 7) {
          SPS_BUFFER.length = 0
          SPS_BUFFER.push(nalu)
          const spsInfo = parseH264SpsPsp(nalu)
          if (spsInfo.width && spsInfo.height && spsInfo.width !== w) {
            initVideoDecoder(spsInfo.width, spsInfo.height)
          }
        } else if (naluType === 8) {
          PPS_BUFFER.length = 0
          PPS_BUFFER.push(nalu)
          if (decoderConfigured && SPS_BUFFER.length > 0) {
            videoDecoder?.configure({
              codec: "avc1.42001e",
              width: w,
              height: h,
              description: createAVCC(SPS_BUFFER, PPS_BUFFER),
            })
            decoderConfigured = true
          }
        }

        // Decode the NAL unit as a complete frame
        if (naluType === 1 || naluType === 5 || naluType === 6) {
          try {
            const chunk = new EncodedVideoChunk({
              type: naluType === 5 ? "key" : "delta",
              timestamp: Date.now() * 1000,
              data: nalu,
            })
            videoDecoder?.decode(chunk)
          } catch (e) {
            logStore.add("warn", `Decode error: ${e}`)
          }
        }

        offset = naluEnd + (naluEnd < payload.length ? scLen : 0)
      }
      bitrate.value = data.length * 8 / ((Date.now() - lastFrameTime) / 1000)
      lastFrameTime = Date.now()
      } else {
        // BGRA / raw - render directly to canvas
        codecName.value = 'BGRA'
        if (!ctx || !canvasRef.value) return
        try {
          const rgba = new Uint8ClampedArray(payload.length)
          for (let i = 0; i < payload.length; i += 4) {
            rgba[i]     = payload[i + 2]
            rgba[i + 1] = payload[i + 1]
            rgba[i + 2] = payload[i]
            rgba[i + 3] = payload[i + 3]
          }
          const bmp = createImageBitmap(new ImageData(new Uint8ClampedArray(rgba), w, h))
          canvasRef.value.width = w
          canvasRef.value.height = h
          ctx.drawImage(bmp, 0, 0)
          bmp.close()
          width.value = w
          height.value = h
          bitrate.value = data.length * 8 / ((Date.now() - lastFrameTime) / 1000)
          lastFrameTime = Date.now()
        } catch(e) {
          logStore.add('error', 'BGRA render error: ' + e)
        }
      }

  }
  reader.readAsArrayBuffer(blob)
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

onMounted(async () => {
  if (canvasRef.value) ctx = canvasRef.value.getContext("2d")
  const deviceId = (route.params.deviceId as string) || "local"
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
  videoDecoder?.close()
  videoDecoder = null
  decoderConfigured = false
  client.disconnect()
  document.removeEventListener("pointerlockchange", () => {})
  document.removeEventListener("mousemove", onMouseMove)
  document.removeEventListener("keydown", onKeyDown)
  document.removeEventListener("keyup", onKeyUp)
  document.removeEventListener("contextmenu", onContextMenu)
  document.removeEventListener("wheel", onWheel)
})
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
