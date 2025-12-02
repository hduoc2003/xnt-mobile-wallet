<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { SeedPhraseDisplayComponent, ConfirmSeedComponent } from '..'
import { CreatePasswordStep, WalletCreatedStep } from '.'
import { useNeptuneWallet } from '@/composables/useNeptuneWallet'
import { message } from 'ant-design-vue'
import { useRouter } from 'vue-router'

const emit = defineEmits<{
    navigateToRecoverWallet: []
}>()

const { generateWallet, clearWallet } = useNeptuneWallet()
const router = useRouter()

const step = ref(1)
const seedPhrase = ref<string[]>([])
const password = ref('')

const handleNavigateToRecoverWallet = () => {
    emit('navigateToRecoverWallet')
}

const handleNextToConfirmSeed = () => {
    step.value = 3
}

const handleNextToWalletCreated = () => {
    step.value = 4
}

const handleNextFromPassword = async (pwd: string) => {
    try {
        const result = await generateWallet()
        seedPhrase.value = result.seed_phrase
        password.value = pwd
        step.value = 2
    } catch (err: any) {
        message.error(err?.message || 'Failed to generate wallet')
    }
}

const handleBackToCreatePassword = () => {
    step.value = 1
}

const handleBackToSeedPhrase = () => {
    step.value = 2
}

const handleAccessWallet = () => {
    router.push({ name: 'home' })
}

function resetAll() {
    step.value = 1
    seedPhrase.value = []
    password.value = ''
    clearWallet()
}
</script>
<template>
    <div class="auth-container">
        <CardBaseScrollable>
            <div class="auth-card">
                <!-- Step 1: Create Password -->
                <CreatePasswordStep
                    v-if="step === 1"
                    @next="handleNextFromPassword"
                    @navigate-to-recover-wallet="handleNavigateToRecoverWallet"
                />

                <!-- Step 2: Recovery Seed -->
                <SeedPhraseDisplayComponent
                    v-else-if="step === 2"
                    :seed-phrase="seedPhrase"
                    @back="handleBackToCreatePassword"
                    @next="handleNextToConfirmSeed"
                />

                <!-- Step 3: Confirm Seed -->
                <ConfirmSeedComponent
                    v-else-if="step === 3"
                    :seed-phrase="seedPhrase"
                    @back="handleBackToSeedPhrase"
                    @next="handleNextToWalletCreated"
                />

                <!-- Step 4: Success -->
                <WalletCreatedStep
                    v-else-if="step === 4"
                    :seed-phrase="seedPhrase"
                    :password="password"
                    @access-wallet="handleAccessWallet"
                    @create-another="resetAll"
                />
            </div>
        </CardBaseScrollable>
    </div>
</template>

<style lang="scss" scoped>
.auth-container {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--spacing-xl);
}

.auth-card {
    max-width: 720px;
    width: 100%;

    @include screen(mobile) {
        max-width: 100%;
        padding: var(--spacing-md);
    }
}
</style>
