<script setup lang="ts">
import { computed, nextTick, ref } from 'vue'
import { Eye, EyeOff, Pencil, Plus, Trash2, Users } from 'lucide-vue-next'
import { toast } from 'vue-sonner'
import { useAccounts } from '@/composables/useAccounts'
import { useNeptuneWallet } from '@/composables/useNeptuneWallet'
import { MAIN_SEED, useAccountsStore } from '@/stores'
import type { SeedEntry } from '@/types'
import { IS_MOBILE } from '@/utils'
import ImportSeedDialog from './ImportSeedDialog.vue'

const PHONE_ACCOUNT_LIMIT = 10

const accountsStore = useAccountsStore()
const { createAccount, removeSeed, rename, setHidden } = useAccounts()
const { getBalance } = useNeptuneWallet()

const busy = ref<string>()
const editing = ref<string>()
const draftName = ref('')
/** The title is kept, since the group leaves the registry before the dialog closes. */
const removing = ref<{ seed: SeedEntry; title: string }>()
/** Removing a seed that was never backed up needs this second confirmation. */
const backupConfirmed = ref(false)
const importOpen = ref(false)

const groups = computed(() =>
  (accountsStore.registry?.seeds ?? []).map((seed) => ({
    seed,
    title: accountsStore.seedTitle(seed.id),
    accounts: accountsStore.accounts
      .filter((a) => a.seedId === seed.id)
      .sort((a, b) => a.index - b.index),
    locked: accountsStore.lockedSeeds.get(seed.id),
  })),
)

const showPhoneWarning = computed(
  () => IS_MOBILE && accountsStore.visibleAccounts.length > PHONE_ACCOUNT_LIMIT,
)

const run = async (key: string, action: () => Promise<unknown>, failure: string) => {
  busy.value = key
  try {
    await action()
  } catch (error: unknown) {
    toast.error(failure, { description: error instanceof Error ? error.message : String(error) })
  } finally {
    busy.value = undefined
  }
}

const addAccount = async (seedId: string) => {
  const { scanError } = await createAccount(seedId)
  if (scanError !== undefined) {
    toast.error('Could not scan the new account', { description: scanError })
  }
  await getBalance()
}

const startRename = async (id: string) => {
  editing.value = id
  draftName.value = accountsStore.shortName(id)
  await nextTick()
  const input = document.getElementById(`rename-${id}`)
  if (input instanceof HTMLInputElement) input.select()
}

/** An empty or default name keeps the default "Account N", which follows the account's position. */
const saveRename = (id: string) =>
  run(
    id,
    async () => {
      const name = draftName.value.trim()
      await rename(id, name === '' || name === accountsStore.defaultName(id) ? null : name)
      editing.value = undefined
    },
    'Could not rename the account',
  )

const confirmRemove = () => {
  const seed = removing.value?.seed
  if (!seed || (!seed.backedUp && !backupConfirmed.value)) return
  void run(
    seed.id,
    async () => {
      await removeSeed(seed.id)
      removing.value = undefined
    },
    'Could not remove the seed',
  )
}
</script>

