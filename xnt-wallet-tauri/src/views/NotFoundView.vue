<script setup lang="ts">
const router = useRouter()

const goHome = () => {
  router.push({ name: 'wallet' })
}
</script>

<template>
  <div class="flex min-h-screen items-center justify-center">
    <div class="text-center">
      <h1 class="text-6xl font-bold text-foreground">404</h1>
      <p class="mt-4 text-xl text-muted-foreground">Page not found</p>
      <Button class="mt-8" @click="goHome">Go Home</Button>
    </div>
  </div>
</template>

