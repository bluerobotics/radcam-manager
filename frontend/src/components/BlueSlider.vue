<template>
  <div class="flex w-full justify-between items-center">
    <div
      v-if="label"
      class="min-w-[130px]"
    >
      <label
        class="text-start mr-6"
        :class="theme === 'dark' ? 'text-white' : 'text-black'"
      >{{ label }}</label>
    </div>
    <div class="flex justify-between items-center">
      <div
        name="slider-track"
        class="relative overflow-visible rounded-[6px] elevation-1"
        :class="[theme === 'dark' ? 'bg-[#464646AA]' : 'bg-[#00000011]', disabled ? 'opacity-50' : '']"
        :style="{ width: width || '100%', height: height || '30px', cursor: disabled ? 'not-allowed' : 'pointer' }"
      >
        <div class="absolute inset-x-[18%] top-1/2 -translate-y-1/2 flex justify-between pointer-events-none">
          <div
            v-for="i in 6"
            :key="i"
            class="mdi mdi-circle w-2 h-2"
            :style="{
              fontSize: '6px',
              color: theme === 'dark' ? '#ffffff11' : '#00000011',
            }"
          />
        </div>
        <div
          class="absolute translate-y-1/2 bottom-1/2 text-white text-center text-[14px] min-w-[60px] py-[1px] rounded-[6px] elevation-1 z-50 h-3/4"
          :class="isEditingCurrentSliderValue ? 'pointer-events-auto' : 'pointer-events-none select-none'"
          :style="{
            left: pillLeft,
            marginLeft: '5px',
            backgroundColor: color || '#0B5087',
          }"
        >
          <div v-if="!isEditingCurrentSliderValue">
            <p
              class="font-bold select-none"
              draggable="false"
            >
              {{ formatDisplay ? formatDisplay(scaledValue) : scaledValue.toFixed(defaultDecimals) }}
            </p>
          </div>
          <div v-else>
            <input
              ref="editInput"
              v-model.number="editedDisplayValue"
              type="number"
              :min="displayMin"
              :max="displayMax"
              :step="displayStep"
              autofocus
              class="bg-white border border-gray-300 rounded px-1 py-0.5"
              @input="clampEditedValue"
              @keydown="handleValueChange"
              @blur="isEditingCurrentSliderValue = false"
            >
          </div>
        </div>
        <input
          v-model.number="currentSliderValue"
          type="range"
          class="absolute inset-0 w-full h-full opacity-0"
          :class="disabled ? 'cursor-not-allowed' : 'cursor-pointer'"
          style="width: 95%; left: 2.5%"
          :min="min"
          :max="max"
          :step="rawStep"
          :disabled="disabled || isEditingCurrentSliderValue"
          @input="onSliderInput"
          @change="onSliderChange"
          @pointerdown="startInteracting"
          @keydown="onRangeKeydown"
          @keyup="onRangeKeyup"
          @blur="endInteracting"
          @dblclick="isEditingCurrentSliderValue = true"
        >
        <p
          class="absolute min-w-[30px] ml-[10px] mt-1 text-[15px] text-center z-10 pointer-events-none"
          :class="theme === 'dark' ? 'text-[#ffffff44]' : 'text-[#00000066]'"
        >
          {{ labelMinDisplay }}
        </p>
        <p
          class="absolute right-0 min-w-[30px] mr-[10px] mt-1 text-[15px] text-center z-10 pointer-events-none"
          :class="theme === 'dark' ? 'text-[#ffffff44]' : 'text-[#00000066]'"
        >
          {{ labelMaxDisplay }}
        </p>
        <div
          v-if="isEditingCurrentSliderValue"
          class="absolute mdi mdi-check mdi-24px"
          style="right: -30px; top: 50%; transform: translateY(-50%); cursor: pointer; color: green"
          @click="isEditingCurrentSliderValue = false"
        />
      </div>
    </div>
  </div>
</template>
<script setup lang="ts">
import { ref, watch, onBeforeUnmount, computed } from 'vue'

const props = defineProps<{
  /** Pill color override. */
  color?: string
  /** Disable user interaction. Add disabled visual feedback */
  disabled?: boolean
  /** Slider height (default. '30px'). */
  height?: string
  /** Main Label text on the left. */
  label?: string
  /** Custom text for the max label. */
  labelMax?: string
  /** Custom text for the min label. */
  labelMin?: string
  /** Maximum allowed value. */
  max: number
  /** Minimum allowed value. */
  min: number
  /** Name attribute for the range input. */
  name: string
  /** Step increment (default 1). */
  step?: number
  /** Keyboard arrow step multiplier (default 3). */
  keyboardStepMultiplierLimit?: number
  /** 'light' or 'dark' theme. (default 'light')*/
  theme?: 'light' | 'dark' | 'transparent'
  /** Container width (default '100%'). */
  width?: string
  /** Model value for v-model */
  modelValue: number | null
  /** Function to scale the internal percentage value to a display value */
  scaleFn?: (raw: number) => number
  /** Function to unscale the displayed value to the internal percentage value */
  unscaleFn?: (scaled: number) => number
  /** Function format scaled values, converting them to strings */
  formatDisplay?: (scaled: number) => string
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: number | null): void
}>()

