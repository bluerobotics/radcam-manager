<template>
  <div class="flex w-full justify-between items-center">
    <div v-if="label">
      <label
        class="text-start mr-6"
        :class="theme === 'dark' ? 'text-white' : 'text-black'"
      >{{ label }}</label>
    </div>
    <div
      name="switch-track"
      class="relative rounded-[8px] elevation-1 cursor-pointer overflow-hidden"
      :class="[theme === 'dark' ? 'bg-[#464646AA]' : 'bg-[#00000011]', disabled ? 'opacity-50 cursor-not-allowed' : '']"
      :style="{ minWidth: width || '75px', height: height || '30px' }"
      @click="toggleSwitch"
    >
      <p
        class="absolute left-[8px] top-1/2 -translate-y-1/2 text-[11px] pointer-events-none"
        :class="theme === 'dark' ? 'text-[#ffffff44]' : 'text-[#00000066]'"
      >
        {{ labelOff || '' }}
      </p>
      <p
        class="absolute right-[7px] top-1/2 -translate-y-1/2 text-[11px] pointer-events-none"
        :class="theme === 'dark' ? 'text-[#ffffff44]' : 'text-[#00000066]'"
      >
        {{ labelOn || '' }}
      </p>
      <div
        class="absolute top-0 bottom-0 flex items-center justify-center text-[14px] text-white px-2 rounded-[8px] elevation-1 transition-all duration-300 my-[4px] mx-[4px]"
        :style="{
          left: modelValue ? '30%' : 0,
          width: 'calc(60%)',
          backgroundColor: modelValue ? color || '#0B5087' : '#777777',
        }"
      >
        {{ modelValue ? labelOn || 'On' : labelOff || 'Off' }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'

const props = defineProps<{
  /** Color of the switch's knob when it is on. */
  color?: string
  /** The current value of the switch. */
  modelValue: boolean | null
  /** Whether the switch is disabled. */
  disabled?: boolean
  /** Height of the switch container. */
  height?: string
  /** Label on the left side of the component. */
  label?: string
  /** Custom text for the switch when it is on. */
  labelOn?: string
  /** Custom text for the switch when it is off. */
  labelOff?: string
  /** Name of the component's container. */
  name: string
  /** Theme of the component. */
  theme?: 'light' | 'dark'
  /** Minimum width of the container. */
  width?: string
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
}>()

const modelValue = ref(props.modelValue || false)

const toggleSwitch = (): void => {
  if (props.disabled) return
  modelValue.value = !modelValue.value
  emit('update:modelValue', modelValue.value)
}

watch(
  () => props.modelValue,
  (v) => (modelValue.value = v ?? false),
  { immediate: true }
)
</script>
