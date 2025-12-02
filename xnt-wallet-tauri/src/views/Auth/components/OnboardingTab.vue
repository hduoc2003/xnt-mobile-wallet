<script setup lang="ts">
const emit = defineEmits<{
    goToCreate: []
    goToRecover: []
}>()

const handleGoToCreate = () => {
    emit('goToCreate')
}

const handleRecover = () => {
    emit('goToRecover')
}
</script>

<template>
    <div class="welcome-page flex-center" style="min-height: 100%; width: 100%; display: flex; align-items: center; justify-content: center; padding: 20px;">
        <div class="welcome-card flex-center" style="background-color: rgba(255, 255, 255, 0.1); backdrop-filter: blur(10px); box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3); border-radius: 12px; width: 100%; max-width: 500px; display: flex; flex-direction: column; text-align: center; padding: 2rem; border: 1px solid rgba(255, 255, 255, 0.1);">
            <div class="welcome-box" style="width: 100%; max-width: 700px; padding: 30px;">
                <div class="header-section" style="margin-bottom: 2rem;">
                    <h2 style="color: #ffffff; font-size: 1.5rem; margin-bottom: 1rem; text-align: center; font-weight: 700;">Welcome to XNT Wallet</h2>
                    <p style="color: rgba(255, 255, 255, 0.8); margin: 1rem; font-size: 0.9rem;">Choose the next action:</p>
                </div>
                <div
                    class="button-group"
                    style="display: flex; flex-direction: column; gap: 1rem; margin: 2rem 0"
                >
                    <ButtonCommon type="primary" size="large" @click="handleGoToCreate">
                        Create new wallet
                    </ButtonCommon>
                    <ButtonCommon type="default" size="large" @click="handleRecover">
                        Recover wallet
                    </ButtonCommon>
                </div>
                <div class="note" style="margin-top: 1rem; font-size: 0.9rem; color: rgba(255, 255, 255, 0.6);">Thank you for being a part of the XNT community!</div>
            </div>
        </div>
    </div>
</template>

<style lang="scss" scoped>
.welcome-page {
    min-height: 100vh;
    position: relative;

    .welcome-card {
        background-color: var(--bg-white);
        box-shadow: var(--shadow-md);
        border-radius: var(--radius-md);
        width: 100%;
        max-width: 500px;
        flex-direction: column;
        text-align: center;
        padding: 2rem;
        .welcome-box {
            width: 100%;
            max-width: 700px;
            padding: 30px;
        }
    }
}

p {
    margin: 1rem;
    font-size: var(--font-sm);
}

.note {
    margin-top: 1rem;
    font-size: 0.9rem;
    color: var(--text-secondary);
}
</style>
