<script setup lang="ts">
import type { ProgressRootProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import { ProgressIndicator, ProgressRoot } from 'reka-ui'
import { cn } from '@/lib/utils'

const props = withDefaults(
  defineProps<ProgressRootProps & { class?: HTMLAttributes['class']; color?: string }>(),
  {
    modelValue: 0,
    max: 100,
    color: 'bg-primary',
  },
)

const delegatedProps = reactiveOmit(props, 'class')
</script>

<template>
  <ProgressRoot
    v-bind="delegatedProps"
    :class="cn('relative h-2 w-full overflow-hidden rounded-full bg-primary/20', props.class)"
  >
    <ProgressIndicator
      class="h-full w-full flex-1 transition-all"
      :class="props.color"
      :style="`transform: translateX(-${props.max - (props.modelValue ?? 0)}%);`"
    />
  </ProgressRoot>
</template>