const decimalsFromStep = (step: number): number => {
  if (!Number.isFinite(step) || step <= 0) return 0
  const s = step.toString()
  if (s.includes('e-')) {
    const exponent = parseInt(s.split('e-')[1] ?? '0', 10)
    return Math.min(6, Math.max(0, exponent))
  }
  const [, frac = ''] = s.split('.')
  return Math.min(6, frac.length)
}

// Raw value (internal logic always uses this)
const currentSliderValue = ref<number>(props.modelValue ?? props.min)
const lastSentValue = ref<number>(currentSliderValue.value)

// Editing state
const isEditingCurrentSliderValue = ref(false)
const isInteracting = ref(false)
const editedDisplayValue = ref<number>(0)

// Derived display values
const scaledValue = computed(() => 
  props.scaleFn ? props.scaleFn(currentSliderValue.value) : currentSliderValue.value
)

const labelMinDisplay = computed(() =>
  props.labelMin 
    ? props.labelMin 
    : props.formatDisplay 
      ? props.formatDisplay(props.scaleFn ? props.scaleFn(props.min) : props.min)
      : props.min
)

const labelMaxDisplay = computed(() =>
  props.labelMax 
    ? props.labelMax 
    : props.formatDisplay 
      ? props.formatDisplay(props.scaleFn ? props.scaleFn(props.max) : props.max)
      : props.max
)

const displayValue = computed(() => {
  const raw = currentSliderValue.value
  return props.scaleFn ? props.scaleFn(raw) : raw
})

const displayMin = computed(() => props.scaleFn ? props.scaleFn(props.min) : props.min)
const displayMax = computed(() => props.scaleFn ? props.scaleFn(props.max) : props.max)

const rawStep = computed(() => props.step ?? 0.1)

const keyboardStepMultiplier = computed(() => {
  const multiplier = props.keyboardStepMultiplierLimit ?? 3
  return Number.isFinite(multiplier) && multiplier >= 1 ? multiplier : 3
})

// Ballistic keyboard stepping:
// Start at 1x step and ease up toward `keyboardStepMultiplier` while
// the user keeps pressing/holding an arrow key.
const keyboardBallisticLastTs = ref<number>(0)
const keyboardBallisticStreak = ref<number>(0)
const keyboardBallisticDirection = ref<number>(0)

const resetKeyboardBallistics = (): void => {
  keyboardBallisticLastTs.value = 0
  keyboardBallisticStreak.value = 0
  keyboardBallisticDirection.value = 0
}

const ballisticMultiplier = (direction: number): number => {
  const now = Date.now()
  const max = keyboardStepMultiplier.value

  // A new burst starts if the user pauses or changes direction.
  const sameBurst =
    keyboardBallisticDirection.value === direction &&
    now - keyboardBallisticLastTs.value <= 250

  if (!sameBurst) keyboardBallisticStreak.value = 0

  keyboardBallisticStreak.value += 1
  keyboardBallisticLastTs.value = now
  keyboardBallisticDirection.value = direction

  // Ramp to max over a handful of repeats, with an ease-out curve.
  const rampRepeats = 6
  const t = Math.min(1, (keyboardBallisticStreak.value - 1) / rampRepeats)
  const eased = 1 - Math.pow(1 - t, 2)

  return 1 + eased * (max - 1)
}

const displayStep = computed(() => {
  if (!props.scaleFn) return rawStep.value
  const a = props.scaleFn(props.min)
  const b = props.scaleFn(props.min + rawStep.value)
  const delta = Math.abs(b - a)
  return delta > 0 ? delta : rawStep.value
})

const defaultDecimals = computed(() => decimalsFromStep(displayStep.value))

const approxEqual = (a: number, b: number): boolean => {
  const epsilon = Math.max(rawStep.value / 10, 1e-4)
  return Math.abs(a - b) <= epsilon
}

// Pill position logic
const fillWidth = computed(() => {
  const val = currentSliderValue.value
  return ((val - props.min) / (props.max - props.min)) * 100
})
const staticFillWidth = ref<number>(0)

const pillLeft = computed(() =>                      
  `calc((100% - 70px) * ${                           
    (isEditingCurrentSliderValue.value               
      ? staticFillWidth.value
      : fillWidth.value) / 100})`
)

const sendValue = (val: number) => {
  if (val === lastSentValue.value) return
  lastSentValue.value = val
  emit('update:modelValue', val)
}

const commitLockValue = ref<number | null>(null)
let commitLockTimeout: number | null = null
const commitLockUntilMs = ref<number>(0)

const clearCommitLock = (): void => {
  commitLockValue.value = null
  commitLockUntilMs.value = 0
  if (commitLockTimeout) {
    clearTimeout(commitLockTimeout)
    commitLockTimeout = null
  }
}

const setCommitLock = (val: number): void => {
  const lockMs = 2000
  commitLockValue.value = val
  commitLockUntilMs.value = Date.now() + lockMs
  if (commitLockTimeout) clearTimeout(commitLockTimeout)
  commitLockTimeout = window.setTimeout(() => {
    commitLockTimeout = null
    commitLockValue.value = null
    commitLockUntilMs.value = 0
  }, lockMs)
}

