<script setup lang="ts">
import { computed, watch, onMounted, onUnmounted } from 'vue'

interface Props {
    open: boolean
    title?: string
    width?: string | number
    maskClosable?: boolean
    keyboard?: boolean
    footer?: boolean
    disabled?: boolean
    loading?: boolean
    getContainer?: () => HTMLElement
}

const props = withDefaults(defineProps<Props>(), {
    title: '',
    width: '520px',
    maskClosable: true,
    keyboard: true,
    footer: true,
    disabled: false,
    loading: false,
})

const emit = defineEmits<{
    'update:open': [value: boolean]
    cancel: []
    ok: []
}>()

const handleClose = () => {
    if (!props.maskClosable) return
    emit('update:open', false)
    emit('cancel')
}

const handleCancel = () => {
    emit('update:open', false)
    emit('cancel')
}

const handleOk = () => {
    emit('ok')
}

const handleMaskClick = () => {
    if (props.maskClosable) {
        handleClose()
    }
}

const handleContentClick = (e: Event) => {
    e.stopPropagation()
}

const handleKeydown = (e: KeyboardEvent) => {
    if (props.keyboard && e.key === 'Escape' && props.open) {
        handleClose()
    }
}

const modalWidth = computed(() => {
    if (typeof props.width === 'number') {
        return `${props.width}px`
    }
    return props.width
})

watch(
    () => props.open,
    (isOpen) => {
        if (isOpen) {
            document.body.style.overflow = 'hidden'
        } else {
            document.body.style.overflow = ''
        }
    }
)

onMounted(() => {
    document.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
    document.removeEventListener('keydown', handleKeydown)
    document.body.style.overflow = ''
})
</script>

<template>
    <Teleport :to="getContainer ? getContainer() : 'body'">
        <Transition name="modal-fade">
            <div v-if="open" class="modal-mask" @click="handleMaskClick">
                <Transition name="modal-slide">
                    <div
                        v-if="open"
                        class="modal-wrapper"
                        :style="{ width: modalWidth }"
                        @click="handleContentClick"
                    >
                        <div class="modal-header" v-if="title">
                            <h3 class="modal-title">{{ title }}</h3>
                            <button
                                class="modal-close-btn"
                                @click="handleCancel"
                                aria-label="Close"
                            >
                                <svg
                                    xmlns="http://www.w3.org/2000/svg"
                                    width="20"
                                    height="20"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="2"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                >
                                    <line x1="18" y1="6" x2="6" y2="18"></line>
                                    <line x1="6" y1="6" x2="18" y2="18"></line>
                                </svg>
                            </button>
                        </div>
                        <div class="modal-body">
                            <slot />
                        </div>
                        <div v-if="footer" class="modal-footer">
                            <slot name="footer">
                                <button class="modal-btn modal-btn-default" @click="handleCancel">
                                    Cancel
                                </button>
                                <button
                                    class="modal-btn modal-btn-primary"
                                    @click="handleOk"
                                    :disabled="disabled"
                                >
                                    <SpinnerCommon v-if="loading" size="small" />
                                    <span v-else>OK</span>
                                </button>
                            </slot>
                        </div>
                    </div>
                </Transition>
            </div>
        </Transition>
    </Teleport>
</template>

<style lang="scss" scoped>
.modal-mask {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    padding: var(--spacing-lg);
}

.modal-wrapper {
    background: var(--bg-white);
    border-radius: var(--radius-lg);
    box-shadow: 0 3px 20px rgba(0, 0, 0, 0.2);
    max-width: 100%;
    max-height: 90vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
}

.modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--spacing-lg) var(--spacing-xl);
    border-bottom: 1px solid var(--border-color);
    flex-shrink: 0;

    .modal-title {
        margin: 0;
        font-size: var(--font-xl);
        font-weight: var(--font-bold);
        color: var(--text-primary);
    }

    .modal-close-btn {
        display: flex;
        align-items: center;
        justify-content: center;
        width: 32px;
        height: 32px;
        border: none;
        background: transparent;
        cursor: pointer;
        border-radius: var(--radius-sm);
        color: var(--text-secondary);
        transition: var(--transition-all);

        &:hover {
            background: var(--bg-hover);
            color: var(--text-primary);
        }
    }
}

.modal-body {
    padding: var(--spacing-xl);
    overflow-y: auto;
    flex: 1;
}

.modal-footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: var(--spacing-md);
    padding: var(--spacing-lg) var(--spacing-xl);
    border-top: 1px solid var(--border-color);
    flex-shrink: 0;

    .modal-btn {
        padding: 8px 16px;
        border-radius: var(--radius-sm);
        font-size: var(--font-base);
        font-weight: var(--font-medium);
        cursor: pointer;
        transition: var(--transition-all);
        border: 1px solid transparent;

        &.modal-btn-default {
            background: var(--bg-white);
            border-color: var(--border-color);
            color: var(--text-primary);

            &:hover {
                background: var(--bg-hover);
                border-color: var(--primary-color);
                color: var(--primary-color);
            }
        }

        &.modal-btn-primary {
            background: var(--primary-color);
            border-color: var(--primary-color);
            color: var(--text-light);

            &:hover {
                background: var(--primary-hover);
                border-color: var(--primary-hover);
            }
        }
    }
}

// Transitions
.modal-fade-enter-active,
.modal-fade-leave-active {
    transition: opacity 0.3s ease;
}

.modal-fade-enter-from,
.modal-fade-leave-to {
    opacity: 0;
}

.modal-slide-enter-active,
.modal-slide-leave-active {
    transition: all 0.3s ease;
}

.modal-slide-enter-from {
    opacity: 0;
    transform: translateY(-50px) scale(0.95);
}

.modal-slide-leave-to {
    opacity: 0;
    transform: translateY(50px) scale(0.95);
}

@media (max-width: 767px) {
    .modal-mask {
        padding: var(--spacing-sm);
    }

    .modal-wrapper {
        width: 90% !important;
        max-width: 90% !important;
    }

    .modal-header,
    .modal-body,
    .modal-footer {
        padding: var(--spacing-md);
    }
}
</style>
