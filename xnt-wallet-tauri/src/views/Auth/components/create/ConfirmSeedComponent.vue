<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { message } from 'ant-design-vue'

interface Props {
    seedPhrase: string[]
}

const props = defineProps<Props>()

const emit = defineEmits<{
    next: []
    back: []
}>()

const seedWords = computed(() => props.seedPhrase || [])
const currentQuestionIndex = ref(0)
const selectedAnswer = ref('')
const isCorrect = ref(false)
const showResult = ref(false)
const correctCount = ref(0)
const totalQuestions = 3
const askedPositions = ref<Set<number>>(new Set())

const generateQuiz = (): {
    position: number
    correctWord: string
    options: string[]
} | null => {
    if (!seedWords.value || seedWords.value.length === 0) {
        message.error('No seed phrase found')
        return null
    }

    let randomPosition: number
    let attempts = 0
    const maxAttempts = 50

    do {
        randomPosition = Math.floor(Math.random() * seedWords.value.length) + 1
        attempts++
        if (attempts > maxAttempts) {
            message.error('Unable to generate new question')
            return null
        }
    } while (askedPositions.value.has(randomPosition))

    currentQuestionIndex.value = randomPosition - 1

    const correctWord = seedWords.value[randomPosition - 1]
    const options = [correctWord]

    const otherWords = seedWords.value.filter((_, index) => index !== randomPosition - 1)

    while (options.length < 4 && otherWords.length > 0) {
        const randomIndex = Math.floor(Math.random() * otherWords.length)
        const randomWord = otherWords[randomIndex]

        if (!options.includes(randomWord)) {
            options.push(randomWord)
            otherWords.splice(randomIndex, 1)
        }
    }

    options.sort(() => Math.random() - 0.5)

    return {
        position: randomPosition,
        correctWord,
        options,
    }
}

const quizData = ref<{
    position: number
    correctWord: string
    options: string[]
} | null>(null)

const handleAnswerSelect = (answer: string) => {
    selectedAnswer.value = answer
    isCorrect.value = answer === quizData.value?.correctWord
    showResult.value = true
}

const handleNext = () => {
    if (isCorrect.value) {
        correctCount.value++
        askedPositions.value.add(quizData.value!.position)

        if (correctCount.value >= totalQuestions) {
            emit('next')
        } else {
            showResult.value = false
            selectedAnswer.value = ''
            const newQuiz = generateQuiz()
            if (newQuiz) {
                quizData.value = newQuiz
            }
        }
    } else {
        showResult.value = false
        selectedAnswer.value = ''
        const newQuiz = generateQuiz()
        if (newQuiz) {
            quizData.value = newQuiz
        }
    }
}

const handleBack = () => {
    emit('back')
}

onMounted(() => {
    const seeds = props.seedPhrase

    if (!seeds || seeds.length === 0) {
        message.warning('No seed phrase found. Please go back and generate a wallet first.')
        return
    }

    const newQuiz = generateQuiz()
    if (newQuiz) {
        quizData.value = newQuiz
    }
})
</script>

<template>
    <div class="confirm-container">
        <div class="confirm-card">
            <div class="confirm-header">
                <h1 class="confirm-title">Recovery Seed</h1>
            </div>

            <div class="confirm-content">
                <div class="progress-indicators">
                    <div class="progress-circle"></div>
                    <div class="progress-circle active"></div>
                    <div class="progress-circle"></div>
                </div>

                <div class="instruction-text">
                    <p>
                        Make sure you wrote the phrase down correctly by answering this quick
                        checkup.
                    </p>
                    <p class="progress-text">
                        Question {{ correctCount + 1 }} / {{ totalQuestions }}
                    </p>
                </div>

                <div class="quiz-section">
                    <h2 class="quiz-question">What is the {{ quizData?.position }}th word?</h2>

                    <div class="answer-options">
                        <button
                            v-for="(option, index) in quizData?.options"
                            :key="index"
                            class="answer-button"
                            :class="{
                                selected: selectedAnswer === option,
                                correct: showResult && option === quizData?.correctWord,
                                incorrect:
                                    showResult &&
                                    selectedAnswer === option &&
                                    option !== quizData?.correctWord,
                            }"
                            @click="handleAnswerSelect(option)"
                            :disabled="showResult"
                        >
                            {{ option }}
                        </button>
                    </div>

                    <div v-if="showResult" class="result-message">
                        <p
                            v-if="isCorrect && correctCount + 1 >= totalQuestions"
                            class="success-message"
                        >
                            ✓ Correct! You answered all {{ totalQuestions }} questions correctly.
                        </p>
                        <p v-else-if="isCorrect" class="success-message">
                            ✓ Correct! Next question...
                        </p>
                        <p v-else class="error-message">✗ Incorrect. Please try again.</p>
                    </div>
                </div>

                <div class="confirm-actions">
                    <ButtonCommon
                        v-if="
                            !showResult ||
                            !isCorrect ||
                            (isCorrect && correctCount + 1 < totalQuestions)
                        "
                        type="default"
                        size="large"
                        @click="handleBack"
                    >
                        BACK
                    </ButtonCommon>
                    <ButtonCommon
                        v-if="showResult && isCorrect && correctCount + 1 < totalQuestions"
                        type="primary"
                        size="large"
                        @click="handleNext"
                    >
                        NEXT QUESTION
                    </ButtonCommon>
                    <ButtonCommon
                        v-if="showResult && isCorrect && correctCount + 1 >= totalQuestions"
                        type="primary"
                        size="large"
                        @click="handleNext"
                    >
                        CONTINUE
                    </ButtonCommon>
                </div>
            </div>
        </div>
    </div>
