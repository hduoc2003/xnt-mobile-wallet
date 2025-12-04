import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export const useTickerStore = defineStore('ticker', () => {
    const xntPrice = ref<number | null>(null)

    // ===== SETTERS =====

    const setXntPrice = (price: number | null) => {
        xntPrice.value = price
    }

    // ===== GETTERS =====
    const getXntPrice = computed(() => xntPrice.value)

    return {
        getXntPrice,
        setXntPrice,
    }
})