// Slider input → update raw
const onSliderInput = (): void => {
  const clamped = Math.min(Math.max(currentSliderValue.value, props.min), props.max)
  currentSliderValue.value = clamped
  throttleSendValue(clamped)
}

const onSliderChange = (): void => {
  endInteracting()
}

// Clamp during input
const clampEditedValue = () => {
  editedDisplayValue.value = Math.min(Math.max(editedDisplayValue.value, displayMin.value), displayMax.value)
}

let throttleTimeout: number | null = null
let pendingValue: number | null = null

const throttleSendValue = (val: number) => {
  pendingValue = val
  if (throttleTimeout) return

  const leading = pendingValue
  pendingValue = null
  sendValue(leading)

  throttleTimeout = window.setTimeout(() => {
    throttleTimeout = null
    if (pendingValue === null) return
    const trailing = pendingValue
    pendingValue = null
    sendValue(trailing)
  }, 500)
}

const flushPendingValue = (): void => {
  if (throttleTimeout) {
    clearTimeout(throttleTimeout)
    throttleTimeout = null
  }
  if (pendingValue === null) return
  const toSend = pendingValue
  pendingValue = null
  sendValue(toSend)
}

const endInteracting = (): void => {
  isInteracting.value = false

  const clamped = Math.min(Math.max(currentSliderValue.value, props.min), props.max)
  currentSliderValue.value = clamped
  pendingValue = clamped
  flushPendingValue()
  sendValue(clamped)
  setCommitLock(clamped)
}

const handlePointerUp = (): void => endInteracting()
const handlePointerCancel = (): void => endInteracting()

const startInteracting = (): void => {
  if (props.disabled || isEditingCurrentSliderValue.value) return
  isInteracting.value = true
  clearCommitLock()
  window.addEventListener('pointerup', handlePointerUp, { once: true })
  window.addEventListener('pointercancel', handlePointerCancel, { once: true })
}

const isArrowKey = (key: string): boolean =>
  key === 'ArrowLeft' || key === 'ArrowRight' || key === 'ArrowUp' || key === 'ArrowDown'

const onRangeKeydown = (e: KeyboardEvent): void => {
  if (props.disabled || isEditingCurrentSliderValue.value) return
  if (!isArrowKey(e.key)) return

  // Override native range keyboard step so we can apply ballistics.
  e.preventDefault()

  const direction = (e.key === 'ArrowRight' || e.key === 'ArrowUp') ? 1 : -1
  const step = rawStep.value

  if (!Number.isFinite(step) || step <= 0) return

  const multiplier = ballisticMultiplier(direction)
  const keyboardStep = step * multiplier

  const next = currentSliderValue.value + direction * keyboardStep
  const clamped = Math.min(Math.max(next, props.min), props.max)

  // Align to step grid to avoid float drift.
  const stepsFromMin = Math.round((clamped - props.min) / step)
  const aligned = Math.min(
    Math.max(props.min + stepsFromMin * step, props.min),
    props.max,
  )

  currentSliderValue.value = aligned
  throttleSendValue(aligned)

  if (!isInteracting.value) {
    isInteracting.value = true
    clearCommitLock()
  }
}

const onRangeKeyup = (e: KeyboardEvent): void => {
  if (!isArrowKey(e.key)) return
  resetKeyboardBallistics()
  endInteracting()
}

// Keyboard handling
const handleValueChange = (e: KeyboardEvent): void => {
  if (e.key === 'Escape') {
    isEditingCurrentSliderValue.value = false
    editedDisplayValue.value = props.scaleFn ? props.scaleFn(lastSentValue.value) : lastSentValue.value
    currentSliderValue.value = lastSentValue.value
  } else if (e.key === 'Enter') {
    isEditingCurrentSliderValue.value = false
  }
}

// Sync from parent
watch(
  () => props.modelValue,
  (v) => {
    const next = v ?? props.min
    if (isEditingCurrentSliderValue.value || isInteracting.value) return
    if (
      commitLockValue.value !== null &&
      Date.now() < commitLockUntilMs.value &&
      !approxEqual(next, commitLockValue.value)
    ) {
      return
    }
    currentSliderValue.value = next
  },
  { immediate: true }
)

watch(isEditingCurrentSliderValue, (isEditing) => {
  if (isEditing) {
    editedDisplayValue.value = displayValue.value
    staticFillWidth.value = fillWidth.value
  } else {
    const raw = props.unscaleFn ? props.unscaleFn(editedDisplayValue.value) : editedDisplayValue.value
    currentSliderValue.value = Math.min(Math.max(raw, props.min), props.max)
    sendValue(currentSliderValue.value)
    setCommitLock(currentSliderValue.value)
  }
})

onBeforeUnmount(() => {
  if (throttleTimeout) clearTimeout(throttleTimeout)
  if (commitLockTimeout) clearTimeout(commitLockTimeout)
  window.removeEventListener('pointerup', handlePointerUp)
  window.removeEventListener('pointercancel', handlePointerCancel)
})
</script>
