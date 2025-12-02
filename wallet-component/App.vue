<script setup lang="ts">
import { Toaster } from '@/components/ui/sonner'
</script>

<template>
    <router-view />
    <Toaster richColors position="top-center" :duration="2000" closeButton  />
</template>


