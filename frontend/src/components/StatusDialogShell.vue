<template>
  <v-dialog
    :model-value="props.show"
    :persistent="props.persistent"
    :width="props.width"
    @update:model-value="(value) => { if (!value) emit('dismiss') }"
    @click:outside="emit('click-outside')"
  >
    <div class="status-card flex flex-col items-center pt-[10px] rounded-lg max-h-[90vh]">
      <img
        v-if="props.logo"
        src="../../public/assets/logo.svg"
        class="w-[120px] h-[120px] mb-1 shrink-0"
        alt=""
      >
      <v-card-title class="text-h6 text-center py-2 text-white shrink-0">
        {{ props.title }}
      </v-card-title>
      <v-card-text class="px-6 pb-3 w-full flex-1 min-h-0 overflow-y-auto">
        <slot />
      </v-card-text>
      <v-card-actions
        v-if="$slots.actions"
        class="px-4 pb-4 w-full shrink-0"
      >
        <slot name="actions" />
      </v-card-actions>
    </div>
  </v-dialog>
</template>

<script setup lang="ts">
// Defaults must go through withDefaults: Vue casts an absent boolean prop to false,
// so an omitted `logo` or `persistent` would otherwise read as an explicit false.
const props = withDefaults(
  defineProps<{
    show: boolean
    title: string
    width?: string
    /** False lets a click outside or Esc close the dialog. */
    persistent?: boolean
    /** False drops the logo, for compact confirmation dialogs. */
    logo?: boolean
  }>(),
  {
    width: '400px',
    persistent: true,
    logo: true,
  },
)

const emit = defineEmits<{
  (e: 'click-outside'): void
  (e: 'dismiss'): void
}>()
</script>

<style scoped>
.status-card {
  background-color: #10101065;
  backdrop-filter: blur(5px);
  border: 1px solid rgba(255, 255, 255, 0.08);
  box-shadow: 0px 4px 4px 0px #00000033, 0px 8px 12px 6px #00000026;
  width: 100%;
}
</style>
