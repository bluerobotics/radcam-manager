<template>
  <v-dialog
    :model-value="props.show"
    persistent
    width="400px"
    @click:outside="close"
  >
    <div class="error-card flex flex-col items-center justify-center pt-[10px] rounded-lg">
      <img
        src="../../public/assets/logo.svg"
        class="w-[120px] h-[120px] mb-1"
        alt="logo"
      >
      <v-card-title class="text-h6 text-center py-2 text-white">
        Hardware Setup Required
      </v-card-title>
      <v-card-text class="px-6 pb-3 text-center">
        <p class="text-sm text-white">
          Your camera actuators are not configured yet.
        </p>
        <p class="text-xs opacity-70 mt-3 text-white">
          Please complete the <b>Hardware Setup</b> section below to enable full functionality.
        </p>
      </v-card-text>
      <v-card-actions class="px-4 pb-4">
        <v-spacer />
        <v-btn
          class="py-1 px-4 rounded-md bg-[#414141] hover:bg-[#0A3E6B]"
          size="small"
          variant="elevated"
          theme="dark"
          @click="close"
        >
          Go to Setup
        </v-btn>
      </v-card-actions>
    </div>
  </v-dialog>
</template>

<script setup lang="ts">
const props = defineProps<{
  show: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const close = () => {
  emit('close')
}
</script>

<style scoped>
.error-card {
  background-color: #10101065;
  backdrop-filter: blur(5px);
  border: 1px solid rgba(255, 255, 255, 0.08);
  box-shadow: 0px 4px 4px 0px #00000033, 0px 8px 12px 6px #00000026;
  min-height: 280px;
  width: 100%;
}
</style>