<template>
  <StatusDialogShell
    :show="props.show"
    title="Hardware Setup Required"
    @click-outside="close"
  >
    <div class="text-center">
      <p class="text-sm text-white">
        Your camera actuators are not configured yet.
      </p>
      <p class="text-xs opacity-70 mt-3 text-white">
        Please complete the <b>Hardware Setup</b> section below to enable full functionality.
      </p>
    </div>
    <template #actions>
      <v-spacer />
      <v-btn
        class="py-1 px-4 rounded-md bg-[#414141] hover:bg-[#0A3E6B]"
        size="small"
        variant="elevated"
        theme="dark"
        @click="goToSetup"
      >
        Go to setup
      </v-btn>
    </template>
  </StatusDialogShell>
</template>

<script setup lang="ts">
import StatusDialogShell from '@/components/StatusDialogShell.vue'

const props = defineProps<{
  show: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'go-to-setup'): void
}>()

const close = () => {
  emit('close')
}

const goToSetup = () => {
  emit('go-to-setup')
}
</script>
