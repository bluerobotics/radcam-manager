<template>
  <div class="rounded-lg overflow-hidden">
    <div
      class="flex items-center justify-between bg-transparent cursor-pointer select-none px-3 my-2"
      @click="togglePanel"
    >
      <div class="flex-1 flex items-center">
        <div
          class="flex-1 h-px"
          :class="props.theme === 'dark' ? 'bg-[#ffffff22]' : 'bg-[#00000033]'"
        />
        <span
          class="mx-3 text-sm font-medium opacity-60"
          :class="props.theme === 'dark' ? 'text-white' : 'text-black'"
        >{{ title }}</span>
        <div
          class="flex-1 h-px"
          :class="props.theme === 'dark' ? 'bg-[#ffffff11]' : 'bg-[#00000033]'"
        />
      </div>
      <v-icon
        :class="['ml-3 transition-transform', props.theme === 'dark' ? 'text-white' : 'text-black']"
        class="opacity-40"
      >
        {{ isOpen ? 'mdi-menu-up' : 'mdi-menu-down' }}
      </v-icon>
    </div>
    <transition name="expand">
      <div
        v-show="isOpen"
        class="px-8 my-6 pb-2"
        :class="props.theme === 'dark' ? 'border-[#555555]' : 'border-[#dddddd]'"
      >
        <slot />
      </div>
    </transition>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'

const props = defineProps<{
  /** Panel title */
  title: string
  /** Initial open state */
  expanded?: boolean
  /** 'light' or 'dark' theme */
  theme?: 'light' | 'dark'
}>()

const emit = defineEmits<{
  (e: 'update:expanded', value: boolean): void
}>()

const isOpen = ref(props.expanded ?? true)

const togglePanel = (): void => {
  isOpen.value = !isOpen.value
  emit('update:expanded', isOpen.value)
}

watch(
  () => props.expanded,
  (val) => {
    if (val !== undefined) isOpen.value = val
  }
)
</script>
<style scoped>
.expand-enter-active,
.expand-leave-active {
  transition: max-height 0.3s ease;
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
