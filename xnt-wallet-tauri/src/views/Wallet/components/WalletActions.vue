<script setup lang="ts">
import { Send, FileDown, Key } from 'lucide-vue-next'
import { Button } from '@/components/ui/button'
import { useRouter } from 'vue-router'
import { useNeptuneWallet } from '@/composables/useNeptuneWallet'
import { toast } from 'vue-sonner'

const router = useRouter()
const { saveKeystoreAs } = useNeptuneWallet()

const handleSend = () => {
  router.push('/wallet/send')
}

const handleBackupFile = async () => {
  try {
    await saveKeystoreAs()
    toast.success('Keystore file saved successfully')
  } catch (error) {
    if (error instanceof Error && error.message !== 'User canceled') {
      toast.error('Failed to save keystore file')
    }
  }
}

const handleBackupSeed = () => {
  router.push('/wallet/backup-seed')
}
</script>

<template>
  <div class="grid grid-cols-1 gap-4 sm:grid-cols-3">
    <Button size="lg" class="w-full gap-2" @click="handleSend">
      <Send class="size-4" />
      Send
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

