import { reactive } from 'vue'

export interface ConnectionState {
  status: 'disconnected' | 'connecting' | 'connected' | 'error'
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
    status: 'disconnected',
    deviceId: '',
    pairCode: '',
    latency: 0,
    fps: 0,
    bitrate: 0,
    width: 0,
    height: 0,
  })
  private onFrameCallback: ((data: Blob) => void) | null = null
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null
  private readonly RECONNECT_DELAY = 3000

  get connectionState() { return this.state }
  get isConnected() { return this.state.status === 'connected' }

  onFrame(callback: (data: Blob) => void) {
    this.onFrameCallback = callback
  }

  connect(host: string, port: number): Promise<void> {
    return new Promise((resolve, reject) => {
      if (this.ws) this.disconnect()
      this.state.status = 'connecting'
      this.state.deviceId = ''
      this.state.pairCode = ''
      const wsUrl = 'ws://' + host + ':' + port
      this.ws = new WebSocket(wsUrl)
      this.ws.onopen = () => {
        this.state.status = 'connected'
        this.state.latency = 0
        resolve()
      }
      this.ws.onmessage = (event) => {
        this.handleMessage(event.data)
      }
      this.ws.onerror = () => {
        this.state.status = 'error'
        reject(new Error('WebSocket error'))
      }
      this.ws.onclose = () => {
        this.state.status = 'disconnected'
        this.state.deviceId = ''
        this.state.pairCode = ''
        this.scheduleReconnect(host, port)
      }
    })
  }

  private handleMessage(data: string | Blob) {
    if (typeof data === 'string') {
      try {
        const msg = JSON.parse(data)
        if (msg.type === 'pair') {
          this.state.deviceId = msg.deviceId
          this.state.pairCode = msg.pairCode
        } else if (msg.type === 'status') {
          this.state.width = msg.width || 0
          this.state.height = msg.height || 0
        }
      } catch {}
    } else if (this.onFrameCallback) {
      this.onFrameCallback(data)
    }
  }

  private scheduleReconnect(host: string, port: number) {
    if (this.reconnectTimer) clearTimeout(this.reconnectTimer)
    this.reconnectTimer = setTimeout(() => {
      if (this.state.status === 'disconnected') {
        this.connect(host, port).catch(() => {})
      }
    }, this.RECONNECT_DELAY)
  }

  sendMouse(x: number, y: number) {
    this.ws?.send(JSON.stringify({ type: 'mouse_move', x, y }))
  }

  sendClick(button: 'left' | 'right' = 'left', pressed: boolean = true) {
    this.ws?.send(JSON.stringify({ type: 'mouse_click', button, pressed }))
  }

  sendWheel(delta: number) {
    this.ws?.send(JSON.stringify({ type: 'wheel', delta }))
  }

  sendKey(code: string, pressed: boolean = true) {
    this.ws?.send(JSON.stringify({ type: 'key', code, pressed }))
  }

  sendPairCode(code: string) {
    this.ws?.send(JSON.stringify({ type: 'pair_accept', code }))
  }

  disconnect() {
    if (this.reconnectTimer) clearTimeout(this.reconnectTimer)
    this.ws?.close()
    this.ws = null
    this.state.status = 'disconnected'
  }
}

export const client = new AtlasClient()
