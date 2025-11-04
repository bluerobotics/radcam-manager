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
        @mousedown="startSliding"
        @mouseup="stopSliding"
        @mouseleave="stopSliding"
        @touchstart="startSliding"
        @touchend="stopSliding"
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
              {{ formatDisplay ? formatDisplay(scaledValue) : (step && step < 1 ? scaledValue?.toFixed(1) || '0' : scaledValue.toFixed(0)) }}
            </p>
          </div>
          <div v-else>
            <input
              ref="editInput"
              v-model.number="scaledValueForEdit"
              type="number"
              :min="displayMin"
              :max="displayMax"
              :step="step || 0.1"
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
          :step="step || 0.1"
          :disabled="disabled || isEditingCurrentSliderValue"
          @input="onSliderInput"
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

// Raw value (internal logic always uses this)
const currentSliderValue = ref<number>(props.modelValue ?? props.min)
const lastSentValue = ref<number>(currentSliderValue.value)

// Editing state
const isEditingCurrentSliderValue = ref(false)
const scaledValueForEdit = ref(0)
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

// Slider input → update raw
const onSliderInput = (): void => {
  const clamped = Math.min(Math.max(currentSliderValue.value, props.min), props.max)
  currentSliderValue.value = clamped
  if (!isSliding) sendValue(clamped)
}

// Handle double-click edit start
watch(isEditingCurrentSliderValue, (v) => {
  if (v) {
    staticFillWidth.value = fillWidth.value
    editedDisplayValue.value = displayValue.value
  } else {
    // Commit edited value
    if (props.unscaleFn) {
      const raw = props.unscaleFn(editedDisplayValue.value)
      currentSliderValue.value = Math.min(Math.max(raw, props.min), props.max)
    } else {
      currentSliderValue.value = Math.min(Math.max(editedDisplayValue.value, props.min), props.max)
    }
    sendValue(currentSliderValue.value)
  }
})

// Clamp during input
const clampEditedValue = () => {
  editedDisplayValue.value = Math.min(Math.max(editedDisplayValue.value, displayMin.value), displayMax.value)
}

// Sliding logic (for continuous updates)
let sliderInterval: number | null = null
let isSliding = false
const startSliding = (): void => {
  isSliding = true
  if (!sliderInterval) {
    sliderInterval = window.setInterval(() => {
      if (isSliding) sendValue(currentSliderValue.value)
    }, 500)
  }
}

const stopSliding = (): void => {
  isSliding = false
  if (sliderInterval) {
    clearInterval(sliderInterval)
    sliderInterval = null
  }
  sendValue(currentSliderValue.value)
}

// Keyboard handling
const handleValueChange = (e: KeyboardEvent): void => {
  if (e.key === 'Escape') {
    isEditingCurrentSliderValue.value = false
    currentSliderValue.value = lastSentValue.value
  } else if (e.key === 'Enter') {
    isEditingCurrentSliderValue.value = false
  }
}

// Sync from parent
watch(
  () => props.modelValue,
  (v) => (currentSliderValue.value = v ?? props.min),
  { immediate: true }
)

watch(isEditingCurrentSliderValue, (isEditing) => {
  if (isEditing) {
    scaledValueForEdit.value = scaledValue.value
    staticFillWidth.value = fillWidth.value
  } else {
    if (props.unscaleFn) {
      const raw = props.unscaleFn(scaledValueForEdit.value)
      currentSliderValue.value = Math.min(Math.max(raw, props.min), props.max)
    } else {
      currentSliderValue.value = Math.min(Math.max(scaledValueForEdit.value, props.min), props.max)
    }
    sendValue(currentSliderValue.value)
  }
})

onBeforeUnmount(() => {
  if (sliderInterval) clearInterval(sliderInterval)
})
</script>
