<template>
  <ExpansiblePanel
    title="Diagnostics for support"
    :expanded="false"
    theme="dark"
  >
    <div class="flex flex-wrap items-center gap-2 mb-4">
      <v-btn
        class="py-1 px-3 rounded-md bg-[#414141] hover:bg-[#0A3E6B]"
        size="small"
        variant="elevated"
        theme="dark"
        @click="copyHealthDiagnostics"
      >
        Copy diagnostics
      </v-btn>
    </div>
    <textarea
      v-if="copyFallbackText"
      :value="copyFallbackText"
      readonly
      class="mb-4 w-full text-xs font-mono opacity-80"
      rows="6"
      aria-label="Diagnostics text for manual copy"
      @focus="($event.target as HTMLTextAreaElement).select()"
    />
    <p
      v-if="!props.systemHealth"
      class="text-sm opacity-70 mb-4"
    >
      Waiting for health data…
    </p>
    <details
      v-else
      class="text-sm"
    >
      <summary class="opacity-70 cursor-pointer mb-2">
        Raw health data
      </summary>
      <pre class="mt-2 text-xs font-mono whitespace-pre-wrap opacity-90">{{ rawHealthText }}</pre>
    </details>
  </ExpansiblePanel>
  <CopyFeedbackToast
    :message="copyFeedback"
    @dismiss="clearCopyFeedbackMessage"
  />
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { CameraConnectivity, SystemHealth } from '@/bindings/br4kcam_api'
import CopyFeedbackToast from '@/components/CopyFeedbackToast.vue'
import { useCopyDiagnostics } from '@/utils/useCopyDiagnostics'
import ExpansiblePanel from './ExpansiblePanel.vue'

const props = defineProps<{
  systemHealth: SystemHealth | null
  cameraConnectivity?: CameraConnectivity | null
  problemTitles?: string[]
}>()

const {
  copyFeedback,
  copyFallbackText,
  clearCopyFeedbackMessage,
  copyDiagnostics: copyDiagnosticsPayload,
} = useCopyDiagnostics()

const rawHealthText = computed((): string =>
  props.systemHealth ? JSON.stringify(props.systemHealth, null, 2) : '',
)

const copyHealthDiagnostics = async (): Promise<void> => {
  await copyDiagnosticsPayload({
    system_health: props.systemHealth,
    camera_connectivity: props.cameraConnectivity ?? null,
    problem_titles: props.problemTitles ?? [],
    user_agent: navigator.userAgent,
  })
}
</script>
