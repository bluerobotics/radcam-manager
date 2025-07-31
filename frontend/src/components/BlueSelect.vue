<template>
  <div class="flex w-full justify-between items-center">
    <div v-if="label">
      <label
        class="text-start mr-6"
        :class="theme === 'dark' ? 'text-white' : 'text-black'"
      >
        {{ label }}
      </label>
    </div>
    <div v-else />
    <slot name="insetElement" />

    <v-menu
      offset-y
      :disabled="disabled"
      class="flex"
      style="z-index: 999999"
      :close-on-content-click="!multiSelect"
    >
      <template #activator="{ props: menuProps }">
        <button
          v-bind="menuProps"
          class="relative inline-flex items-center justify-between pl-4 rounded-[6px] elevation-1"
          :class="[
            theme === 'dark' ? 'bg-[#464646]' : 'bg-[#00000011]',
            disabled ? 'opacity-50 pointer-events-none' : 'cursor-pointer',
          ]"
          :style="{ height: height || '30px', width: width || 'auto' }"
        >
          <span
            class="text-sm font-medium truncate mr-1 maxWidth-[100%] -mb-[1px]"
            :class="selectedTextClass"
          >
            {{ !multiSelect ? selectedItem.name : selectedValues.length > 0 ? selectedValues.join(', ') : 'Select...' }}
          </span>
          <v-icon :class="['transition-transform', iconClass]">
            mdi-menu-down
          </v-icon>
        </button>
      </template>

      <v-list
        class="py-0"
        :theme="theme"
      >
        <v-list-item
          v-for="(opt, idx) in items"
          :key="opt.name"
          :disabled="disabled || opt.disabled"
          @click="selectOption(idx)"
        >
          <div class="flex justify-between">
            <v-icon
              v-if="selectedValues.includes(opt.value || opt.name)"
              class="mt-1 text-[16px]"
            >
              mdi-check
            </v-icon>
            <div v-else />
            <v-list-item-title :class="itemTextClass(opt)">
              {{ opt.name }}
            </v-list-item-title>
          </div>
        </v-list-item>
      </v-list>
    </v-menu>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'

/**
 * Option in the select
 */
interface OptionItem {
  /**
   * Name of the option to display
   */
  name: string
  /**
   * Value of the option, if not provided, name will be used
   */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  value?: any
  /**
   * Whether the option is disabled
   */
  disabled?: boolean
  /**
   * Callback when the option is selected
   */
  onSelected?: () => void
}

const props = defineProps<{
  /** Options to select from */
  items: OptionItem[]
  /** Disable entire select */
  disabled?: boolean
  /** light or dark theme */
  theme?: 'light' | 'dark'
  /** Label on the left */
  label?: string
  /** Control height */
  height?: string
  /** Control width */
  width?: string
  /** Multiple values selection */
  multiSelect?: boolean
  /** Model value for v-model */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  modelValue?: any
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: string | number | (string | number)[]): void
}>()

const selectedValues = ref<(string | number)[]>([])
const selectedItem = ref<OptionItem>({ name: 'Select...' })

const selectOption = (index: number) => {
  if (props.multiSelect) {
    const value = props.items[index].value || props.items[index].name
    const idx = selectedValues.value.indexOf(value)
    if (idx > -1) {
      selectedValues.value.splice(idx, 1)
    } else {
      selectedValues.value.push(value)
    }
    emit('update:modelValue', selectedValues.value)
  } else {
    const value = props.items[index].value || props.items[index].name
    selectedItem.value = props.items[index]
    emit('update:modelValue', value)
  }
}

const iconClass = computed(() => (props.theme === 'dark' ? 'text-white' : 'text-black'))
const selectedTextClass = computed(() => (props.theme === 'dark' ? 'text-white' : 'text-black'))

/**
 * Returns the class for the item text based on the option and props
 * @param opt
 */
function itemTextClass(opt: OptionItem) {
  if (opt.disabled || props.disabled) return 'text-gray-500'
  return props.theme === 'dark' ? 'text-white' : 'text-black'
}

watch(
  () => props.modelValue,
  (newVal) => {
    if (!props.multiSelect) {
      const found = props.items.find(item =>
        item.value !== undefined ? item.value === newVal : item.name === newVal
      )
      selectedItem.value = found ?? { name: 'Select...' }
    } else if (Array.isArray(newVal)) {
      selectedValues.value = newVal
    }
  },
  { immediate: true }
)
</script>
