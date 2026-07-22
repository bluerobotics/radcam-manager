<template>
  <Teleport to="body">
    <Transition name="toast-fade">
      <div
        v-if="message"
        class="warning-toast"
      >
        <v-card
          class="bg-[#363636dd] backdrop-blur-sm border border-[#ffffff22] text-white px-4 py-2 rounded-md shadow-lg"
          theme="dark"
        >
          <v-icon
            :icon="icon"
            class="mr-2"
          />
          <span class="text-sm break-words">{{ message }}</span>
        </v-card>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
withDefaults(
  defineProps<{
    message: string | null
    icon?: string
  }>(),
  {
    icon: 'mdi-alert-circle-outline',
  },
)
</script>

<style scoped>
.warning-toast {
  position: fixed;
  bottom: 1rem;
  left: 50%;
  z-index: 9999;
  max-width: min(90vw, 640px);
  transform: translateX(-50%);
  pointer-events: none;
}

.warning-toast :deep(.v-card) {
  pointer-events: auto;
}

.toast-fade-enter-active,
.toast-fade-leave-active {
  transition: opacity 0.2s ease;
}

.toast-fade-enter-from,
.toast-fade-leave-to {
  opacity: 0;
}
</style>
