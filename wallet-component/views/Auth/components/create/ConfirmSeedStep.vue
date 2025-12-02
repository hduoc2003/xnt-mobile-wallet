<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { ChevronLeft, Check, CheckCircle2, XCircle } from 'lucide-vue-next'

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
const answeredQuestions = ref<number[]>([])

const generateQuiz = (): {
  position: number
  correctWord: string
  options: string[]
} | null => {
  if (!seedWords.value || seedWords.value.length === 0) return null

  let randomPosition: number
  let attempts = 0
  const maxAttempts = 50

  do {
    randomPosition = Math.floor(Math.random() * seedWords.value.length) + 1
    attempts++
    if (attempts > maxAttempts) return null
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
    correctWord: correctWord as string,
    options: options as string[],
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
    answeredQuestions.value.push(quizData.value!.position)

    if (correctCount.value >= totalQuestions) {
      emit('next')
    } else {
      // Move to next question
      showResult.value = false
      selectedAnswer.value = ''
      const newQuiz = generateQuiz()
      if (newQuiz) quizData.value = newQuiz
    }
  } else {
    setTimeout(() => {
      showResult.value = false
      selectedAnswer.value = ''
    }, 1500)
  }
}

const handleBack = () => {
  emit('back')
}

onMounted(() => {
  const newQuiz = generateQuiz()
  if (newQuiz) {
    quizData.value = newQuiz
  }
})
</script>

<template>
  <div class="space-y-6">
    <!-- Header -->
    <div class="space-y-2 text-center">
      <h1 class="text-2xl font-bold text-foreground">Confirm Recovery Phrase</h1>
      <p class="text-sm text-muted-foreground">Select the correct word for each position</p>
    </div>

    <Separator />

    <!-- Progress -->
    <div class="space-y-3">
      <div class="flex items-center justify-center gap-2">
        <div
          v-for="i in totalQuestions"
          :key="i"
          class="flex h-8 w-8 items-center justify-center rounded-full transition-all"
          :class="
            answeredQuestions.includes(i)
              ? 'bg-green-500 text-white'
              : i === correctCount + 1
                ? 'border-2 border-primary bg-primary/10 text-primary'
                : 'bg-muted text-muted-foreground'
          "
        >
          <CheckCircle2 v-if="answeredQuestions.includes(i)" :size="16" />
          <span v-else class="text-xs font-bold">{{ i }}</span>
        </div>
      </div>
      <p class="text-center text-sm font-semibold">
        Question <span class="text-primary">{{ correctCount + 1 }}</span> of {{ totalQuestions }}
      </p>
    </div>

    <!-- Quiz Section -->
    <div v-if="quizData" class="space-y-5">
      <!-- Question Card -->
      <Card class="border-2 border-primary/30 bg-linear-to-br from-primary/5 to-accent/5">
        <CardContent class="py-8">
          <h2 class="text-center text-xl font-bold text-foreground">
            Select word
            <span class="text-primary">#{{ quizData.position }}</span>
          </h2>
          <p class="mt-2 text-center text-sm text-muted-foreground">
            What is the
            {{
              quizData.position === 1
                ? '1st'
                : quizData.position === 2
                  ? '2nd'
                  : quizData.position === 3
                    ? '3rd'
                    : `${quizData.position}th`
            }}
            word in your recovery phrase?
          </p>
        </CardContent>
      </Card>

      <!-- Answer Options -->
      <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <button
          v-for="(option, index) in quizData.options"
          :key="index"
          class="group relative overflow-hidden rounded-xl border-2 p-5 text-left transition-all disabled:cursor-not-allowed"
          :class="{
            'border-primary bg-primary/10 shadow-lg': selectedAnswer === option && !showResult,
            'border-green-500 bg-green-500/10':
              showResult && isCorrect && option === quizData.correctWord,
            'border-destructive bg-destructive/10':
              showResult && selectedAnswer === option && option !== quizData.correctWord,
            'border-border hover:border-primary/50 hover:bg-accent':
              !selectedAnswer || (selectedAnswer !== option && !showResult),
          }"
          :disabled="showResult"
          @click="handleAnswerSelect(option)"
        >
          <div class="flex items-center gap-3">
            <!-- Option Number -->
            <div
              class="flex h-8 w-8 items-center justify-center rounded-full text-xs font-bold transition-colors"
              :class="{
                'bg-primary text-primary-foreground': selectedAnswer === option && !showResult,
                'bg-green-500 text-white':
                  showResult && isCorrect && option === quizData.correctWord,
                'bg-destructive text-destructive-foreground':
                  showResult && selectedAnswer === option && option !== quizData.correctWord,
                'bg-muted text-muted-foreground':
                  !selectedAnswer || (selectedAnswer !== option && !showResult),
              }"
            >
              {{ String.fromCharCode(65 + index) }}
            </div>

            <!-- Word -->
            <span class="flex-1 text-base font-semibold">{{ option }}</span>

            <!-- Check/X Icon -->
            <CheckCircle2
              v-if="showResult && isCorrect && option === quizData.correctWord"
              :size="24"
              class="text-green-500"
            />
            <XCircle
              v-else-if="showResult && selectedAnswer === option && option !== quizData.correctWord"
              :size="24"
              class="text-destructive"
            />
          </div>
        </button>
      </div>

      <!-- Result Message -->
      <div v-if="showResult" class="animate-in fade-in slide-in-from-top-4 duration-500">
        <Alert :variant="isCorrect ? 'default' : 'destructive'" class="border-2">
          <CheckCircle2 v-if="isCorrect" :size="20" class="text-green-500" />
          <XCircle v-else :size="20" class="text-destructive" />
          <AlertDescription class="text-base font-medium">
            <span v-if="isCorrect && correctCount + 1 >= totalQuestions">
              Perfect! You've verified your recovery phrase. 🎉
            </span>
            <span v-else-if="isCorrect"> Correct! Moving to next question... </span>
            <span v-else> That's not correct. Please try again. </span>
          </AlertDescription>
        </Alert>
      </div>
    </div>

    <!-- Action Buttons -->
    <div class="flex gap-3">
      <Button
        v-if="!showResult || !isCorrect"
        variant="outline"
        size="lg"
        class="flex-1 gap-2"
        @click="handleBack"
      >
        <ChevronLeft :size="18" />
        Back
      </Button>
      <Button
        v-if="showResult && isCorrect"
        size="lg"
        class="flex-1 gap-2 text-base font-semibold"
        @click="handleNext"
      >
        {{ correctCount + 1 >= totalQuestions ? 'Continue' : 'Next Question' }}
        <Check :size="18" />
      </Button>
    </div>
  </div>
</template>
