<script setup lang="ts">
import { computed } from 'vue'
import { useColorMode } from '@vueuse/core'
import { Moon, Sun } from 'lucide-vue-next'
import { Switch } from '@/components/ui/switch'

const mode = useColorMode()

const isDark = computed({
  get: () => mode.value === 'dark',
  set: (value: boolean) => {
    mode.value = value ? 'dark' : 'light'
  },
})
</script>

<template>
  <div class="flex items-center gap-2">
    <Sun class="h-4 w-4 text-muted-foreground transition-colors" :class="{ 'text-foreground': !isDark }" />
    <Switch v-model="isDark" />
    <Moon class="h-4 w-4 text-muted-foreground transition-colors" :class="{ 'text-foreground': isDark }" />
  </div>
</template>