</template>

<style lang="scss" scoped>
.confirm-container {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-light);
}

.confirm-card {
    max-width: 500px;
    width: 100%;
    padding: var(--spacing-xl);
    border: 2px solid var(--primary-color);
    border-radius: var(--radius-md);
}

.confirm-header {
    text-align: center;
    margin-bottom: var(--spacing-2xl);
    padding-bottom: var(--spacing-xl);
    border-bottom: 1px solid var(--border-color);

    .confirm-title {
        font-size: var(--font-2xl);
        font-weight: var(--font-bold);
        color: var(--text-primary);
        margin: 0;
    }
}

.confirm-content {
    .progress-indicators {
        display: flex;
        justify-content: center;
        gap: var(--spacing-sm);
        margin-bottom: var(--spacing-2xl);

        .progress-circle {
            width: 12px;
            height: 12px;
            border-radius: 50%;
            background: var(--border-light);
            transition: background-color 0.3s ease;

            &.active {
                background: var(--primary-color);
            }
        }
    }

    .instruction-text {
        text-align: center;
        margin-bottom: var(--spacing-2xl);

        p {
            font-size: var(--font-sm);
            color: var(--text-secondary);
            line-height: var(--leading-normal);
            margin: 0;
        }

        .progress-text {
            margin-top: var(--spacing-md);
            font-weight: var(--font-bold);
            color: var(--primary-color);
            font-size: var(--font-base);
        }
    }

    .quiz-section {
        margin-bottom: var(--spacing-2xl);

        .quiz-question {
            font-size: var(--font-lg);
            font-weight: var(--font-bold);
            color: var(--text-primary);
            text-align: center;
            margin-bottom: var(--spacing-xl);
        }

        .answer-options {
            display: grid;
            grid-template-columns: repeat(2, 1fr);
            gap: var(--spacing-md);
            margin-bottom: var(--spacing-lg);

            .answer-button {
                padding: var(--spacing-md);
                border: 2px solid var(--border-light);
                background: var(--bg-white);
                border-radius: var(--radius-md);
                font-size: var(--font-sm);
                font-weight: var(--font-medium);
                color: var(--text-primary);
                cursor: pointer;
                transition: all 0.3s ease;

                &:hover:not(:disabled) {
                    border-color: var(--primary-color);
                    background: var(--primary-light);
                }

                &.selected {
                    border-color: var(--primary-color);
                    background: var(--primary-light);
                }

                &.correct {
                    border-color: var(--success-color);
                    background: var(--success-light);
                    color: var(--success-color);
                }

                &.incorrect {
                    border-color: var(--error-color);
                    background: var(--error-light);
                    color: var(--error-color);
                }

                &:disabled {
                    cursor: not-allowed;
                    opacity: 0.7;
                }
            }
        }

        .result-message {
            text-align: center;

            .success-message {
                color: var(--success-color);
                font-weight: var(--font-medium);
                margin: 0;
            }

            .error-message {
                color: var(--error-color);
                font-weight: var(--font-medium);
                margin: 0;
            }
        }
    }

    .confirm-actions {
        display: flex;
        justify-content: space-between;
        gap: var(--spacing-md);

        &:has(:only-child) {
            justify-content: flex-end;
        }
    }
}

@include screen(mobile) {
    .confirm-container {
        padding: var(--spacing-md);
    }

    .confirm-card {
        max-width: 100%;
    }

    .confirm-content {
        .quiz-section {
            .answer-options {
                grid-template-columns: 1fr;
            }
        }

        .confirm-actions {
            flex-direction: column;
        }
    }
}
</style>
