<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { useDebounceFn } from '@vueuse/core'
import { ScrollArea } from '@/components/ui/scroll-area'
import { cn } from '@/lib/utils'

interface Props {
  items: any[]
  itemHeight?: number
  threshold?: number
  loading?: boolean
  hasMore?: boolean
  emptyMessage?: string
  maxHeight?: string
  debounceMs?: number
}

const props = withDefaults(defineProps<Props>(), {
  itemHeight: 100,
  threshold: 200,
  loading: false,
  hasMore: true,
  emptyMessage: 'No items found',
  maxHeight: '550px',
  debounceMs: 500,
})

const emit = defineEmits<{
  loadMore: []
}>()

const scrollAreaRef = ref<InstanceType<typeof ScrollArea>>()
const isLoadingMore = ref(false)

const handleScrollInternal = () => {
  console.log('handleScrollInternal')
  if (!scrollAreaRef.value?.viewport || isLoadingMore.value || props.loading || !props.hasMore)
    return

  const viewport = scrollAreaRef.value.viewport as HTMLElement
  const scrollTop = viewport.scrollTop
  const scrollHeight = viewport.scrollHeight
  const clientHeight = viewport.clientHeight
  const distanceToBottom = scrollHeight - scrollTop - clientHeight

  if (distanceToBottom < props.threshold) {
    isLoadingMore.value = true
    emit('loadMore')

    setTimeout(() => {
      isLoadingMore.value = false
    }, 500)
  }
}

const handleScroll = useDebounceFn(handleScrollInternal, props.debounceMs)

onMounted(async () => {
  await nextTick()

  let attempts = 0
  const maxAttempts = 10

  while (attempts < maxAttempts) {
    if (scrollAreaRef.value?.viewport) {
      const viewport = scrollAreaRef.value.viewport as HTMLElement
      viewport.addEventListener('scroll', handleScroll, { passive: true })
      break
    }
    await new Promise((resolve) => setTimeout(resolve, 100))
    attempts++
  }
})

onUnmounted(() => {
  if (scrollAreaRef.value?.viewport) {
    const viewport = scrollAreaRef.value.viewport as HTMLElement
    viewport.removeEventListener('scroll', handleScroll)
  }
})

watch(
  () => props.items,
  async () => {
    await nextTick()
    if (scrollAreaRef.value?.viewport) {
      setTimeout(() => {
        handleScroll()
      }, 100)
    }
  },
)
</script>

<template>
  <ScrollArea
    ref="scrollAreaRef"
    :style="{ maxHeight: props.maxHeight }"
    :class="cn('w-full rounded-md', { 'pointer-events-none': loading })"
  >
    <div>
      <!-- Empty state -->
      <div v-if="items.length === 0" class="flex items-center justify-center py-12 text-center">
        <p class="text-sm text-muted-foreground">{{ emptyMessage }}</p>
      </div>

      <!-- Items slot -->
      <template v-else>
        <slot :items="items" />

        <!-- Loading indicator at bottom -->
        <div v-if="loading || isLoadingMore" class="flex items-center justify-center py-4">
          <div
            class="size-6 animate-spin rounded-full border-2 border-primary border-t-transparent"
          />
        </div>
      </template>
    </div>
  </ScrollArea>
</template>
