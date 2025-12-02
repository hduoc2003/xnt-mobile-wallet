<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, inject, watch, type ComputedRef } from 'vue'
import { useNeptuneStore } from '@/stores/neptuneStore'
import { useNeptuneWallet } from '@/composables/useNeptuneWallet'
import { message } from 'ant-design-vue'
import SeedPhraseDisplayComponent from '@/views/Auth/components/SeedPhraseDisplayComponent.vue'
import SendTransactionComponent from './SendTransactionComponent.vue'
import { WalletAddress, WalletBalance } from '.'
import type { BalanceResult, Utxo } from '@/interface'

const neptuneStore = useNeptuneStore()
const {
    getBalance,
    saveKeystoreAs,
    buildTransaction,
    broadcastSignedTransaction,
    decryptKeystore,
} = useNeptuneWallet()
const activeTabKey = inject<ComputedRef<string>>('activeTabKey')

const availableBalance = ref<BalanceResult['balance']>('0.00000000')
const pendingBalance = ref< BalanceResult['pendingBalance']>('0.00000000')
const loading = ref(false)
const sendingLoading = ref(false)
const error = ref<string | null>(null)
const showSeedModal = ref(false)
const isSendingMode = ref(false)
const isVerifyingPassword = ref(false)
const passwordError = ref(false)
const isVerifying = ref(false)
const seedPhrase = ref<string[]>([])

const receiveAddress = computed(() => neptuneStore.getWallet?.address || '')

const walletStatus = computed(() => {
    if (loading.value) return 'Loading...'
    if (error.value) return 'Error'
    if (neptuneStore.getWallet?.address) return 'Online'
    return 'Offline'
})

const windowWidth = ref(window.innerWidth)
const modalWidth = computed(() => {
    if (windowWidth.value <= 767) {
        return '90%'
    }
    return '60%'
})

const handleResize = () => {
    windowWidth.value = window.innerWidth
}

const handleClickSendButton = () => {
    isSendingMode.value = true
}

const handleCancelSend = () => {
    isSendingMode.value = false
}

const handleSendTransaction = async (data: {
    outputAddresses: string
    outputAmounts: string
    fee: string
}) => {
    try {
        sendingLoading.value = true
        const payload = {
            spendingKeyHex: neptuneStore.getSpendingKey || '',
            inputAdditionRecords: neptuneStore.getUtxos?.map((utxo: Utxo) => utxo.additionRecord),
            outputAddresses: [data.outputAddresses],
            outputAmounts: [data.outputAmounts],
            fee: data.fee,
        }

        const result = await buildTransaction(payload)
        if (!result.success) {
            const errorMessage = result.error || 'Failed to build transaction'
            await (window as any).logger.error(`[SendTx] Build transaction failed: ${errorMessage}`)
            message.error(errorMessage)
            return
        }

        const broadcastResult = await broadcastSignedTransaction(result.transaction)
        if (!broadcastResult.success) {
            const errorMessage = broadcastResult.error || 'Failed to broadcast transaction'
            await (window as any).logger.error(`[SendTx] Broadcast failed: ${errorMessage}`)
            message.error(errorMessage)
            return
        }
        message.success('Transaction sent successfully!')
        handleCancelSend()
        await loadWalletData()
    } catch (error: any) {
        const errorMessage = error?.message || error?.error || 'Failed to send transaction'
        await (window as any).logger.error(`[SendTx] Unexpected error: ${errorMessage}`)
        message.error(errorMessage)
    } finally {
        sendingLoading.value = false
    }
}

const handleBackupFile = async () => {
    try {
        await saveKeystoreAs()
        message.success('Keystore saved successfully')
    } catch (err) {
        if (err instanceof Error && err.message.includes('User canceled')) return
        message.error('Failed to save keystore')
    }
}

const handleBackupSeed = () => {
    isVerifyingPassword.value = true
}

const handleCancelVerify = () => {
    isVerifyingPassword.value = false
    passwordError.value = false
}

const handlePasswordVerify = async (password: string) => {
    try {
        isVerifying.value = true
        passwordError.value = false

        const result = await decryptKeystore(password)
        seedPhrase.value = result.seedPhrase

        isVerifyingPassword.value = false
        showSeedModal.value = true
    } catch (err) {
        passwordError.value = true
    } finally {
        isVerifying.value = false
    }
}

const handleCloseModal = () => {
    showSeedModal.value = false
}

const loadWalletData = async () => {
    const receiveAddress = neptuneStore.getWallet?.address || ''
    if (!receiveAddress) return

    loading.value = true
    error.value = null
    try {
        const result = await getBalance()

        availableBalance.value = result?.balance ?? '0.00000000'
        pendingBalance.value = result?.pendingBalance ?? '0.00000000'
    } catch (error) {
        message.error('Failed to load wallet data')
    } finally {
        loading.value = false
    }
}

onMounted(() => {
    loadWalletData()
    window.addEventListener('resize', handleResize)
})

onUnmounted(() => {
    window.removeEventListener('resize', handleResize)
})

watch(
    () => activeTabKey?.value,
    (newTab) => {
        if (newTab === 'WALLET') loadWalletData()
    }
)
</script>

