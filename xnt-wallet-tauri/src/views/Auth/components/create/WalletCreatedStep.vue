<script setup lang="ts">
import { useNeptuneWallet } from '@/composables/useNeptuneWallet'
import { message } from 'ant-design-vue'

const props = defineProps<{
    seedPhrase: string[]
    password: string
}>()

const { createKeystore } = useNeptuneWallet()

const emit = defineEmits<{
    accessWallet: []
    createAnother: []
}>()

const handleAccessWallet = async () => {
    try {
        const seedPhraseString = props.seedPhrase.join(' ')
        
        if (!seedPhraseString || !props.password) {
            message.error('Missing seed or password')
            return
        }

        await createKeystore(seedPhraseString, props.password)
        emit('accessWallet')
    } catch (err) {
        message.error('Failed to create keystore')
    }
}

const handleCreateAnother = () => {
    emit('createAnother')
}
</script>

<template>
    <div class="well-done-step">
        <h2 class="done-main">You are done!</h2>
        <p class="done-desc">
            You are now ready to take advantage of all that your wallet has to offer! Access with
            keystore file should only be used in an offline setting.
        </p>
        <div class="center-svg" style="margin: 14px auto 12px auto">
            <img
                src="@/assets/imgs/logo.png"
                alt="XNT Logo"
                style="max-width: 180px; height: auto"
            />
        </div>
        <div class="btn-row">
            <ButtonCommon
                class="done-btn"
                type="primary"
                size="large"
                block
                style="margin-bottom: 0.3em"
                @click="handleAccessWallet"
            >
                Access Wallet
            </ButtonCommon>
            <button class="done-link" type="button" @click="handleCreateAnother">
                Create Another Wallet
            </button>
        </div>
    </div>
</template>

<style lang="scss" scoped>
.well-done-step {
    text-align: center;
    padding: 20px 8px;

    .done-title {
        color: var(--primary-color);
        font-weight: 700;
        letter-spacing: 0.07em;
        margin-bottom: 1px;
    }

    .done-main {
        font-size: 1.36rem;
        font-weight: 700;
        color: var(--text-primary);
        margin-bottom: 2px;
    }

    .done-desc {
        color: var(--text-secondary);
        font-size: 1.11em;
        max-width: 410px;
        margin: 2px auto 15px auto;
    }

    .center-svg {
        display: flex;
        justify-content: center;
    }

    .btn-row {
        display: flex;
        flex-direction: column;
        gap: 11px;
        align-items: center;
        width: 100%;
        max-width: 400px;
        margin: 0 auto 5px auto;
    }

    .done-btn {
        margin-bottom: 0.3em;
    }

    .done-link {
        background: none;
        border: none;
        color: var(--primary-color);
        font-size: 1em;
        text-decoration: underline;
        cursor: pointer;
        margin: 0 auto;
        font-weight: 600;
    }
}
</style>
