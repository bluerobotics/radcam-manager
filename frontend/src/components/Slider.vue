<template>
  <div class="slider-row">
    <label class="slider-label">{{ label }}</label>
    <v-slider
      v-model="current"
      class="slider-component"
      :name="name"
      :min="min"
      :max="max"
      :step="step"
      :disabled="disabled"
      hide-details
      @mousedown="startSliding"
      @touchstart.passive="startSliding"
      @mouseup="stopSliding"
      @touchend="stopSliding"
      @input="onSliderInput"
    >
      <template #append>
        <v-text-field
          v-model.number="current"
          class="slider-field"
          density="compact"
          type="number"
          width="90px"
          hide-details
          single-line
        />
      </template>
    </v-slider>
  </div>
</template>

<script setup lang="ts">
import { ref, onBeforeUnmount, watch } from 'vue'

const props = withDefaults(defineProps<{
  current: number,
  name: string
  label: string
  min: number
  max: number
  step?: number
  disabled?: boolean
}>(), {
  step: 1,
  disabbled: false
})

const current = ref(props.current)
const lastSentValue = ref(current.value)
const emit = defineEmits<{
  (e: 'update:current', value: number): void
}>()
let sliderInterval: number | null = null
let isSliding = false

watch(() => props.current, (newVal) => {
  current.value = newVal;
});

onBeforeUnmount(() => {
  if (sliderInterval) {
    clearInterval(sliderInterval)
    sliderInterval = null
  }
})

const sendValue = async (value: number) => {
  if (value === lastSentValue.value) return
  lastSentValue.value = value

  emit('update:current', current.value) // Emit the updated value back to the parent
}

const onSliderInput = () => {
  current.value = Math.min(Math.max(current.value, props.min), props.max)

  if (!isSliding) {
    sendValue(current.value)
  }

}

const startSliding = () => {
  isSliding = true
  if (!sliderInterval) {
    sliderInterval = setInterval(() => {
      if (isSliding) {
        sendValue(current.value)
      }
    }, 500)
  }
}

const stopSliding = () => {
  isSliding = false
  if (sliderInterval) {
    clearInterval(sliderInterval)
    sliderInterval = null
  }
  sendValue(current.value)
}
</script>
<style scoped>
/* Row for each slider */
.slider-row {
  display: flex;
  align-items: center;
}

/* Label styling */
.slider-label {
  width: 80px;
  text-align: right;
  margin-right: 5px;
}

/* Slider styling */
.slider-component {
  flex: 1;
}

/* .slider-field {
} */
</style>