<script setup lang="ts">
import { Wallet, KeyRound, Sparkles } from 'lucide-vue-next'
import { Card, CardContent } from '@/components/ui/card'
import { Logo } from '@/components/commons'

const emit = defineEmits<{
  goToCreate: []
  goToRecover: []
}>()

const handleGoToCreate = () => {
  emit('goToCreate')
}

const handleRecover = () => {
  emit('goToRecover')
}
</script>

<template>
  <div class="relative flex min-h-full flex-col bg-linear-to-br from-background via-background to-primary/5 p-4">

    <!-- Content Container -->
    <div class="flex flex-1 flex-col items-center justify-center">
      <div class="w-full max-w-md space-y-8">
        <!-- Logo and Branding -->
        <div class="flex flex-col items-center space-y-6 text-center">
          <Logo />

          <div class="space-y-2">
            <h1 class="text-4xl font-bold tracking-tight text-foreground">
              XNT Wallet
            </h1>
          </div>
        </div>

        <!-- Action Cards -->
        <div class="space-y-4">
          <!-- Create New Wallet Card -->
          <Card class="border-2 border-primary/20 bg-linear-to-br from-primary/5 to-accent/5 transition-all hover:border-primary/40 hover:shadow-lg">
            <CardContent class="p-6">
              <button
                class="w-full space-y-3 text-left"
                @click="handleGoToCreate"
              >
                <div class="flex items-center gap-4">
                  <div class="flex h-12 w-12 items-center justify-center rounded-xl bg-primary/10">
                    <Sparkles :size="24" class="text-primary" />
                  </div>
                  <div class="flex-1">
                    <h3 class="text-lg font-semibold text-foreground">Create New Wallet</h3>
                    <p class="text-sm text-muted-foreground">Start fresh with a new wallet</p>
                  </div>
                </div>
              </button>
            </CardContent>
          </Card>

          <!-- Recover Wallet Card -->
          <Card class="border-2 border-border/50 transition-all hover:border-border hover:shadow-lg">
            <CardContent class="p-6">
              <button
                class="w-full space-y-3 text-left"
                @click="handleRecover"
              >
                <div class="flex items-center gap-4">
                  <div class="flex h-12 w-12 items-center justify-center rounded-xl bg-muted">
                    <KeyRound :size="24" class="text-muted-foreground" />
                  </div>
                  <div class="flex-1">
                    <h3 class="text-lg font-semibold text-foreground">Import Wallet</h3>
                    <p class="text-sm text-muted-foreground">Recover with seed phrase</p>
                  </div>
                </div>
              </button>
            </CardContent>
          </Card>
        </div>

        <!-- Info Badge -->
        <div class="rounded-xl border border-border/50 bg-muted/30 p-4 backdrop-blur-sm">
          <p class="text-center text-sm text-muted-foreground">
            <Wallet :size="16" class="mr-2 inline" />
            XNT Wallet is a secure, decentralized wallet that puts you in control
          </p>
        </div>
      </div>
    </div>

    <!-- Footer -->
    <div class="py-4 text-center text-xs text-muted-foreground">
      Made with 💙 for the XNT community
    </div>
  </div>
</template>
