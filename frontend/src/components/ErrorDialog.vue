<template>
  <StatusDialogShell
    :show="!!props.message"
    title="Operation Failed"
    @click-outside="close"
  >
    <div class="text-center">
      <p class="text-sm text-white">
        {{ props.message }}
      </p>
      <p class="text-xs opacity-70 mt-3 text-white">
        Please try again. If the problem persists, contact support
      </p>
    </div>
    <template #actions>
      <v-spacer />
      <v-btn
        class="py-1 px-4 rounded-md bg-[#414141] hover:bg-[#0A3E6B]"
        size="small"
        variant="elevated"
        theme="dark"
        @click="close"
      >
        Close
      </v-btn>
    </template>
  </StatusDialogShell>
</template>

<script setup lang="ts">
import StatusDialogShell from '@/components/StatusDialogShell.vue'

const props = defineProps<{
  message: string | null
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const close = () => {
  emit('close')
}
</script>