<template>
    <CardBaseScrollable class="wallet-info-container">
        <!-- Normal Wallet View -->
        <div v-if="!isSendingMode && !isVerifyingPassword" class="wallet-content">
            <div class="balance-wrapper">
                <WalletBalance
                    :is-loading-data="loading"
                    :available-balance="availableBalance"
                    :pending-balance="pendingBalance"
                />
            </div>

            <div v-if="receiveAddress" class="address-actions-row">
                <div class="address-wrapper">
                    <WalletAddress />
                </div>
                <div class="action-buttons">
                    <ButtonCommon
                        type="primary"
                        size="large"
                        block
                        @click="handleClickSendButton"
                        class="btn-send"
                    >
                        SEND
                    </ButtonCommon>
                    <ButtonCommon type="primary" size="large" block @click="handleBackupFile">
                        Backup File
                    </ButtonCommon>
                    <ButtonCommon type="primary" size="large" block @click="handleBackupSeed">
                        Backup Seed
                    </ButtonCommon>
                </div>
            </div>
        </div>

        <!-- Password Verification View -->
        <div v-else-if="isVerifyingPassword" class="password-verify-wrapper">
            <div class="password-verify-content">
                <div class="verify-header">
                    <h2 class="verify-title">Verify Password</h2>
                    <p class="verify-subtitle">Enter your password to view seed phrase</p>
                </div>
                <PasswordForm
                    button-text="Verify"
                    placeholder="Enter your password"
                    back-button-text="Cancel"
                    label="Password"
                    :loading="isVerifying"
                    :error="passwordError"
                    error-message="Invalid password"
                    @submit="handlePasswordVerify"
                    @back="handleCancelVerify"
                />
            </div>
        </div>

        <!-- Send Transaction View -->
        <div v-else-if="isSendingMode" class="send-transaction-wrapper">
            <SendTransactionComponent
                :is-loading="sendingLoading"
                :available-balance="availableBalance"
                @cancel="handleCancelSend"
                @send="handleSendTransaction"
            />

            <!-- Loading Overlay -->
            <div v-if="sendingLoading" class="sending-overlay">
                <div class="sending-content">
                    <SpinnerCommon size="large" />
                    <p class="sending-text">Sending...</p>
                </div>
            </div>
        </div>

        <div v-if="receiveAddress && !isSendingMode && !isVerifyingPassword" class="wallet-status">
            <span
                >Wallet Status: <strong>{{ walletStatus }}</strong></span
            >
        </div>
    </CardBaseScrollable>
    <ModalCommon
        v-model:open="showSeedModal"
        title="Backup Seed Phrase"
        :footer="false"
        :width="modalWidth"
        :mask-closable="false"
        :keyboard="false"
        @cancel="handleCloseModal"
    >
        <div class="seed-modal-content">
            <SeedPhraseDisplayComponent :seed-phrase="seedPhrase" :back-button="false" :next-button="false" />
        </div>
    </ModalCommon>
</template>

<style lang="scss" scoped>
.wallet-info-container {
    height: 100%;
}

.wallet-content {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xl);
    flex: 1;
    min-height: 0;
}

.balance-wrapper {
    flex-shrink: 0;
}

.address-actions-row {
    display: flex;
    gap: var(--spacing-xl);
    align-items: center;
    flex: 1;
    min-height: 0;

    .address-wrapper {
        flex: 1;
        min-width: 0;
    }
}

.action-buttons {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-md);
    flex-shrink: 0;
    min-width: 200px;

    :deep(.btn-send) {
        letter-spacing: var(--tracking-wide);
    }
}

.wallet-status {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
    padding: 1rem;
    background: var(--bg-light);
    border-radius: var(--radius-md);
    font-size: var(--font-base);
    color: var(--text-secondary);
    margin-top: auto;
    flex-shrink: 0;

    strong {
        color: var(--text-primary);
        font-weight: var(--font-semibold);
    }
}

.seed-modal-content {
    :deep(.recovery-container) {
        padding: 0;
        min-height: auto;
        background: transparent;
    }

    :deep(.recovery-card) {
        border: none;
        box-shadow: none;
        padding: 0;
    }
}

.password-verify-wrapper {
    display: flex;
    align-items: center;
    justify-content: center;
    flex: 1;
    min-height: 0;
    padding: var(--spacing-xl);
}

.password-verify-content {
    width: 100%;
    max-width: 480px;
}

.verify-header {
    text-align: center;
    margin-bottom: var(--spacing-2xl);

    .verify-title {
        font-size: var(--font-2xl);
        font-weight: var(--font-bold);
        color: var(--text-primary);
        margin: 0 0 var(--spacing-xs) 0;
    }

    .verify-subtitle {
        font-size: var(--font-sm);
        color: var(--text-secondary);
        margin: 0;
    }
}

.send-transaction-wrapper {
    position: relative;
    flex: 1;
    min-height: 0;
}

.sending-overlay {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(2px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 10;
    border-radius: var(--radius-md);

    .sending-content {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: var(--spacing-lg);
        padding: var(--spacing-2xl);
        background: rgba(255, 255, 255, 0.95);
        border-radius: var(--radius-lg);
        box-shadow: var(--shadow-lg);

        .sending-text {
            margin: 0;
            font-size: var(--font-lg);
            font-weight: var(--font-semibold);
            color: var(--text-primary);
        }
    }
}
</style>
