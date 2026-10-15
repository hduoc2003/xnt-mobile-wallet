import { defineStore } from 'pinia'
import { ref } from 'vue'
import {
  DEFAULT_LOW_MEMORY_PROVING,
  DEFAULT_MAX_INPUTS,
  MAX_INPUTS_LIMIT,
} from '@/utils/constants/constants'

const MAX_INPUTS_KEY = 'xntMaxInputsPerTx'
const LOW_MEMORY_KEY = 'xntLowMemoryProving'

const isValidMaxInputs = (n: number) => Number.isInteger(n) && n >= 1 && n <= MAX_INPUTS_LIMIT

export const useSettingsStore = defineStore('settings', () => {
  // A bad stored value resets to the default and is shown in Settings instead of blocking startup.
  const loadErrors = ref<string[]>([])

  const read = <T>(key: string, fallback: T, parse: (raw: string) => T | undefined): T => {
    const raw = localStorage.getItem(key)
    if (raw === null) return fallback
    const value = parse(raw)
    if (value !== undefined) return value
    localStorage.removeItem(key)
    loadErrors.value.push(`Saved setting ${key}=${raw} was invalid and has been reset`)
    return fallback
  }

  const maxInputsPerTx = ref(
    read(MAX_INPUTS_KEY, DEFAULT_MAX_INPUTS, (raw) =>
      isValidMaxInputs(Number(raw)) ? Number(raw) : undefined,
    ),
  )
  const lowMemoryProving = ref(
    read(LOW_MEMORY_KEY, DEFAULT_LOW_MEMORY_PROVING, (raw) =>
      raw === 'true' ? true : raw === 'false' ? false : undefined,
    ),
  )

  const setMaxInputsPerTx = (n: number) => {
    if (!isValidMaxInputs(n)) {
      throw new Error(`Max inputs per transaction must be 1 to ${MAX_INPUTS_LIMIT}`)
    }
    localStorage.setItem(MAX_INPUTS_KEY, String(n))
    maxInputsPerTx.value = n
  }

  const setLowMemoryProving = (on: boolean) => {
    localStorage.setItem(LOW_MEMORY_KEY, String(on))
    lowMemoryProving.value = on
  }

  return {
    maxInputsPerTx,
    lowMemoryProving,
    loadErrors,
    setMaxInputsPerTx,
    setLowMemoryProving,
  }
})
