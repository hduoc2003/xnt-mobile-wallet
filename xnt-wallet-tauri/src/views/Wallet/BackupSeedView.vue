<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import { toast } from 'vue-sonner'
import { Key, ArrowLeft, Eye, EyeOff } from 'lucide-vue-next'
import { SeedPhraseGrid } from '@/components/commons'
import { walletApi } from '@/api/tauri/walletApi'
import { useAccountsStore } from '@/stores'

const router = useRouter()
const accountsStore = useAccountsStore()
/** The active account's seed; its other accounts share these words. */
const seedId = accountsStore.findAccount(accountsStore.requireActiveId()).seedId
const seedTitle = computed(() => accountsStore.seedTitle(seedId))
const backedUp = ref(accountsStore.registry?.seeds.find((s) => s.id === seedId)?.backedUp === true)

const verifyPassword = ref('')
const showPassword = ref(false)
const verifyLoading = ref(false)
const seedPhrase = ref<string[]>([])
const seedRevealed = ref(false)
const seedPhraseGridRef = ref<InstanceType<typeof SeedPhraseGrid>>()

const handleVerifyPassword = async () => {
  try {
    verifyLoading.value = true

    seedPhrase.value = await walletApi.accountsSeedPhrase(seedId, verifyPassword.value)

    seedRevealed.value = true
    toast.success('Seed phrase revealed')
  } catch (error) {
    toast.error('Could not reveal the seed phrase', {
      description: error instanceof Error ? error.message : String(error),
    })
  } finally {
    verifyLoading.value = false
  }
}

const onBackedUpChange = async (event: Event) => {
  const box = event.target
  if (!(box instanceof HTMLInputElement) || !box.checked) return
  try {
    accountsStore.setRegistry(await walletApi.accountsMarkBackedUp(seedId))
    backedUp.value = true
  } catch (error: unknown) {
    box.checked = false
    toast.error('Could not save the backup state', {
      description: error instanceof Error ? error.message : String(error),
    })
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
          <p class="text-sm text-muted-foreground">{{ seedTitle }}</p>
          <p v-if="!seedRevealed" class="text-sm text-muted-foreground">
            Verify password to view your seed phrase
          </p>
          <p v-if="seedRevealed" class="text-sm text-muted-foreground">
            Write down these 18 words in order
          </p>
        </div>
      </div>

      <!-- Password Verification Card -->
      <Card v-if="!seedRevealed">
        <CardHeader>
          <CardTitle class="flex items-center gap-2">
            <Key class="size-5" />
            Verify Password
          </CardTitle>
        </CardHeader>
        <CardContent class="space-y-4">
          <Alert variant="destructive">
            <AlertDescription>
              Never share your seed phrase with anyone. It gives full access to your wallet.
            </AlertDescription>
          </Alert>

          <div class="space-y-2">
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

          <!-- Action Buttons -->
          <div class="flex gap-2">
            <Button variant="outline" class="flex-1" @click="router.push('/')"> Cancel </Button>
            <Button
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

      <!-- Seed Phrase Card -->
      <div v-else>
        <SeedPhraseGrid ref="seedPhraseGridRef" :seed-phrase="seedPhrase" :require-reveal="false" />

        <label class="mt-4 flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            :checked="backedUp"
            :disabled="backedUp"
            @change="onBackedUpChange"
          />
          I have written down these words
        </label>

        <!-- Action Buttons -->
        <div class="flex gap-2 mt-4">
          <Button variant="outline" class="flex-1" @click="router.push('/')"> Close </Button>
        </div>
      </div>
    </div>
  </div>
</template>
