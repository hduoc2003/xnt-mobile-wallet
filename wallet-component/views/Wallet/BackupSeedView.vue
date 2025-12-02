<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { toast } from 'vue-sonner'
import { Key, ArrowLeft, Copy, Eye, EyeOff } from 'lucide-vue-next'
import { Button } from '@/components/ui/button'
import { Label } from '@/components/ui/label'
import { Input } from '@/components/ui/input'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'

const router = useRouter()

const verifyPassword = ref('')
const showPassword = ref(false)
const verifyLoading = ref(false)
const seedPhrase = ref<string[]>([])
const seedRevealed = ref(false)

const handleVerifyPassword = async () => {
  try {
    verifyLoading.value = true

    // TODO: Implement decrypt keystore with Tauri command
    // const result = await decryptKeystore(verifyPassword.value)
    // seedPhrase.value = result.seedPhrase

    // Mock: Use test seed phrase
    seedPhrase.value = [
      'abandon',
      'ability',
      'able',
      'about',
      'above',
      'absent',
      'absorb',
      'abstract',
      'absurd',
      'abuse',
      'access',
      'accident',
      'account',
      'accuse',
      'achieve',
      'acid',
      'acoustic',
      'acquire',
    ]

    seedRevealed.value = true
    toast.success('Seed phrase revealed')
  } catch (error) {
    toast.error('Invalid password')
  } finally {
    verifyLoading.value = false
  }
}

const handleCopySeed = async () => {
  try {
    await navigator.clipboard.writeText(seedPhrase.value.join(' '))
    toast.success('Seed phrase copied to clipboard')
  } catch (error) {
    toast.error('Failed to copy seed phrase')
  }
}
</script>

<template>
  <div class="p-4">
    <div class="mx-auto max-w-2xl space-y-6">
      <!-- Header -->
      <div class="flex items-center gap-3">
        <Button variant="ghost" size="icon" @click="router.push('/')">
          <ArrowLeft class="size-5" />
        </Button>
        <div>
          <h1 class="text-2xl font-bold text-foreground">Backup Seed Phrase</h1>
          <p class="text-sm text-muted-foreground">Verify password to view your seed phrase</p>
        </div>
      </div>

      <!-- Backup Seed Card -->
      <Card>
        <CardHeader>
          <CardTitle class="flex items-center gap-2">
            <Key class="size-5" />
            {{ seedRevealed ? 'Your Seed Phrase' : 'Verify Password' }}
          </CardTitle>
        </CardHeader>
        <CardContent class="space-y-4">
          <Alert v-if="!seedRevealed" variant="destructive">
            <AlertDescription>
              Never share your seed phrase with anyone. It gives full access to your wallet.
            </AlertDescription>
          </Alert>

          <div v-if="!seedRevealed" class="space-y-2">
            <Label for="password">Password</Label>
            <div class="relative">
              <Input
                id="password"
                v-model="verifyPassword"
                :type="showPassword ? 'text' : 'password'"
                placeholder="Enter your password"
                :disabled="verifyLoading"
                @keyup.enter="handleVerifyPassword"
              />
              <Button
                variant="ghost"
                size="icon"
                class="absolute right-0 top-0 h-full"
                @click="showPassword = !showPassword"
              >
                <Eye v-if="showPassword" class="size-4" />
                <EyeOff v-else class="size-4" />
              </Button>
            </div>
          </div>

          <div v-else class="space-y-4">
            <Alert>
              <AlertDescription>
                Write down these 18 words in order and store them safely offline.
              </AlertDescription>
            </Alert>

            <div class="grid grid-cols-2 gap-3 rounded-lg border border-border bg-muted/30 p-4 sm:grid-cols-3">
              <div
                v-for="(word, index) in seedPhrase"
                :key="index"
                class="flex items-center gap-2 rounded-md border border-border bg-background p-2"
              >
                <Badge variant="secondary" class="shrink-0 text-xs">{{ index + 1 }}</Badge>
                <span class="font-mono text-sm">{{ word }}</span>
              </div>
            </div>

            <Button variant="outline" class="w-full" @click="handleCopySeed">
              <Copy class="mr-2 size-4" />
              Copy Seed Phrase
            </Button>
          </div>

          <!-- Action Buttons -->
          <div class="flex gap-2">
            <Button variant="outline" class="flex-1" @click="router.push('/')">
              {{ seedRevealed ? 'Close' : 'Cancel' }}
            </Button>
            <Button
              v-if="!seedRevealed"
              class="flex-1"
              :disabled="!verifyPassword || verifyLoading"
              @click="handleVerifyPassword"
            >
              <span
                v-if="verifyLoading"
                class="mr-2 size-4 animate-spin rounded-full border-2 border-white border-t-transparent"
              />
              {{ verifyLoading ? 'Verifying...' : 'Reveal Seed' }}
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  </div>
</template>

