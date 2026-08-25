import { reactive } from "vue"

export interface ConnectionState {
  status: "disconnected" | "connecting" | "connected" | "error"
  deviceId: string
  pairCode: string
  latency: number
  fps: number
  bitrate: number
  width: number
  height: number
}

export class AtlasClient {
  private ws: WebSocket | null = null
  private state = reactive<ConnectionState>({
    status: "disconnected",
    deviceId: "",
    pairCode: "",
    latency: 0,
    fps: 0,
    bitrate: 0,
    width: 0,
    height: 0,
  })
  private onFrameCallback: ((data: Blob) => void) | null = null
  private onPairCallback: ((deviceId: string, pairCode: string) => void) | null = null
  private onPairFailCallback: ((reason: string) => void) | null = null
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null
  private readonly RECONNECT_DELAY = 3000
  private relayHost = "127.0.0.1"
  private relayPort = 8080

  get connectionState() { return this.state }
  get isConnected() { return this.state.status === "connected" }
  get relayHost() { return this.relayHost }
  get relayPort() { return this.relayPort }

  setRelay(host: string, port: number) {
    this.relayHost = host
    this.relayPort = port
  }

  onFrame(callback: (data: Blob) => void) {
    this.onFrameCallback = callback
  }

  onPair(callback: (deviceId: string, pairCode: string) => void) {
    this.onPairCallback = callback
  }

  onPairFail(callback: (reason: string) => void) {
    this.onPairFailCallback = callback
  }

  connect(deviceId: string, pairCode: string): Promise<void> {
    return new Promise((resolve, reject) => {
      if (this.ws) this.disconnect()
      this.state.status = "connecting"
      this.state.deviceId = deviceId
      this.state.pairCode = pairCode
      const params = new URLSearchParams({ device: deviceId, code: pairCode })
      const wsUrl = `ws://${this.relayHost}:${this.relayPort}?${params.toString()}`
      this.ws = new WebSocket(wsUrl)
      this.ws.onopen = () => {
        this.state.status = "connected"
        this.state.latency = 0
        resolve()
      }
      this.ws.onmessage = (event) => {
        this.handleMessage(event.data)
      }
      this.ws.onerror = () => {
        this.state.status = "error"
        reject(new Error("WebSocket error"))
      }
      this.ws.onclose = () => {
        this.state.status = "disconnected"
        this.scheduleReconnect(deviceId, pairCode)
      }
    })
  }

  private handleMessage(data: string | Blob) {
    if (typeof data === "string") {
      try {
        const msg = JSON.parse(data)
        if (msg.type === "pair") {
          this.state.deviceId = msg.deviceId
          this.state.pairCode = msg.pairCode || ""
          this.onPairCallback?.(msg.deviceId, msg.pairCode || "")
        } else if (msg.type === "pair_ok") {
          this.state.deviceId = msg.deviceId
          this.state.status = "connected"
        } else if (msg.type === "pair_fail") {
          this.state.status = "error"
          this.onPairFailCallback?.(msg.reason || "Invalid pair code")
        } else if (msg.type === "status") {
          this.state.width = msg.width || 0
          this.state.height = msg.height || 0
        }
      } catch {}
    } else if (this.onFrameCallback) {
      this.onFrameCallback(data)
    }
  }

  private scheduleReconnect(deviceId: string, pairCode: string) {
    if (this.reconnectTimer) clearTimeout(this.reconnectTimer)
    this.reconnectTimer = setTimeout(() => {
      if (this.state.status === "disconnected" && deviceId) {
        this.connect(deviceId, pairCode).catch(() => {})
      }
    }, this.RECONNECT_DELAY)
  }

  sendMouse(x: number, y: number) {
    this.ws?.send(JSON.stringify({ type: "mouse_move", x, y }))
  }

  sendClick(button: "left" | "right" = "left", pressed: boolean = true) {
    this.ws?.send(JSON.stringify({ type: "mouse_click", button, pressed }))
  }

  sendWheel(delta: number) {
    this.ws?.send(JSON.stringify({ type: "wheel", delta }))
  }

  sendKey(code: string, pressed: boolean = true) {
    this.ws?.send(JSON.stringify({ type: "key", code, pressed }))
  }

  sendPairCode(code: string) {
    this.ws?.send(JSON.stringify({ type: "pair_accept", code }))
  }

  disconnect() {
    if (this.reconnectTimer) clearTimeout(this.reconnectTimer)
    this.ws?.close()
    this.ws = null
    this.state.status = "disconnected"
    this.state.deviceId = ""
    this.state.pairCode = ""
  }
}

export const client = new AtlasClient()
