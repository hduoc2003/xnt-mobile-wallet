<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { CheckCircle2, Wallet, Sparkles, ArrowRight, Shield, Lock } from 'lucide-vue-next'

interface Props {
  seedPhrase: string[]
  password: string
}

defineProps<Props>()

const emit = defineEmits<{
  accessWallet: []
  createAnother: []
}>()

const showConfetti = ref(false)

const handleAccessWallet = () => {
  emit('accessWallet')
}

onMounted(() => {
  showConfetti.value = true
})
</script>

<template>
  <div class="space-y-8">
    <div class="space-y-6">
      <div class="flex justify-center">
        <Logo />
      </div>

      <div class="grid gap-3">
        <Card class="border-2 border-green-500/20 bg-linear-to-br from-green-500/5 to-transparent">
          <CardContent class="flex items-center gap-4 p-4">
            <div class="flex h-12 w-12 items-center justify-center rounded-xl bg-primary/10">
              <Shield />
            </div>
            <div class="flex-1">
              <h3 class="font-semibold text-foreground">Secure & Private</h3>
              <p class="text-xs text-muted-foreground">Your keys, your crypto</p>
            </div>
          </CardContent>
        </Card>

        <Card
          class="border-2 border-green-500/20 bg-linear-to-br from-green-500/5 to-transparent"
        >
          <CardContent class="flex items-center gap-4 p-4">
            <div class="flex h-12 w-12 items-center justify-center rounded-xl bg-green-500/10">
              <Lock :size="24" class="text-green-600" />
            </div>
            <div class="flex-1">
              <h3 class="font-semibold text-foreground">Encrypted Locally</h3>
              <p class="text-xs text-muted-foreground">Protected by your password</p>
            </div>
          </CardContent>
        </Card>

        <Card class="border-2 border-green-500/20 bg-linear-to-br from-green-500/5 to-transparent">
          <CardContent class="flex items-center gap-4 p-4">
            <div class="flex h-12 w-12 items-center justify-center rounded-xl bg-accent/10">
              <Sparkles :size="24" />
            </div>
            <div class="flex-1">
              <h3 class="font-semibold text-foreground">Ready to Use</h3>
              <p class="text-xs text-muted-foreground">Start exploring now</p>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>

    <Alert class="border-2 border-primary/20 bg-primary/5">
      <Shield :size="18" />
      <AlertDescription class="text-sm">
        <strong>Remember:</strong> Keep your recovery phrase safe. You'll need it to restore your
        wallet if you ever lose access.
      </AlertDescription>
    </Alert>

    <div class="space-y-3">
      <Button
        size="lg"
        class="h-14 w-full gap-2 text-lg font-semibold shadow-lg"
        @click="handleAccessWallet"
      >
        <Wallet :size="24" />
        Open My Wallet
        <ArrowRight :size="20" class="ml-auto" />
      </Button>
    </div>
  </div>
</template>
