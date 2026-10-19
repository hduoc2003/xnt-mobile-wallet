<script setup lang="ts">
import { computed, ref } from 'vue'
import { Check, ChevronDown } from 'lucide-vue-next'
import { toast } from 'vue-sonner'
import { useAccounts } from '@/composables/useAccounts'
import { useAccountsStore } from '@/stores'
import { formatXnt } from '@/utils'

const accountsStore = useAccountsStore()
const { switchTo } = useAccounts()
const open = ref(false)

const balanceOf = (id: string) => {
  const balance = accountsStore.balances.get(id)
  return balance === undefined ? '-' : `${formatXnt(balance.available)} XNT`
}

/** Hidden accounts count too (they are still scanned); unknown until every opened one is known. */
const total = computed(() => {
  const balances = accountsStore.keyRefs.map((r) => accountsStore.balances.get(r.accountId))
  if (balances.some((b) => b === undefined)) return '-'
  const sum = balances.reduce((acc, b) => acc + Number(b?.available), 0)
  return `${formatXnt(sum)} XNT`
})

const activeName = computed(() =>
  accountsStore.activeId === undefined ? '' : accountsStore.displayName(accountsStore.activeId),
)

const choose = async (id: string) => {
  try {
    await switchTo(id)
    open.value = false
  } catch (error: unknown) {
    toast.error('Could not switch account', {
      description: error instanceof Error ? error.message : String(error),
    })
  }
}
</script>

<template>
  <Drawer v-if="accountsStore.activeId !== undefined" v-model:open="open">
    <DrawerTrigger as-child>
      <button
        type="button"
        class="flex min-w-0 max-w-[60vw] items-center gap-1 rounded-md px-2 py-1 text-left hover:bg-muted"
        :aria-label="`Switch account, current: ${activeName}`"
      >
        <span class="min-w-0">
          <span class="block truncate text-sm font-semibold text-foreground">{{ activeName }}</span>
          <span class="block truncate text-xs text-muted-foreground">
            {{ balanceOf(accountsStore.activeId) }}
          </span>
        </span>
        <ChevronDown class="size-4 shrink-0 text-muted-foreground" />
      </button>
    </DrawerTrigger>
    <DrawerContent>
      <DrawerHeader>
        <DrawerTitle>Accounts</DrawerTitle>
        <DrawerDescription
          >Locked accounts can't be opened until their seed opens.</DrawerDescription
        >
      </DrawerHeader>
      <div class="max-h-[60vh] space-y-1 overflow-y-auto px-4 pb-6">
        <div class="flex items-center justify-between rounded-md bg-muted px-3 py-2 text-sm">
          <span class="font-medium">All accounts</span>
          <span>{{ total }}</span>
        </div>
        <button
          v-for="account in accountsStore.visibleAccounts"
          :key="account.id"
          type="button"
          class="flex w-full items-center justify-between gap-3 rounded-md px-3 py-2 text-left text-sm hover:bg-muted disabled:opacity-50"
          :disabled="!accountsStore.keys.has(account.id)"
          :aria-current="account.id === accountsStore.activeId ? 'true' : undefined"
          @click="choose(account.id)"
        >
          <span class="flex min-w-0 items-center gap-2">
            <Check
              class="size-4 shrink-0"
              :class="account.id === accountsStore.activeId ? 'text-primary' : 'invisible'"
            />
            <span class="truncate">{{ accountsStore.displayName(account.id) }}</span>
          </span>
          <span class="shrink-0 text-muted-foreground">{{ balanceOf(account.id) }}</span>
        </button>
      </div>
    </DrawerContent>
  </Drawer>
</template>
