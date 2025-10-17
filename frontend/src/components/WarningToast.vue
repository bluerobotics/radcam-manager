<template>
  <transition name="slide-fade">
    <div
      v-if="message"
      class="warning-toast fixed bottom-4 left-1/2 transform -translate-x-1/2 z-50"
    >
      <v-card
        class="bg-[#363636dd] backdrop-blur-sm border border-[#ffffff22] text-white px-4 py-2 rounded-md shadow-lg"
        theme="dark"
      >
        <v-icon
          icon="mdi-alert-circle-outline"
          class="mr-2"
        />
        <span class="text-sm">{{ message }}</span>
      </v-card>
    </div>
  </transition>
</template>

<script setup lang="ts">
defineProps<{
  message: string | null
}>()
</script>

<style scoped>
.slide-fade-enter-active {
  transition: all 0.3s ease-out;
}
.slide-fade-leave-active {
  transition: all 0.2s ease-in;
}
.slide-fade-enter-from,
.slide-fade-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(10px);
}
</style>