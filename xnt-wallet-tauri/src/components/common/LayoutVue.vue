<script lang="ts" setup></script>

<template>
    <div class="app-layout">
        <div class="app-layout-body">
            <div class="app-content">
                <slot />
            </div>
        </div>
    </div>
</template>

<style lang="scss" scoped>
.app-layout {
    height: 100vh;
    min-height: 100vh;
    width: 100%;
    display: flex;
    flex-direction: column;
    position: relative;
}

.app-layout-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: auto;
}

.app-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    position: relative;
}
</style>
