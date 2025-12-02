<script setup lang="ts">
import { ref, computed } from 'vue'
import type { BalanceResult } from '@/interface'

interface Props {
    isLoading?: boolean
    availableBalance?: BalanceResult['balance']
}

const props = withDefaults(defineProps<Props>(), {
    isLoading: false,
    availableBalance: '0.00000000',
})

const emit = defineEmits(['cancel', 'send'])

const outputAddresses = ref('')
const outputAmounts = ref('')
const fee = ref('')
const showConfirmModal = ref(false)
const isAddressExpanded = ref(false)

// Validation
const isAddressValid = computed(() => outputAddresses.value.trim().length > 0)
const isAmountValid = computed(() => {
    if (!outputAmounts.value) return false
    const num = parseFloat(outputAmounts.value)
    if (isNaN(num) || num <= 0) return false

    // Check if amount exceeds available balance
    const balance = parseFloat(props.availableBalance ?? '0.00000000')
    if (!isNaN(balance) && num > balance) return false

    return true
})
const isFeeValid = computed(() => {
    if (!fee.value) return false
    const num = parseFloat(fee.value)
    return !isNaN(num) && num >= 0
})

const amountErrorMessage = computed(() => {
    if (!outputAmounts.value) return ''
    const num = parseFloat(outputAmounts.value)
    if (isNaN(num) || num <= 0) return 'Invalid amount'

    const balance = parseFloat(props.availableBalance ?? '0.00000000')
    if (!isNaN(balance) && num > balance) {
        return `Insufficient balance. Available: ${props.availableBalance} XNT`
    }

    return ''
})

const isFormValid = computed(
    () => isAddressValid.value && isAmountValid.value && isFeeValid.value && !props.isLoading
)

// Format decimal for amount and fee
const formatDecimal = (value: string) => {
    if (!value) return ''

    // Remove any non-numeric characters except dot
    let cleaned = value.replace(/[^\d.]/g, '')

    // Ensure only one dot
    const parts = cleaned.split('.')
    if (parts.length > 2) {
        cleaned = parts[0] + '.' + parts.slice(1).join('')
    }

    // If it's a valid number and doesn't have a dot, add decimal point and zeros
    if (cleaned && !cleaned.includes('.') && /^\d+$/.test(cleaned)) {
        cleaned = cleaned + '.0'
    }

    // Limit to 8 decimal places
    if (cleaned.includes('.')) {
        const [integer, decimal] = cleaned.split('.')
        cleaned = integer + '.' + decimal.slice(0, 8)
    }

    return cleaned
}

const handleAmountBlur = () => {
    if (outputAmounts.value) {
        outputAmounts.value = formatDecimal(outputAmounts.value)
    }
}

const handleFeeBlur = () => {
    if (fee.value) {
        fee.value = formatDecimal(fee.value)
    }
}

const handleCancel = () => {
    emit('cancel')
}

const handleSend = () => {
    if (!isFormValid.value) return
    showConfirmModal.value = true
}

const handleConfirm = () => {
    showConfirmModal.value = false
    isAddressExpanded.value = false
    emit('send', {
        outputAddresses: outputAddresses.value.trim(),
        outputAmounts: outputAmounts.value,
        fee: fee.value,
    })
}

const handleCancelConfirm = () => {
    showConfirmModal.value = false
    isAddressExpanded.value = false
}

const toggleAddressExpand = () => {
    isAddressExpanded.value = !isAddressExpanded.value
}

const displayAddress = computed(() => {
    if (!outputAddresses.value) return ''
    if (isAddressExpanded.value) return outputAddresses.value

    const address = outputAddresses.value

    if (address.length <= 40) return address
    return address.slice(0, 20) + '...' + address.slice(-20)
})
</script>

