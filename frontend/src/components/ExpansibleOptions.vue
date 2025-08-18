<template>
  <div
    class="flex flex-col"
    :class="buttonClass"
  >
    <div class="flex justify-start items-center">
      <button
        class="inline-flex items-center text-[12px] font-medium font-xs focus:outline-none opacity-75 hover:opacity-100"
        style="text-transform: none"
        @click="toggle"
      >
        <div class="flex items-center mr-2">
          {{ isOpen ? closeLabel : openLabel }}
          <v-icon
            size="18"
            :class="['transition-transform', isOpen ? 'rotate-180' : 'rotate-0']"
          >
            mdi-menu-down
          </v-icon>
        </div>
      </button>
      <div class="h-[1px] w-[15%] bg-[#ffffff25] my-2 ml-6" />
    </div>

    <transition name="expand">
      <div
        v-show="isOpen"
        class="flex justify-end bg-[#00000011] rounded-[8px] pa-3"
        :class="contentClass"
      >
        <slot />
      </div>
    </transition>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'

/**
 * ExpansiblePanel component allows toggling visibility of its content.
 */
interface Props {
  /**
   * Label for the button when the panel is open.
   */
  openLabel?: string
  /**
   * Label for the button when the panel is closed.
   */
  closeLabel?: string
  /**
   * Initial state of the panel when it is first rendered.
   */
  initiallyOpen?: boolean
  /**
   * Controls the visibility of the panel.
   */
  isOpen?: boolean
  /**
   * Custom class for the button container.
   */
  buttonClass?: string
  /**
   * Custom class for the content area.
   */
  contentClass?: string
}

const props = defineProps<Props>()
const emit = defineEmits<{
  (e: 'update:isOpen', value: boolean): void
}>()

const { openLabel = 'More', closeLabel = 'Less', initiallyOpen = false } = props

const isOpen = ref<boolean>(props.isOpen ?? initiallyOpen)

/**
 *
 */
function toggle(): void {
  isOpen.value = !isOpen.value
  emit('update:isOpen', isOpen.value)
}

watch(
  () => props.isOpen,
  (value) => {
    if (value !== undefined) {
      isOpen.value = value
    }
  },
  { immediate: true }
)
</script>

<style scoped>
.expand-enter-active,
.expand-leave-active {
  transition: max-height 0.2s;
  overflow: hidden;
}
.expand-enter-from,
.expand-leave-to {
  max-height: 0;
}
.expand-enter-to,
.expand-leave-from {
  max-height: 500px;
}
</style>
