<script setup lang="ts">
import { ref, computed } from 'vue'
import { z } from 'zod'
import { Eye, EyeOff, Lock, Check, X, Shield, KeyRound, Loader2 } from 'lucide-vue-next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Alert, AlertDescription } from '@/components/ui/alert'

interface Props {
  passwordPlaceholder?: string
  confirmPasswordPlaceholder?: string
  submitButtonText?: string
  loadingText?: string
  loading?: boolean
  showSecurityInfo?: boolean
  formId?: string
}

const props = withDefaults(defineProps<Props>(), {
  passwordPlaceholder: 'Enter your password',
  confirmPasswordPlaceholder: 'Re-enter your password',
  submitButtonText: 'Continue',
  loadingText: '',
  loading: false,
  showSecurityInfo: true,
  formId: 'password-creation-form',
})

const displayButtonText = computed(() => {
  if (props.loading && props.loadingText) {
    return props.loadingText
  }
  return props.submitButtonText
})

const emit = defineEmits<{
  submit: [password: string]
}>()

// Form state
const password = ref('')
const confirmPassword = ref('')
const showPassword = ref(false)
const showConfirmPassword = ref(false)
const passwordTouched = ref(false)
const confirmPasswordTouched = ref(false)

// Validation schema
const passwordSchema = z
  .string()
  .min(8, 'Password must be at least 8 characters')
  .regex(/[A-Z]/, 'Must contain at least one uppercase letter')
  .regex(/[a-z]/, 'Must contain at least one lowercase letter')
  .regex(/[0-9]/, 'Must contain at least one number')

// Validate password
const passwordError = computed(() => {
  if (!passwordTouched.value || !password.value) return ''
  try {
    passwordSchema.parse(password.value)
    return ''
  } catch (error) {
    if (error instanceof z.ZodError) {
      return error.issues[0]?.message || ''
    }
    return ''
  }
})

const passwordStrength = computed(() => {
  if (!password.value) return { level: 0, text: '', color: '', width: '0%' }

  let strength = 0
  const checks = {
    length: password.value.length >= 8,
    uppercase: /[A-Z]/.test(password.value),
    lowercase: /[a-z]/.test(password.value),
    number: /[0-9]/.test(password.value),
    special: /[!@#$%^&*(),.?":{}|<>]/.test(password.value),
  }

  strength = Object.values(checks).filter(Boolean).length

  if (strength <= 2) return { level: 1, text: 'Weak', color: 'bg-destructive', width: '25%' }
  if (strength <= 3) return { level: 2, text: 'Fair', color: 'bg-yellow-500', width: '50%' }
  if (strength <= 4) return { level: 3, text: 'Good', color: 'bg-blue-500', width: '75%' }
  return { level: 4, text: 'Strong', color: 'bg-green-500', width: '100%' }
})

const isPasswordMatch = computed(() => {
  if (!confirmPassword.value) return true
  return password.value === confirmPassword.value
})

const canProceed = computed(() => {
  return (
    password.value.length >= 8 &&
    confirmPassword.value.length >= 8 &&
    isPasswordMatch.value &&
    passwordStrength.value.level >= 2 &&
    !passwordError.value
  )
})

const handleSubmit = () => {
  passwordTouched.value = true
  confirmPasswordTouched.value = true

  if (canProceed.value) {
    emit('submit', password.value)
  }
}

// Expose methods for parent components
defineExpose({
  reset: () => {
    password.value = ''
    confirmPassword.value = ''
    passwordTouched.value = false
    confirmPasswordTouched.value = false
  },
  getPassword: () => password.value,
})
</script>

<template>
  <form :id="formId" @submit.prevent="handleSubmit">
    <div class="space-y-6">
      <!-- Password Field -->
      <div class="space-y-2">
        <Label for="password" class="text-base">Password</Label>
        <div class="relative">
          <Lock
            :size="20"
            class="absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground"
          />
          <Input
            id="password"
            v-model="password"
            :type="showPassword ? 'text' : 'password'"
            :placeholder="passwordPlaceholder"
            class="h-12 pl-11 pr-11 text-base"
            :class="{ 'border-destructive': passwordError }"
            @blur="passwordTouched = true"
          />
          <div
            class="absolute right-3 top-1/2 -translate-y-1/2 cursor-pointer text-muted-foreground transition-colors hover:text-foreground"
            @click="showPassword = !showPassword"
          >
            <Eye v-if="!showPassword" :size="20" />
            <EyeOff v-else :size="20" />
          </div>
        </div>

        <!-- Password Strength -->
        <div v-if="password" class="space-y-2">
          <div class="flex items-center gap-2">
            <div class="h-2 flex-1 overflow-hidden rounded-full bg-muted">
              <div
                class="h-full transition-all duration-300"
                :class="passwordStrength.color"
                :style="{ width: passwordStrength.width }"
              />
            </div>
            <span
              class="text-xs font-semibold"
              :class="`text-${passwordStrength.color.replace('bg-', '')}`"
            >
              {{ passwordStrength.text }}
            </span>
          </div>
        </div>

        <!-- Error Message -->
        <p v-if="passwordError" class="text-sm text-destructive">
          {{ passwordError }}
        </p>
      </div>

      <!-- Confirm Password Field -->
      <div class="space-y-2">
        <Label for="confirmPassword" class="text-base">Confirm Password</Label>
        <div class="relative">
          <KeyRound
            :size="20"
            class="absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground"
          />
          <Input
            id="confirmPassword"
            v-model="confirmPassword"
            :type="showConfirmPassword ? 'text' : 'password'"
            :placeholder="confirmPasswordPlaceholder"
            class="h-12 pl-11 pr-11 text-base"
            :class="{ 'border-destructive': confirmPassword && !isPasswordMatch }"
            @blur="confirmPasswordTouched = true"
          />
          <div
            class="absolute right-3 top-1/2 -translate-y-1/2 cursor-pointer text-muted-foreground transition-colors hover:text-foreground"
            @click="showConfirmPassword = !showConfirmPassword"
          >
            <Eye v-if="!showConfirmPassword" :size="20" />
            <EyeOff v-else :size="20" />
          </div>
        </div>

        <!-- Password Match Indicator -->
        <div v-if="confirmPassword" class="flex items-center gap-2">
          <Check v-if="isPasswordMatch" :size="16" class="text-green-500" />
          <X v-else :size="16" class="text-destructive" />
          <span
            class="text-sm"
            :class="isPasswordMatch ? 'text-green-500' : 'text-destructive'"
          >
            {{ isPasswordMatch ? 'Passwords match' : 'Passwords do not match' }}
          </span>
        </div>
      </div>

      <!-- Security Info -->
      <Alert v-if="showSecurityInfo">
        <Shield :size="16" />
        <AlertDescription class="text-xs">
          Use at least 8 characters with uppercase, lowercase, and numbers for a strong
          password.
        </AlertDescription>
      </Alert>

      <Button
        type="submit"
        size="lg"
        class="h-12 w-full max-w-[375px] text-base font-semibold mx-auto"
        :disabled="!canProceed || loading"
      >
        <Loader2 v-if="loading" :size="20" class="mr-2 animate-spin" />
        {{ displayButtonText }}
      </Button>
    </div>
  </form>
</template>