<template>
    <div class="send-transaction-container">
        <div class="send-transaction-form">
            <!-- Recipient Address -->
            <div class="form-group">
                <label class="form-label">
                    Recipient Address <span class="required">*</span>
                </label>
                <textarea
                    v-model="outputAddresses"
                    class="form-textarea"
                    :class="{ invalid: outputAddresses && !isAddressValid }"
                    placeholder="Type the recipient address"
                    rows="3"
                    :disabled="isLoading"
                />
                <span v-if="outputAddresses && !isAddressValid" class="error-message">
                    Address is required
                </span>
            </div>

            <!-- Amount and Priority Fee Row -->
            <div class="amount-row">
                <div class="amount-field">
                    <div class="form-group">
                        <label class="form-label-extra">
                            <span class="form-label"> Amount <span class="required">*</span> </span>
                            <span class="balance-info"
                                >Available: <strong>{{ availableBalance }}</strong> XNT</span
                            >
                        </label>
                        <input
                            v-model="outputAmounts"
                            type="text"
                            class="form-input"
                            :class="{ invalid: outputAmounts && !isAmountValid }"
                            placeholder="0.0"
                            :disabled="isLoading"
                            @blur="handleAmountBlur"
                        />
                        <span v-if="outputAmounts && amountErrorMessage" class="error-message">
                            {{ amountErrorMessage }}
                        </span>
                    </div>
                </div>
                <div class="priority-fee-field">
                    <div class="form-group">
                        <label class="form-label">
                            Priority Fee <span class="required">*</span>
                        </label>
                        <input
                            v-model="fee"
                            type="text"
                            class="form-input"
                            :class="{ invalid: fee && !isFeeValid }"
                            placeholder="0.0"
                            :disabled="isLoading"
                            @blur="handleFeeBlur"
                        />
                        <span v-if="fee && !isFeeValid" class="error-message"> Invalid fee </span>
                    </div>
                </div>
            </div>

            <!-- Action Buttons -->
            <div class="action-buttons">
                <ButtonCommon
                    type="default"
                    size="large"
                    :disabled="isLoading"
                    @click="handleCancel"
                >
                    Cancel
                </ButtonCommon>
                <ButtonCommon
                    type="primary"
                    size="large"
                    :disabled="!isFormValid"
                    @click="handleSend"
                >
                    SEND
                </ButtonCommon>
            </div>
        </div>
    </div>

    <!-- Confirm Modal -->
    <ModalCommon
        v-model:open="showConfirmModal"
        title="Confirm Transaction"
        :footer="false"
        width="500px"
        :mask-closable="false"
        :keyboard="false"
        @cancel="handleCancelConfirm"
    >
        <div class="confirm-modal-content">
            <div class="confirm-section">
                <h4 class="confirm-label">Recipient Address:</h4>
                <div class="confirm-value address-value" :class="{ expanded: isAddressExpanded }">
                    <span class="address-text">{{ displayAddress }}</span>
                    <button
                        v-if="outputAddresses.length > 40"
                        class="toggle-btn"
                        @click="toggleAddressExpand"
                    >
                        {{ isAddressExpanded ? 'Show Less' : 'Show More' }}
                    </button>
                </div>
            </div>

            <div class="confirm-row">
                <div class="confirm-section">
                    <h4 class="confirm-label">Amount:</h4>
                    <div class="confirm-value">{{ outputAmounts }} XNT</div>
                </div>

                <div class="confirm-section">
                    <h4 class="confirm-label">Priority Fee:</h4>
                    <div class="confirm-value">{{ fee }} XNT</div>
                </div>
            </div>

            <div class="confirm-actions">
                <ButtonCommon type="default" size="large" @click="handleCancelConfirm">
                    Cancel
                </ButtonCommon>
                <ButtonCommon type="primary" size="large" @click="handleConfirm">
                    Confirm & Send
                </ButtonCommon>
            </div>
        </div>
    </ModalCommon>
</template>

<style lang="scss" scoped>
.send-transaction-container {
    padding: var(--spacing-xl);
    height: 100%;
    overflow-y: auto;
}

.send-transaction-form {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-lg);
}

.form-group {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xs);
}

.form-label-extra {
    display: flex;
    justify-content: space-between;
    align-items: center;
}

.form-label {
    font-size: var(--font-sm);
    font-weight: var(--font-medium);
    color: var(--text-primary);
    display: flex;
    align-items: center;

    .required {
        color: var(--error-color);
        margin-left: 2px;
    }

    .balance-info {
        font-size: var(--font-xs);
        font-weight: var(--font-regular);
        color: var(--text-secondary);
        background: var(--bg-secondary);
        padding: 2px 8px;
        border-radius: var(--radius-sm);
    }
}

.form-input,
.form-textarea {
    padding: var(--spacing-sm) var(--spacing-md);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    font-size: var(--font-base);
    color: var(--text-primary);
    background: var(--bg-primary);
    transition: all 0.2s ease;

    &:focus {
        outline: none;
        border-color: var(--primary-color);
        box-shadow: 0 0 0 2px rgba(66, 165, 245, 0.1);
    }

    &:disabled {
        background: var(--bg-disabled);
        cursor: not-allowed;
        opacity: 0.6;
    }

    &.invalid {
        border-color: var(--error-color);
    }
}

.form-textarea {
    resize: vertical;
    min-height: 80px;
    font-family: 'Courier New', monospace;
    line-height: 1.5;
    &::placeholder {
        font-family: var(--font-primary);
    }
}

.error-message {
    font-size: var(--font-xs);
    color: var(--error-color);
    margin-top: -4px;
}

.amount-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--spacing-md);
}

.amount-field,
.priority-fee-field {
    flex: 1;
}

.action-buttons {
    display: flex;
    justify-content: flex-end;
    gap: var(--spacing-md);
    margin-top: var(--spacing-lg);
}

@include screen(mobile) {
    .amount-row {
        grid-template-columns: 1fr;
    }
}

.confirm-modal-content {
    padding: var(--spacing-lg);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xl);
}

.confirm-section {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xs);
}

.confirm-label {
    font-size: var(--font-sm);
    font-weight: var(--font-medium);
    color: var(--text-secondary);
    margin: 0;
}

.confirm-value {
    font-size: var(--font-base);
    font-weight: var(--font-semibold);
    color: var(--text-primary);
    padding: var(--spacing-md);
    background: var(--bg-secondary);
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-color);
    word-break: break-all;
}

.address-value {
    font-family: 'Courier New', monospace;
    font-size: var(--font-sm);
    line-height: 1.6;
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
    position: relative;

    .address-text {
        word-break: break-all;
        transition: all 0.3s ease;
    }

    .toggle-btn {
        align-self: flex-end;
        padding: var(--spacing-xs) var(--spacing-sm);
        background: var(--primary-color);
        color: white;
        border: none;
        border-radius: var(--radius-sm);
        font-size: var(--font-xs);
        font-weight: var(--font-medium);
        cursor: pointer;
        transition: all 0.2s ease;
        font-family: var(--font-primary);

        &:hover {
            background: var(--primary-hover);
            transform: translateY(-1px);
        }

        &:active {
            transform: translateY(0);
        }
    }
}

.confirm-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--spacing-md);
}

.confirm-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--spacing-md);
    margin-top: var(--spacing-md);
}

@include screen(mobile) {
    .confirm-row {
        grid-template-columns: 1fr;
    }
}
</style>
