<script setup lang="ts">
import { ArrowLeft } from 'lucide-vue-next'
import { Button } from '@/components/ui/button'

const emit = defineEmits<{
  click: []
}>()

const handleClick = () => {
  emit('click')
}
</script>

<template>
  <Button variant="ghost" size="icon-lg" @click="handleClick">
    <ArrowLeft />
  </Button>
</template>
