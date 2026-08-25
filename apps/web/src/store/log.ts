import { defineStore } from 'pinia'
import { ref } from 'vue'

export interface LogEntry {
  time: string
  level: 'info' | 'warn' | 'error'
  message: string
}

export const useLogStore = defineStore('log', () => {
  const entries = ref<LogEntry[]>([])

  function add(level: LogEntry['level'], message: string) {
    entries.value.unshift({
      time: new Date().toLocaleTimeString(),
      level,
      message,
    })
    if (entries.value.length > 200) entries.value.pop()
  }

  function clear() { entries.value = [] }

  return { entries, add, clear }
})
