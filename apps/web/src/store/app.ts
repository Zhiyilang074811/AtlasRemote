import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useAppStore = defineStore('app', () => {
  const host = ref('127.0.0.1')
  const port = ref(9090)
  const pairCode = ref('')
  const deviceId = ref('')
  const isFullscreen = ref(false)
  const showDebug = ref(false)
  const connectionStatus = ref<'disconnected' | 'connecting' | 'connected' | 'error'>('disconnected')

  return { host, port, pairCode, deviceId, isFullscreen, showDebug, connectionStatus }
})
