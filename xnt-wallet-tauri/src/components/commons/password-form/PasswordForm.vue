<script setup lang="ts">
import { ref, computed } from 'vue'
import { useForm } from '@tanstack/vue-form'
import { z } from 'zod'
import { Eye, EyeOff, Lock, Loader2, ChevronLeft } from 'lucide-vue-next'
import { Card, CardContent } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'

interface Props {
  label?: string
  placeholder?: string
  buttonText?: string
  hideBackButton?: boolean
  backButtonText?: string
  loading?: boolean
  error?: boolean
  errorMessage?: string
  validateFormat?: boolean
  autocomplete?: string
}

const props = withDefaults(defineProps<Props>(), {
  label: 'Password',
  placeholder: 'Enter your password',
  buttonText: 'Submit',
  backButtonText: 'Back',
  hideBackButton: true,
  loading: false,
  error: false,
  errorMessage: 'Incorrect password. Please try again.',
  validateFormat: false,
  autocomplete: 'current-password',
})

const emit = defineEmits<{
  submit: [password: string]
  back: []
}>()

const showPassword = ref(false)

// Password strength calculation
const passwordStrength = computed(() => {
  const password = form.state.values?.password
  if (!password || !props.validateFormat) return { level: 0, text: '', color: '' }

  let strength = 0
  const checks = {
    length: password.length >= 8,
    uppercase: /[A-Z]/.test(password),
    lowercase: /[a-z]/.test(password),
    number: /[0-9]/.test(password),
    special: /[!@#$%^&*(),.?":{}|<>]/.test(password),
  }
  strength = Object.values(checks).filter(Boolean).length

  if (strength <= 2) return { level: 1, text: 'Weak', color: 'hsl(var(--destructive))' }
  if (strength <= 3) return { level: 2, text: 'Medium', color: 'hsl(48 96% 53%)' } // yellow
  if (strength <= 4) return { level: 3, text: 'Good', color: 'hsl(221 83% 53%)' } // blue
  return { level: 4, text: 'Strong', color: 'hsl(142 76% 36%)' } // green
})

// Custom password schema with strength validation
const createPasswordSchema = () => {
  let schema = z.string().min(1, 'Password is required')
  
  if (props.validateFormat) {
    schema = schema
      .min(8, 'Password must be at least 8 characters')
      .refine((val) => {
        const checks = {
          uppercase: /[A-Z]/.test(val),
          lowercase: /[a-z]/.test(val),
          number: /[0-9]/.test(val),
        }
        const strength = Object.values(checks).filter(Boolean).length
        return strength >= 2 // Medium level minimum
      }, 'Password is too weak. Use uppercase, lowercase, and numbers.')
  }
  
  return schema
}

const form = useForm({
  defaultValues: {
    password: '',
  },
  validators: {
    onChange: z.object({
      password: createPasswordSchema(),
    }),
  },
  onSubmit: async ({ value }) => {
    emit('submit', value.password)
  },
})

const handleBack = () => {
  emit('back')
}

function isInvalid(field: any) {
  return field.state.meta.isTouched && !field.state.meta.isValid
}
</script>

<template>
  <form @submit.prevent="form.handleSubmit">
    <form.Field name="password">
      <template #default="{ field }">
        <div class="space-y-4">
          <div class="space-y-2">
            <Label :for="field.name" class="text-base">{{ label }}</Label>
            <div class="relative">
              <Lock
                :size="20"
                class="absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground"
              />
              <Input
                :id="field.name"
                :name="field.name"
                v-model="field.state.value"
                :type="showPassword ? 'text' : 'password'"
                :placeholder="placeholder"
                class="h-12 pl-11 pr-11 text-base"
                :class="{ 'border-destructive': error || isInvalid(field) }"
                :autocomplete="autocomplete"
                @blur="field.handleBlur"
                @input="(e: Event) => {
                  field.handleChange((e.target as HTMLInputElement).value)
                }"
              />
              <div
                class="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground transition-colors hover:text-foreground"
                @click="showPassword = !showPassword"
              >
                <Eye v-if="!showPassword" :size="20" />
                <EyeOff v-else :size="20" />
              </div>
            </div>

            <!-- Error Message -->
            <p v-if="props.error" class="text-sm text-destructive">
              {{ errorMessage }}
            </p>

            <!-- Validation Error from Zod -->
            <p v-else-if="isInvalid(field)" class="text-sm text-destructive">
              {{ field.state.meta.errors?.[0]?.message }}
            </p>

            <!-- Password Strength Indicator -->
            <div v-if="validateFormat && field.state.value" class="space-y-2 pt-1">
              <div class="flex items-center gap-3">
                <div class="h-1 flex-1 overflow-hidden rounded-full bg-muted">
                  <div
                    class="h-full transition-all duration-300 ease-in-out"
                    :style="{
                      width: `${(passwordStrength.level / 4) * 100}%`,
                      backgroundColor: passwordStrength.color,
                    }"
                  ></div>
                </div>
                <span
                  class="min-w-[60px] text-right text-xs font-semibold"
                  :style="{ color: passwordStrength.color }"
                >
                  {{ passwordStrength.text }}
                </span>
              </div>
              <p class="text-xs text-muted-foreground">
                Use at least 8 characters with uppercase, lowercase, and numbers.
              </p>
            </div>
          </div>

          <!-- Buttons -->
          <div class="flex flex-col gap-3">
            <Button
              type="submit"
              size="lg"
              class="h-12 w-full text-base font-semibold"
              :disabled="!field.state.value || loading"
            >
              <Loader2 v-if="loading" :size="20" class="mr-2 animate-spin" />
              {{ buttonText }}
            </Button>

            <Button
              v-if="!hideBackButton"
              type="button"
              variant="outline"
              size="lg"
              class="h-12 w-full text-base"
              @click="handleBack"
            >
              <ChevronLeft :size="20" class="mr-2" />
              {{ backButtonText }}
            </Button>
          </div>
        </div>
      </template>
    </form.Field>
  </form>
</template>

