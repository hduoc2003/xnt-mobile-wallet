<script setup lang="ts">
import { ref } from 'vue'

interface Props {
    valid?: boolean
}

const props = withDefaults(defineProps<Props>(), {
    valid: true,
})

const emit = defineEmits<{
    (e: 'submit', words: string[]): void
    (e: 'update:words', words: string[]): void
    (e: 'update:valid', valid: boolean): void
}>()

const seedWords = ref<string[]>(Array.from({ length: 18 }, () => ''))

const inputBoxFocus = (idx: number) => {
    if (idx < 18) {
        document.getElementById('input-' + idx)?.focus()
    }
}

const handleGridInput = (index: number, value: string) => {
    emit('update:valid', true)
    seedWords.value[index] = value
    emit(
        'update:words',
        seedWords.value.filter((w) => w.trim())
    )
}

const handlePaste = (event: ClipboardEvent) => {
    event.preventDefault()
    emit('update:valid', true)
    const pastedData = event.clipboardData?.getData('text') || ''
    const words = pastedData
        .trim()
        .split(/\s+/)
        .filter((w) => w)

    if (words.length === 0) return

    const filledWords = Array.from({ length: 18 }, (_, i) => words[i] || '')
    seedWords.value = filledWords
    emit(
        'update:words',
        words.filter((w) => w.trim())
    )
}

const handleSubmit = () => {
    const words = seedWords.value.filter((w) => w.trim())
    emit('submit', words)
}

defineExpose({
    handleSubmit,
})
</script>

<template>
    <div class="recover-seed-form">
        <div class="recover-seed-row-radio">
            <div class="radio active">18 words</div>
        </div>

        <!-- Individual input grid -->
        <div class="recover-seed-inputs">
            <div class="recover-seed-input-grid">
                <div v-for="(word, i) in seedWords" :key="i" class="seed-box">
                    <input
                        :id="'input-' + i"
                        type="text"
                        inputmode="text"
                        autocapitalize="off"
                        autocomplete="off"
                        spellcheck="false"
                        :value="word"
                        :placeholder="i + 1 + '.'"
                        maxlength="24"
                        @keydown.enter="inputBoxFocus(i + 1)"
                        @input="handleGridInput(i, ($event.target as HTMLInputElement).value)"
                        @paste="handlePaste($event)"
                    />
                </div>
            </div>
        </div>
        <div v-if="!props.valid" class="error-text">Invalid seed phrase</div>
    </div>
</template>

<style lang="scss" scoped>
.recover-seed-form {
    .recover-seed-row-radio {
        display: flex;
        gap: 24px;
        align-items: center;
        margin-bottom: 16px;
        .radio {
            background: var(--bg-secondary);
            color: var(--text-secondary);
            border-radius: 14px;
            font-size: 1.08rem;
            padding: 7px 24px 7px 18px;
            font-weight: 500;
            display: flex;
            align-items: center;
            gap: 5px;
            &.active {
                background: var(--primary-color);
                color: var(--text-light);
            }
            opacity: 1;
        }
    }
    .recover-seed-inputs {
        width: 100%;
    }
    .recover-seed-input-grid {
        display: grid;
        grid-template-columns: repeat(4, 1fr);
        gap: 12px;
    }
    .seed-box input {
        width: 100%;
        padding: 8px 12px;
        border-radius: 9px;
        border: 2px solid var(--border-color);
        background: var(--bg-secondary);
        color: var(--text-secondary);
        outline: none;
        font-size: 15px;
        transition:
            border 0.16s,
            box-shadow 0.16s;
        &:focus {
            border-color: var(--primary-color);
            box-shadow: 0 2px 8px var(--shadow-primary);
        }
        &.error {
            border-color: var(--error-color);
            background: var(--bg-secondary);
            color: var(--text-light);
        }
    }
    .error-text {
        color: var(--error-color);
        font-size: 0.97em;
        margin-top: 8px;
    }
}

@include screen(mobile) {
    .seed-input-grid {
        gap: 8px;
        grid-template-columns: repeat(3, 1fr);
    }
}
</style>
