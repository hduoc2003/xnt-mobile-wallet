<script setup lang="ts">
import { toast } from 'vue-sonner'
import { useRouter } from 'vue-router'
import { Send, FileDown, Key, ArrowRightLeft } from 'lucide-vue-next'
import { walletApi } from '@/api/tauri/walletApi'
import { useAccountsStore } from '@/stores'

const router = useRouter()
const accountsStore = useAccountsStore()

const handleSend = () => {
  router.push('/wallet/send')
}

const handleBridge = () => {
  router.push('/wallet/bridge')
}

/** Saves the keystore of the active account's seed; that counts as backing it up. */
const handleBackupFile = async () => {
  try {
    const seedId = accountsStore.findAccount(accountsStore.requireActiveId()).seedId
    const result = await walletApi.accountsSaveKeystoreAs(seedId)
    if (!result?.success) {
      if (result?.error === 'User cancelled') return
      throw new Error(result?.error || 'Failed to save keystore file')
    }
    accountsStore.setRegistry(await walletApi.accountsMarkBackedUp(seedId))
    toast.success('Keystore file saved successfully')
  } catch (error) {
    toast.error('Failed to save keystore file', {
      description: error instanceof Error ? error.message : String(error),
    })
  }
}

const handleBackupSeed = () => {
  router.push('/wallet/backup-seed')
}
</script>

<template>
  <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
    <Button size="lg" class="w-full gap-2" @click="handleSend">
      <Send class="size-4" />
      Send
    </Button>

    <Button size="lg" class="w-full gap-2" @click="handleBridge">
      <ArrowRightLeft class="size-4" />
      Bridge to Leviathan
    </Button>

    <Button size="lg" variant="outline" class="w-full gap-2" @click="handleBackupFile">
      <FileDown class="size-4" />
      Backup File
    </Button>

    <Button size="lg" variant="outline" class="w-full gap-2" @click="handleBackupSeed">
      <Key class="size-4" />
      Backup Seed
    </Button>
  </div>
</template>
