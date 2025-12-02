<script setup lang="ts">
import { ConfirmSeedComponent } from '.'

const emit = defineEmits<{
    next: []
    back: []
}>()

const handleNext = () => {
    emit('next')
}

const handleBack = () => {
    emit('back')
}
</script>

<template>
    <div class="confirm-tab">
        <ConfirmSeedComponent @next="handleNext" @back="handleBack" />
    </div>
</template>

<style lang="scss" scoped>
.confirm-tab {
    padding: var(--spacing-lg);
}
</style>
