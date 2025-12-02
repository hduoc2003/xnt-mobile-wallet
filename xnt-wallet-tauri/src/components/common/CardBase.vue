<script setup lang="ts"></script>

<template>
    <div class="card-base">
        <slot />
    </div>
</template>

<style lang="scss" scoped>
.card-base {
    @include card-base;
}
</style>
