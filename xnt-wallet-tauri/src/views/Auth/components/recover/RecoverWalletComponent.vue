<script setup lang="ts">
import { ref } from 'vue'
import { RecoverSeedComponent } from '..'
import { useNeptuneWallet } from '@/composables/useNeptuneWallet'
import { message } from 'ant-design-vue'

const emit = defineEmits<{
    (e: 'cancel'): void
    (e: 'accessWallet'): void
}>()

const { recoverWalletFromSeed, createKeystore } = useNeptuneWallet()

const recoverSeedComponentRef = ref<InstanceType<typeof RecoverSeedComponent>>()
const showPasswordForm = ref(false)
const seedPhrase = ref<string[]>([])
const isLoading = ref(false)
const isSeedPhraseValid = ref(true)

const handleSeedWordsUpdate = (words: string[]) => {
    seedPhrase.value = words
}

const handleSeedPhraseSubmit = async (words: string[]) => {
    seedPhrase.value = words
    showPasswordForm.value = true
}

const handleContinue = () => {
    recoverSeedComponentRef.value?.handleSubmit?.()
}

const handlePasswordSubmit = async (password: string) => {
    try {
        isLoading.value = true
        const result = await recoverWalletFromSeed(seedPhrase.value)
        if (result.address) {
            await createKeystore(seedPhrase.value.join(' '), password)
            message.success('Loading wallet...')
            emit('accessWallet')
        }
    } catch (err) {
        if (err instanceof Error && err.message.includes('Invalid seed phrase')) {
            isSeedPhraseValid.value = false
        } else {
            message.error('Failed to recover wallet from seed')
        }
    } finally {
        isLoading.value = false
        showPasswordForm.value = false
    }
}

const handlePasswordBack = () => {
    showPasswordForm.value = false
}

const handleCancel = () => {
    emit('cancel')
    showPasswordForm.value = false
    seedPhrase.value = []
    isSeedPhraseValid.value = true
}
</script>
<template>
    <div class="import-wallet dark-card">
        <h2 class="title">Recover Wallet</h2>

        <!-- Seed Phrase Step -->
        <div v-if="!showPasswordForm">
            <div class="desc">Enter your recovery seed phrase</div>
            <RecoverSeedComponent
                ref="recoverSeedComponentRef"
                :valid="isSeedPhraseValid"
                @submit="handleSeedPhraseSubmit"
                @update:words="handleSeedWordsUpdate"
                @update:valid="isSeedPhraseValid = $event"
            />
            <ButtonCommon
                class="mt-20"
                type="primary"
                block
                size="large"
                :disabled="isLoading"
                @click="handleContinue"
            >
                Continue
            </ButtonCommon>
            <ButtonCommon class="mt-10" type="default" block size="large" @click="handleCancel">
                Cancel
            </ButtonCommon>
        </div>

        <!-- Password Step -->
        <div v-else>
            <div class="desc">Enter password to encrypt seed phrase</div>
            <PasswordForm
                button-text="Submit"
                placeholder="Enter password to encrypt seed phrase"
                label="Password"
                :loading="isLoading"
                :validate-format="true"
                @submit="handlePasswordSubmit"
                @back="handlePasswordBack"
            />
        </div>
    </div>
</template>
<style lang="scss" scoped>
.import-wallet {
    max-width: 420px;
    width: 100%;
    margin: 24px auto;
    background: var(--bg-light);
    border-radius: var(--radius-xl);
    box-shadow: var(--shadow-primary);
    padding: 32px 28px 24px 28px;
    color: var(--text-primary);

    .title {
        font-weight: 700;
        font-size: 1.45rem;
        text-align: center;
        margin-bottom: 5px;
    }
    .desc {
        text-align: center;
        color: var(--text-secondary);
        font-size: 1rem;
        margin-bottom: 24px;
    }
}
@include screen(mobile) {
    .import-wallet {
        padding: 20px;
    }
}
</style>
