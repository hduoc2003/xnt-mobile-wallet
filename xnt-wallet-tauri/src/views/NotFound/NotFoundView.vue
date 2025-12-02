<script setup lang="ts">
import { useI18n } from 'vue3-i18n'

const { t } = useI18n()
</script>

<template>
    <section class="page_404">
        <div class="container">
            <div class="row">
                <div class="col-sm-12">
                    <div class="col-sm-10 col-sm-offset-1 text-center">
                        <div class="four_zero_four_bg">
                            <h1 class="text-center">404</h1>
                        </div>
                        <div class="contain_box_404">
                            <h3 class="h2">
                                {{ t('notFound.title') }}
                            </h3>
                            <router-link to="/auth" class="link_404">
                                {{ t('notFound.back') }}
                            </router-link>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </section>
</template>

<script lang="ts" setup></script>

<style lang="scss" scoped>
.page_404 {
    padding: 40px 0;
    background: var(--vt-c-white);
    height: 100%;
    text-align: center;
}

.page_404 img {
    width: 100%;
}

.four_zero_four_bg {
    background-repeat: no-repeat;
    height: 400px;
    background-position: center;
    display: flex;
    align-items: center;
    justify-content: center;
}

.four_zero_four_bg h1 {
    font-size: 80px;
    margin-top: 50px;
}

.four_zero_four_bg h3 {
    font-size: 80px;
}

.link_404 {
    color: var(--vt-c-white);
    padding: 10px 20px;
    background: var(--vt-c-main);
    margin: 20px 0;
    display: inline-block;
}
.contain_box_404 {
    margin-top: -50px;
}
</style>
