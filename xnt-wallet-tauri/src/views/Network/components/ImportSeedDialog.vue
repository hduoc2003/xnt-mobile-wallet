<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { toast } from 'vue-sonner'
import { useAccounts } from '@/composables/useAccounts'
import { useNeptuneWallet } from '@/composables/useNeptuneWallet'
import type { ImportSource } from '@/types'

type Tab = 'phrase' | 'backupFile' | 'nodeWalletDat'

const TABS: { id: Tab; label: string }[] = [
  { id: 'phrase', label: 'Seed phrase' },
  { id: 'backupFile', label: 'Backup file' },
  { id: 'nodeWalletDat', label: 'Node wallet.dat' },
]

const open = defineModel<boolean>('open', { required: true })
const { importSeed } = useAccounts()
const { getBalance } = useNeptuneWallet()

const tab = ref<Tab>('phrase')
const words = ref('')
const fileContent = ref<string>()
const fileName = ref('')
const filePassword = ref('')
const walletPassword = ref('')
const importing = ref(false)

const reset = () => {
  words.value = ''
  fileContent.value = undefined
  fileName.value = ''
  filePassword.value = ''
  walletPassword.value = ''
}

watch(open, (isOpen) => {
  if (!isOpen) reset()
})

watch(tab, () => {
  fileContent.value = undefined
  fileName.value = ''
})

const pickFile = async (event: Event) => {
  const input = event.target
  if (!(input instanceof HTMLInputElement) || !input.files?.[0]) return
  try {
    fileName.value = input.files[0].name
    fileContent.value = await input.files[0].text()
  } catch (error: unknown) {
    toast.error('Could not read the file', {
      description: error instanceof Error ? error.message : String(error),
    })
  }
}

const source = computed<ImportSource | undefined>(() => {
  if (tab.value === 'phrase') {
    const list = words.value.trim().split(/\s+/).filter(Boolean)
    return list.length > 0 ? { kind: 'phrase', words: list } : undefined
  }
  if (fileContent.value === undefined) return undefined
  if (tab.value === 'nodeWalletDat') return { kind: 'nodeWalletDat', content: fileContent.value }
  return filePassword.value !== ''
    ? { kind: 'backupFile', content: fileContent.value, filePassword: filePassword.value }
    : undefined
})

const submit = async () => {
  if (importing.value || !source.value || walletPassword.value === '') return
  importing.value = true
  try {
    const { discoveryError, scanError } = await importSeed(source.value, walletPassword.value)
    toast.success('Seed imported')
    if (discoveryError !== undefined) {
      toast.error('Could not look for more accounts of this seed', { description: discoveryError })
    }
    if (scanError !== undefined) {
      toast.error('Could not scan the imported accounts', { description: scanError })
    }
    open.value = false
    await getBalance()
  } catch (error: unknown) {
    toast.error('Import failed', {
      description: error instanceof Error ? error.message : String(error),
    })
  } finally {
    importing.value = false
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent class="max-h-[90vh] overflow-y-auto">
      <DialogHeader>
        <DialogTitle>Import seed</DialogTitle>
        <DialogDescription>Adds the seed's used accounts to this wallet.</DialogDescription>
      </DialogHeader>

      <div class="flex overflow-hidden rounded-md border text-sm" role="tablist">
        <button
          v-for="t in TABS"
          :key="t.id"
          type="button"
          role="tab"
          :aria-selected="tab === t.id"
          :class="[
            'flex-1 py-1.5 transition-colors',
            tab === t.id
              ? 'bg-primary text-primary-foreground'
              : 'bg-transparent text-muted-foreground hover:bg-muted',
          ]"
          @click="tab = t.id"
        >
          {{ t.label }}
        </button>
      </div>

      <div class="space-y-4">
        <div v-if="tab === 'phrase'" class="space-y-2">
          <Label for="import-words">Seed phrase</Label>
          <Textarea
            id="import-words"
            v-model="words"
            rows="4"
            class="font-mono text-sm"
            autocapitalize="none"
            autocomplete="off"
            autocorrect="off"
            spellcheck="false"
          />
        </div>

        <template v-else>
          <div class="space-y-2">
            <Label for="import-file">
              {{ tab === 'backupFile' ? 'Backup file' : 'Node wallet.dat' }}
            </Label>
            <Input id="import-file" :key="tab" type="file" @change="pickFile" />
          </div>
          <div v-if="tab === 'backupFile'" class="space-y-2">
            <Label for="import-file-password">Backup file password</Label>
            <Input id="import-file-password" v-model="filePassword" type="password" />
          </div>
        </template>

        <div class="space-y-2">
          <Label for="import-wallet-password">This wallet's password</Label>
          <Input
            id="import-wallet-password"
            v-model="walletPassword"
            type="password"
            @keyup.enter="submit"
          />
        </div>
      </div>

      <DialogFooter>
        <Button variant="outline" @click="open = false">Cancel</Button>
        <Button :disabled="!source || walletPassword === '' || importing" @click="submit">
          {{ importing ? 'Importing...' : 'Import' }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
