<script setup lang="ts">
import { computed } from 'vue'
import { useColorMode } from '@vueuse/core'

const mode = useColorMode()
const isDark = computed(() => mode.value === 'dark')
</script>

<template>
  <div class="relative">
    <div
      v-if="isDark"
      class="absolute -inset-1 animate-pulse rounded-full bg-linear-to-r from-primary via-accent to-primary opacity-75 blur-xl"
    ></div>
    <div
      class="relative flex h-24 w-24 items-center justify-center rounded-full bg-linear-to-br from-primary to-accent shadow-2xl ring-4 ring-background"
    >
      <img
        src="@/assets/imgs/neptune_logo.jpg"
        alt="Neptune Privacy"
        class="h-20 w-20 rounded-full object-cover"
      />
    </div>
  </div>
</template>
