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
import type { CameraConnectivity, SystemHealth } from '@/bindings/radcam_api'
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

const formatOptional = (value?: bigint | number | string | null): string =>
  value == null || value === '' ? '—' : String(value)

const rawHealthText = computed((): string => {
  const health = props.systemHealth
  if (!health) return ''
  const diagnostics = health.diagnostics
  const lines: string[] = [
    `mcm: ${health.mcm}`,
    `mcm_detail: ${formatOptional(health.mcm_detail)}`,
    `cameras_discovered: ${health.cameras_discovered}`,
    `autopilot: ${health.autopilot}`,
    `autopilot_detail: ${formatOptional(health.autopilot_detail)}`,
    `param_encoding: ${formatOptional(diagnostics.param_encoding)}`,
    `mavlink_reconnects: ${diagnostics.mavlink_reconnects}`,
    `mavlink_frames_lagged: ${diagnostics.mavlink_frames_lagged}`,
    `state_events_lagged: ${diagnostics.state_events_lagged}`,
    `mcm_consecutive_failures: ${diagnostics.mcm_consecutive_failures}`,
    `last_frame_age_ms: ${formatOptional(diagnostics.last_frame_age_ms)}`,
    `last_heartbeat_age_ms: ${formatOptional(diagnostics.last_heartbeat_age_ms)}`,
    `last_servo_age_ms: ${formatOptional(diagnostics.last_servo_age_ms)}`,
    `backend_version: ${diagnostics.backend_version}`,
  ]
  if (props.cameraConnectivity != null) {
    lines.push(`camera_connectivity: ${props.cameraConnectivity}`)
  }
  return lines.join('\n')
})

const copyHealthDiagnostics = async (): Promise<void> => {
  await copyDiagnosticsPayload({
    system_health: props.systemHealth,
    camera_connectivity: props.cameraConnectivity ?? null,
    problem_titles: props.problemTitles ?? [],
    user_agent: navigator.userAgent,
  })
}
</script>
