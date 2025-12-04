<script setup lang="ts">
import { Toaster } from '@/components/ui/sonner'
</script>

<template>
  <router-view />
  <Toaster richColors closeButton :duration="2000" />
</template>
