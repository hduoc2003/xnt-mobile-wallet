<script setup lang="ts">
import { computed } from 'vue'
import { message } from 'ant-design-vue'

interface Props {
    seedPhrase: string[]
    backButton?: boolean
    nextButton?: boolean
    backButtonText?: string
    nextButtonText?: string
}

const props = withDefaults(defineProps<Props>(), {
    backButton: true,
    nextButton: true,
    backButtonText: 'BACK',
    nextButtonText: 'NEXT',
})

const emit = defineEmits<{
    next: []
    back: []
}>()

const seedWords = computed(() => props.seedPhrase || [])

const handleNext = () => {
    if (!seedWords.value || seedWords.value.length === 0) {
        message.error('Cannot proceed without seed phrase')
        return
    }
    emit('next')
}

const handleBack = () => {
    emit('back')
}

const handleCopySeed = async () => {
    if (!seedWords.value || seedWords.value.length === 0) {
        message.error('No seed phrase to copy')
        return
    }

    try {
        const seedPhrase = seedWords.value.join(' ')
        await navigator.clipboard.writeText(seedPhrase)
        message.success('Seed phrase copied to clipboard!')
    } catch (err) {
        message.error('Failed to copy seed phrase')
    }
}
</script>

<template>
    <div class="recovery-container">
        <div class="recovery-card">
            <div class="recovery-header">
                <h1 class="recovery-title">Recovery Seed</h1>
            </div>

            <div class="recovery-content">
                <div class="instruction-text">
                    <p>
                        Make sure no one is looking, as anyone with your seed phrase can access your
                        wallet and funds. Write it down and keep it safe.
                    </p>
                </div>

                <div v-if="seedWords.length > 0" class="seed-words-container">
                    <div class="seed-words-grid">
                        <div v-for="(word, index) in seedWords" :key="index" class="seed-word-item">
                            <span class="word-number">{{ index + 1 }}</span>
                            <span class="word-text">{{ word }}</span>
                        </div>
                    </div>
                    <button class="copy-seed-btn" @click="handleCopySeed" type="button">
                        <svg
                            xmlns="http://www.w3.org/2000/svg"
                            width="16"
                            height="16"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        >
                            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                            <path
                                d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"
                            ></path>
                        </svg>
                        Copy Seed Phrase
                    </button>
                </div>
                <div v-else class="no-seed-warning">
                    <p>No seed phrase found. Please go back and generate a wallet first.</p>
                </div>

                <div class="recovery-actions">
                    <ButtonCommon
                        v-if="props.backButton"
                        type="default"
                        size="large"
                        @click="handleBack"
                    >
                        {{ backButtonText }}
                    </ButtonCommon>
                    <ButtonCommon
                        v-if="props.nextButton"
                        type="primary"
                        size="large"
                        @click="handleNext"
                        :disabled="!seedWords || seedWords.length === 0"
                    >
                        {{ nextButtonText }}
                    </ButtonCommon>
                </div>
            </div>
        </div>
    </div>
</template>

<style lang="scss" scoped>
.recovery-container {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-light);
}

.recovery-card {
    max-width: 500px;
    padding: var(--spacing-xl);
    width: 100%;
    border: 2px solid var(--primary-color);
    border-radius: var(--radius-md);
}

.recovery-header {
    text-align: center;
    margin-bottom: var(--spacing-2xl);
    padding-bottom: var(--spacing-xl);
    border-bottom: 1px solid var(--border-color);

    .recovery-title {
        font-size: var(--font-2xl);
        font-weight: var(--font-bold);
        color: var(--text-primary);
        margin: 0;
    }
}

.recovery-content {
    .instruction-text {
        margin-bottom: var(--spacing-2xl);

        p {
            font-size: var(--font-sm);
            color: var(--text-secondary);
            line-height: var(--leading-normal);
            margin-bottom: var(--spacing-md);

            &:last-child {
                margin-bottom: 0;
            }
        }
    }

    .seed-words-container {
        margin-bottom: var(--spacing-2xl);
        padding: var(--spacing-lg);
        background: var(--bg-hover);
        border-radius: var(--radius-md);
        border: 1px solid var(--border-light);

        .seed-words-grid {
            display: grid;
            grid-template-columns: repeat(4, 1fr);
            gap: var(--spacing-md);
            margin-bottom: var(--spacing-md);

            .seed-word-item {
                display: flex;
                align-items: center;
                gap: var(--spacing-sm);
                padding: var(--spacing-sm);
                background: var(--bg-white);
                border-radius: var(--radius-sm);
                border: 1px solid var(--border-light);

                .word-number {
                    font-size: var(--font-xs);
                    font-weight: var(--font-bold);
                    color: var(--text-muted);
                    min-width: 20px;
                }

                .word-text {
                    font-size: var(--font-sm);
                    font-weight: var(--font-medium);
                    color: var(--text-primary);
                }
            }
        }

        .copy-seed-btn {
            display: flex;
            align-items: center;
            gap: var(--spacing-sm);
            width: 100%;
            padding: var(--spacing-sm) var(--spacing-md);
            background: var(--primary-color);
            color: var(--text-light);
            border: none;
            border-radius: var(--radius-sm);
            font-size: var(--font-sm);
            font-weight: var(--font-medium);
            cursor: pointer;
            transition: all 0.2s ease;
            justify-content: center;

            svg {
                flex-shrink: 0;
            }

            &:hover {
                background: var(--primary-hover);
                transform: translateY(-1px);
                box-shadow: 0 2px 8px rgba(0, 127, 207, 0.3);
            }

            &:active {
                transform: translateY(0);
            }
        }
    }

    .cool-fact {
        margin-bottom: var(--spacing-2xl);
        text-align: center;

        p {
            font-size: var(--font-xs);
            color: var(--text-muted);
            font-style: italic;
            margin: 0;
        }
    }

    .recovery-actions {
        display: flex;
        justify-content: space-between;
        gap: var(--spacing-md);
    }

    .no-seed-warning {
        padding: var(--spacing-xl);
        background: var(--error-light);
        border: 2px solid var(--error-color);
        border-radius: var(--radius-md);
        text-align: center;
        margin-bottom: var(--spacing-2xl);

        p {
            font-size: var(--font-md);
            color: var(--error-color);
            font-weight: var(--font-bold);
            margin: 0;
        }
    }
}

@include screen(mobile) {
    .recovery-container {
        padding: var(--spacing-md);
    }

    .recovery-card {
        max-width: 100%;
    }

    .recovery-content {
        .seed-words-container {
            .seed-words-grid {
                grid-template-columns: repeat(2, 1fr);
                gap: var(--spacing-sm);

                .seed-word-item {
                    padding: var(--spacing-xs);

                    .word-number {
                        min-width: 16px;
                    }
                }
            }
        }
    }
}
</style>