<template>
  <Card class="border-2 border-border/50 shadow-xl">
    <CardHeader class="space-y-1">
      <CardTitle class="flex items-center gap-2 text-lg">
        <Users class="size-5 text-primary" />
        Accounts
      </CardTitle>
    </CardHeader>
    <CardContent class="space-y-5">
      <p v-if="showPhoneWarning" class="text-xs text-amber-600">
        Many accounts make each sync slower on phones.
      </p>

      <div v-for="group in groups" :key="group.seed.id" class="space-y-2">
        <div class="flex flex-wrap items-center gap-2">
          <span class="text-sm font-medium">{{ group.title }}</span>
          <Badge v-if="!group.seed.backedUp" variant="outline" class="text-amber-600">
            Needs backup
          </Badge>
        </div>
        <p v-if="group.locked" class="text-xs text-destructive">Locked: {{ group.locked }}</p>

        <div
          v-for="account in group.accounts"
          :key="account.id"
          class="flex items-center gap-2 rounded-md border px-3 py-2 text-sm"
          :class="account.hidden && 'opacity-60'"
        >
          <template v-if="editing === account.id">
            <Input
              :id="`rename-${account.id}`"
              v-model="draftName"
              class="h-8 flex-1"
              :aria-label="`New name for ${accountsStore.displayName(account.id)}`"
              :placeholder="accountsStore.defaultName(account.id)"
              @keyup.enter="saveRename(account.id)"
              @keyup.escape="editing = undefined"
            />
            <Button size="sm" :disabled="busy === account.id" @click="saveRename(account.id)">
              Save
            </Button>
            <Button size="sm" variant="ghost" @click="editing = undefined">Cancel</Button>
          </template>
          <template v-else>
            <span class="min-w-0 flex-1 truncate">{{ accountsStore.shortName(account.id) }}</span>
            <Badge v-if="account.id === accountsStore.activeId" variant="secondary">Active</Badge>
            <Button
              size="icon"
              variant="ghost"
              class="size-8"
              :aria-label="`Rename ${accountsStore.displayName(account.id)}`"
              @click="startRename(account.id)"
            >
              <Pencil class="size-4" />
            </Button>
            <Button
              size="icon"
              variant="ghost"
              class="size-8"
              :aria-label="`${account.hidden ? 'Show' : 'Hide'} ${accountsStore.displayName(account.id)}`"
              :title="
                account.id === accountsStore.activeId
                  ? 'The active account stays visible'
                  : account.hidden
                    ? 'Show account'
                    : 'Hide account'
              "
              :disabled="busy === account.id || account.id === accountsStore.activeId"
              @click="
                run(
                  account.id,
                  () => setHidden(account.id, !account.hidden),
                  'Could not change the account',
                )
              "
            >
              <Eye v-if="account.hidden" class="size-4" />
              <EyeOff v-else class="size-4" />
            </Button>
          </template>
        </div>

        <div class="flex flex-wrap gap-2">
          <Button
            size="sm"
            variant="outline"
            class="gap-1"
            :disabled="group.locked !== undefined || busy === `add:${group.seed.id}`"
            @click="
              run(
                `add:${group.seed.id}`,
                () => addAccount(group.seed.id),
                'Could not add an account',
              )
            "
          >
            <Plus class="size-4" />
            {{ busy === `add:${group.seed.id}` ? 'Adding...' : 'Add account' }}
          </Button>
          <Button
            v-if="group.seed.id !== MAIN_SEED"
            size="sm"
            variant="outline"
            class="gap-1 text-destructive"
            :disabled="busy === group.seed.id"
            @click="
              () => {
                backupConfirmed = false
                removing = { seed: group.seed, title: group.title }
              }
            "
          >
            <Trash2 class="size-4" />
            Remove from this device
          </Button>
        </div>
      </div>

      <Button variant="outline" class="w-full" @click="importOpen = true">Import seed</Button>
    </CardContent>
  </Card>

  <ImportSeedDialog v-model:open="importOpen" />

  <Dialog
    :open="removing !== undefined"
    @update:open="(o: boolean) => !o && (removing = undefined)"
  >
    <DialogContent>
      <DialogHeader>
        <DialogTitle>Remove from this device</DialogTitle>
        <DialogDescription>
          {{ removing?.title }}: Funds stay on chain. You can reach them again only by importing
          this seed.
        </DialogDescription>
      </DialogHeader>
      <template v-if="removing && !removing.seed.backedUp">
        <p class="text-sm font-medium text-destructive">
          This seed is not backed up. Without its recovery phrase or backup file, its funds are lost
          for good.
        </p>
        <label class="flex items-start gap-2 text-sm">
          <input
            v-model="backupConfirmed"
            type="checkbox"
            class="mt-0.5 size-4 shrink-0 accent-destructive"
          />
          I have this seed's recovery phrase or backup file.
        </label>
      </template>
      <DialogFooter>
        <Button variant="outline" @click="removing = undefined">Cancel</Button>
        <Button
          variant="destructive"
          :disabled="
            removing === undefined ||
            busy === removing.seed.id ||
            (!removing.seed.backedUp && !backupConfirmed)
          "
          @click="confirmRemove"
        >
          Remove from this device
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
