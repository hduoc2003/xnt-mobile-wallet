import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export const useTickerStore = defineStore('ticker', () => {
  const xntPrice = ref<number | null>(null)
  const xntChange24h = ref<number | null>(null)

  // ===== SETTERS =====

  const setXntPrice = (price: number | null) => {
    xntPrice.value = price
  }

  const setXntChange24h = (percent: number | null) => {
    xntChange24h.value = percent
  }

  // ===== GETTERS =====
  const getXntPrice = computed(() => xntPrice.value)
  const getXntChange24h = computed(() => xntChange24h.value)

  return {
    getXntPrice,
    setXntPrice,
    getXntChange24h,
    setXntChange24h,
  }
})
