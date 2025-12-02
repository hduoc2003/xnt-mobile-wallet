<script setup lang="ts">
const config = {
    token: {
        colorPrimary: '#42A5F5',
        borderRadius: 4,
    },
}
</script>

<template>
    <a-config-provider :theme="config" :autoInsertSpaceInButton="false">
        <LayoutVue>
            <router-view />
        </LayoutVue>
    </a-config-provider>
</template>

<style lang="scss" scoped></style>
