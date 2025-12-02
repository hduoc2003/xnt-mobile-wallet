<script setup lang="ts">
import { inject, computed, type ComputedRef } from 'vue'

interface Props {
    tabKey: string
}

const props = defineProps<Props>()
const activeKey = inject<ComputedRef<string>>('activeTabKey')

const isActive = computed(() => {
    return activeKey?.value === props.tabKey
})
</script>

<template>
    <div v-show="isActive" class="tab-pane">
        <slot />
    </div>
</template>

<style lang="scss" scoped>
.tab-pane {
    height: 100%;
    display: flex;
    flex-direction: column;
}
</style>
