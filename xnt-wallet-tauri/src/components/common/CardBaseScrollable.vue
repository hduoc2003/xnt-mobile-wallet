<script setup lang="ts"></script>

<template>
    <div class="card-base scrollable">
        <slot />
    </div>
</template>

<style lang="scss" scoped>
.card-base {
    @include card-base;

    &.scrollable {
        height: 100%;
        max-height: calc(100vh - 100px);
        overflow-y: auto;
    }
}
</style>
