<template>
  <v-container
    no-gutters
    class="max-w-[800px] text-white pa-0  rounded-[8px] elevation-5 no-user-select"
    :class="[
      theme === 'dark' ? 'bg-[#363636]' : 'bg-[#F5F5F5]',
      {
        'transparent-card mb-10': isCockpitMode,
        'mt-6': !isCockpitMode,
      },
    ]"
  >
    <div
      class="flex items-center justify-between rounded-t-[8px]"
      :class="isCockpitMode ? 'bg-[#2C2C2C88]' : 'bg-[#15151577]'"
    >
      <div class="flex items-center justify-around w-[400px] pl-5 border-b-[1px] border-[#ffffff88]">
        <v-menu
          offset-y
          theme="dark"
          class="cursor-pointer"
        >
          <template #activator="{ props, isActive }">
            <v-icon
              v-bind="props"
              class="mt-[-2px] ml-[-5px]"
            >
              {{ isActive ? 'mdi-menu-open' : 'mdi-menu-close' }}
            </v-icon>
          </template>
          <v-list class="pa-0 border-[1px] border-[#ffffff22] rounded-[4px]">
            <v-list-item
              :disabled="!backendConnected"
              @click="updateLuaScript"
            >
              <v-list-item-title class="flex">
                Update Lua script
              </v-list-item-title>
            </v-list-item>
            <v-list-item
              :disabled="!backendConnected || cameraBackedControlsDisabled"
              @click="applyRecommendedCameraSettings"
            >
              <v-list-item-title class="flex">
                Apply recommended camera settings
              </v-list-item-title>
            </v-list-item>
            <v-list-item
              :disabled="!backendConnected || cameraBackedControlsDisabled"
              @click="rebootCamera"
            >
              <v-list-item-title class="flex">
                Reboot camera
              </v-list-item-title>
            </v-list-item>
            <v-divider />
            <v-divider />
          </v-list>
        </v-menu>
        <v-select
          v-model="selectedCameraUUID"
          :items="cameraOptions"
          item-title="label"
          item-value="uuid"
          label="Camera"
          hide-details
          theme="dark"
          class="bg-[#15151577] ml-3 -mb-[1px]"
        >
          <template #selection="{ item }">
            <div class="flex items-center gap-1 min-w-0">
              <v-icon
                v-if="item.raw.missing"
                icon="mdi-magnify-scan"
                size="16"
                class="shrink-0 opacity-70"
              />
              <span class="truncate">{{ item.title }}</span>
            </div>
          </template>
          <template #item="{ props, item }">
            <v-list-item
              v-bind="props"
              :prepend-icon="item.raw.missing ? 'mdi-magnify-scan' : undefined"
              :subtitle="item.raw.missing ? 'Waiting for discovery' : item.raw.uuid"
            />
          </template>
        </v-select>
      </div>
      <div class="flex items-center mr-4">
        <v-tooltip
          v-if="showBusyChip"
          location="bottom"
          open-delay="200"
        >
          <template #activator="{ props: tooltipProps }">
            <v-progress-circular
              v-bind="tooltipProps"
              indeterminate
              color="#9ec9ef"
              size="16"
              width="2"
              class="mr-2 cursor-pointer health-focusable"
              role="button"
              tabindex="0"
              aria-label="Open system status"
              @click="onBusyChipOpen"
              @keydown.enter="onBusyChipOpen"
              @keydown.space.prevent="onBusyChipOpen"
            />
          </template>
          <div class="text-sm leading-snug max-w-[280px]">
            <div class="font-medium">
              {{ uiLoadingMessage }}
            </div>
            <div class="mt-1 opacity-90">
              Still running. Click to reopen system status.
            </div>
          </div>
        </v-tooltip>
        <v-tooltip
          v-if="showHealthChip"
          location="bottom"
          open-delay="200"
        >
          <template #activator="{ props: tooltipProps }">
            <v-icon
              v-bind="tooltipProps"
              class="mr-2 cursor-pointer health-focusable"
              color="warning"
              size="18"
              icon="mdi-alert"
              role="button"
              tabindex="0"
              aria-label="Open system status"
              @click="onDegradedBannerOpen"
              @keydown.enter="onDegradedBannerOpen"
              @keydown.space.prevent="onDegradedBannerOpen"
            />
          </template>
          <div class="text-sm leading-snug max-w-[280px]">
            <div class="font-medium">
              {{ degradedBannerTitle }}
            </div>
            <div class="mt-1 opacity-90">
              {{ degradedBannerTooltip }}
            </div>
          </div>
        </v-tooltip>
        <v-tooltip
          location="bottom"
          open-delay="200"
        >
          <template #activator="{ props: tooltipProps }">
            <span
              v-bind="tooltipProps"
              class="connection-status-wrap"
            >
              <v-icon
                :icon="connectionIcon"
                :color="connectionColor"
                size="14"
                class="connection-status-icon"
                :class="{ 'connection-status-icon--spinning': connectionState === 'connecting' }"
              />
            </span>
          </template>
          <div class="connection-stats-tooltip text-sm leading-snug">
            <div>{{ connectionStatusLine }}</div>
            <template v-if="connectionState === 'connected' && connectionStats">
              <div>{{ connectionStats.clients_connected }} {{ connectionStats.clients_connected === 1 ? 'client' : 'clients' }} connected</div>
              <div class="connection-stats-bandwidth">
                <span class="connection-stats-label">This</span>
                <v-icon
                  icon="mdi-arrow-up"
                  size="12"
                  class="mr-1"
                />
                {{ formatKbps(connectionStats.this_upload_kbps) }}
                <v-icon
                  icon="mdi-arrow-down"
                  size="12"
                  class="mx-1"
                />
                {{ formatKbps(connectionStats.this_download_kbps) }} kbps
              </div>
              <div class="connection-stats-bandwidth">
                <span class="connection-stats-label">All</span>
                <v-icon
                  icon="mdi-arrow-up"
                  size="12"
                  class="mr-1"
                />
                {{ formatKbps(connectionStats.total_upload_kbps) }}
                <v-icon
                  icon="mdi-arrow-down"
                  size="12"
                  class="mx-1"
                />
                {{ formatKbps(connectionStats.total_download_kbps) }} kbps
              </div>
            </template>
          </div>
        </v-tooltip>
        <BlueButtonGroup
          :button-items="configButtons"
          :theme="theme"
          type="switch"
        />
      </div>
    </div>
    <div class="health-banners-sticky">
      <v-alert
        v-if="showStaleBundleBanner"
        type="info"
        variant="flat"
        density="compact"
        class="mx-6 mt-4 stale-bundle-banner"
        theme="dark"
      >
        <div class="flex items-center justify-between gap-3 text-sm">
          <span>
            The backend was updated while this page was open. Reload to load the matching UI.
          </span>
          <v-btn
            size="small"
            variant="elevated"
            theme="dark"
            class="shrink-0 py-1 px-3 rounded-md bg-[#414141] hover:bg-[#0A3E6B]"
            @click="reloadPage"
          >
            Reload
          </v-btn>
        </div>
      </v-alert>
      <v-alert
        v-if="showDegradedBanner"
        type="warning"
        variant="flat"
        density="compact"
        class="mx-6 mt-4 cursor-pointer health-focusable health-degraded-banner"
        theme="dark"
        role="button"
        tabindex="0"
        @click="onDegradedBannerOpen"
        @keydown.enter="onDegradedBannerOpen"
        @keydown.space.prevent="onDegradedBannerOpen"
      >
        <div class="text-sm font-medium">
          {{ degradedBannerTitle }}
        </div>
      </v-alert>
    </div>
    <div
      class="min-w-[650px] transition-all duration-300 ease-in-out"
    >
      <div
        v-if="discoveryEmpty"
        class="px-6 py-8 text-center text-sm opacity-70"
      >
        No cameras discovered yet. Connect a RadCam and it will appear here.
      </div>
      <div v-if="configMode === 'basic'">
        <BasicSettings
          ref="cameraControls"
          :selected-camera-uuid="selectedCameraUUID"
          :disabled="baseControlsDisabled"
          :camera-controls-disabled="cameraBackedControlsDisabled"
          :loading="uiLoading"
          :cockpit-mode="isCockpitMode"
          :one-push-awb="onePushAwb"
          :backend-connected="backendConnected"
          :welcome-overlay-unblocked="welcomeOverlayUnblocked"
        />
      </div>
      <div v-if="configMode === 'advanced'">
        <v-tabs
          v-model="tab"
          align-tabs="center"
          class="mb-5"
        >
          <v-tab value="image">
            Image
          </v-tab>
          <v-tab value="streams">
            Streams
          </v-tab>
          <v-tab
            value="configs"
            :disabled="true"
          >
            Configs
          </v-tab>
        </v-tabs>

        <v-tabs-window v-model="tab">
          <v-tabs-window-item value="image">
            <ImageTab
              :selected-camera-uuid="selectedCameraUUID"
              :disabled="cameraBackedControlsDisabled"
              :one-push-awb="onePushAwb"
            />
          </v-tabs-window-item>
          <v-tabs-window-item value="streams">
            <StreamsTab
              :selected-camera-uuid="selectedCameraUUID"
              :disabled="cameraBackedControlsDisabled"
            />
          </v-tabs-window-item>
        </v-tabs-window>
      </div>
      <HealthDiagnostics
        :system-health="systemHealth"
        :camera-connectivity="cameraConnectivity"
        :problem-titles="healthProblems.map((problem) => problem.title)"
      />
    </div>
  </v-container>
  <SystemStatusDialog
    :show="showSystemStatusDialog"
    :awaiting-close="healthDialogAwaitingClose"
    :recovery-title="healthDialogRecoveryTitle"
    :recovery-message="healthDialogRecoveryMessage"
    :problems="healthProblems"
    :system-health="systemHealth"
    :camera-connectivity="cameraConnectivity"
    :camera-uuid="selectedCameraUUID"
    :connection-state="dialogConnectionState"
    :ever-connected="connectionPhaseEverConnected"
    :busy="busyDialogState"
    @close="onHealthDialogClose"
    @minimize="onSystemStatusDialogMinimize"
    @forgotten="onHealthCameraForgotten"
    @go-to-setup="onHealthGoToSetup"
  />
  <ErrorDialog
    :message="errorDialogMessage"
    @close="dismissErrorDialog"
  />
  <WarningToast
    :message="warningToastMessage"
    :icon="warningToastIcon"
  />
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRouteQuery } from '@vueuse/router'

import type { Camera } from '@/bindings/mcm_client'
import type {
  CameraConnectivity,
  CameraStateEvent,
  CameraUiState,
  OnePushAwbStatus,
} from '@/bindings/radcam_api'
import HealthDiagnostics from '@/components/HealthDiagnostics.vue'
import BasicSettings from '@/components/BasicSettings.vue'
import BlueButtonGroup from '@/components/BlueButtonGroup.vue'
import ImageTab from '@/components/ImageTab.vue'
import StreamsTab from '@/components/StreamsTab.vue'
import SystemStatusDialog from '@/components/SystemStatusDialog.vue'
import WarningToast from '@/components/WarningToast.vue'
import {
  closeHealthDialog,
  degradedBannerCopy,
  enrichHealthProblems,
  evaluateHealthFlags,
  healthDialogStateOnDisconnect,
  healthDialogView,
  initialHealthDialogState,
  minimizeHealthDialog,
  noteActiveProblems,
  noteForgetSuccess,
  recoveryWhileMinimizedToast,
  reduceHealthDialogOnProblems,
  reopenHealthDialog,
  type HealthDialogState,
} from '@/utils/healthDialogState'
import { backendClient, type ConnectionState, type ConnectionStats } from '@/utils/backendClient'
import { formatRequestError } from '@/utils/formatRequestError'
import { useSystemHealth } from '@/utils/useSystemHealth'

type CameraOption = {
  uuid: string
  label: string
  missing: boolean
}

/** Minimum time the connecting/reconnecting dialog stays up, so it never just flashes. */
const MIN_CONNECTION_PHASE_MS = 1000

const tab = ref(null)
const cameras = ref<Camera[]>([])
const selectedCameraUUID = ref<string | null>(null)
/** Last known labels so a selected camera stays visible across discovery flaps. */
const cameraLabelByUuid = ref<Record<string, string>>({})
const {
  systemHealth,
  discoveryEmpty,
  expectedMissing,
} = useSystemHealth()
const healthDialog = ref<HealthDialogState>(initialHealthDialogState())
const healthProblemsNowMs = ref(Date.now())
const cameraConnectivity = ref<CameraConnectivity>('unknown')
const cameraStreamError = ref<string | null>(null)
const cameraOnvifAuthError = ref<string | null>(null)
const connectionState = ref<ConnectionState>('connecting')
let everConnected = false
/** Connection state the dialog renders, held past reconnect so it cannot flash by. */
const connectionPhase = ref<ConnectionState | null>('connecting')
/** `everConnected` as of when the current connection phase began, so its copy is stable. */
const connectionPhaseEverConnected = ref(false)
let connectionPhaseSince = Date.now()
let connectionPhaseTimer: ReturnType<typeof setTimeout> | null = null
const connectionStats = ref<ConnectionStats | null>(null)
const disconnectedSince = ref<Date | null>(null)

const sinceFormatter = new Intl.DateTimeFormat(undefined, {
  dateStyle: 'medium',
  timeStyle: 'short',
})

const formatSince = (value: string | Date): string => {
  const date = value instanceof Date ? value : new Date(value)
  return sinceFormatter.format(date)
}

const formatKbps = (kbps: number): string => {
  if (kbps < 10) {
    return kbps.toFixed(1)
  }
  return Math.round(kbps).toString()
}

const connectionStatusLine = computed(() => {
  if (connectionState.value === 'connecting') {
    return 'Connecting…'
  }
  if (connectionState.value === 'connected' && connectionStats.value) {
    return `Connected since ${formatSince(connectionStats.value.since)}`
  }
  if (connectionState.value === 'disconnected' && disconnectedSince.value) {
    return `Disconnected since ${formatSince(disconnectedSince.value)}`
  }
  return 'Disconnected'
})

const connectionIcon = computed(() => {
  switch (connectionState.value) {
    case 'connected':
      return 'mdi-lan-connect'
    case 'connecting':
      return 'mdi-sync'
    case 'disconnected':
    default:
      return 'mdi-lan-disconnect'
  }
})

const connectionColor = computed(() => {
  switch (connectionState.value) {
    case 'connected':
      return '#66bb6a'
    case 'connecting':
      return '#ffb74d'
    case 'disconnected':
    default:
      return '#ef5350'
  }
})

const backendConnected = computed(() => connectionState.value === 'connected')

const cameraOptions = computed((): CameraOption[] => {
  const options: CameraOption[] = cameras.value.map((camera) => ({
    uuid: camera.uuid,
    label: camera.hostname,
    missing: false,
  }))
  const listed = new Set(cameras.value.map((camera) => camera.uuid))
  for (const ghost of expectedMissing.value) {
    if (!listed.has(ghost.uuid)) {
      options.push({
        uuid: ghost.uuid,
        label:
          ghost.last_hostname
          ?? cameraLabelByUuid.value[ghost.uuid]
          ?? `Camera ${ghost.uuid.slice(0, 8)}`,
        missing: true,
      })
      listed.add(ghost.uuid)
    }
  }
  // Keep the current selection visible across brief list/health races.
  const selected = selectedCameraUUID.value
  if (selected && !listed.has(selected)) {
    options.push({
      uuid: selected,
      label: cameraLabelByUuid.value[selected] ?? `Camera ${selected.slice(0, 8)}`,
      missing: !cameras.value.some((camera) => camera.uuid === selected),
    })
  }
  return options
})

const selectedCameraLabel = computed(() => {
  if (!selectedCameraUUID.value) return ''
  const option = cameraOptions.value.find((item) => item.uuid === selectedCameraUUID.value)
  if (option) return option.label
  const camera = cameras.value.find((item) => item.uuid === selectedCameraUUID.value)
  return camera?.hostname ?? `Camera ${selectedCameraUUID.value.slice(0, 8)}`
})

const cameraOnline = computed(
  () => cameraConnectivity.value === 'online' || cameraConnectivity.value === 'unknown',
)

const baseControlsDisabled = computed(
  () =>
    !backendConnected.value
    || selectedCameraUUID.value == null
    || uiLoading.value
    || uiRebooting.value,
)

const cameraBackedControlsDisabled = computed(
  () => baseControlsDisabled.value || !cameraOnline.value,
)

const healthInputBase = computed(() => ({
  systemHealth: systemHealth.value,
  cameraUuid: selectedCameraUUID.value,
  cameraLabel: selectedCameraLabel.value,
  cameraConnectivity: cameraConnectivity.value,
  cameraStreamError: cameraStreamError.value,
  cameraOnvifAuthError: cameraOnvifAuthError.value,
  cameraExpectedMissing: expectedMissing.value.some(
    (camera) => camera.uuid === selectedCameraUUID.value,
  ),
}))

const healthFlags = computed(() =>
  evaluateHealthFlags({
    ...healthInputBase.value,
    problemFirstSeen: healthDialog.value.problemFirstSeen,
    nowMs: healthProblemsNowMs.value,
  }),
)
const healthProblems = computed(() =>
  enrichHealthProblems(
    healthFlags.value.problems,
    healthDialog.value.problemFirstSeen,
    healthProblemsNowMs.value,
  ),
)
const healthView = computed(() =>
  healthDialogView(healthDialog.value, healthFlags.value.degraded),
)
const showHealthDialog = computed(() => healthView.value.showDialog)
/** A deliberate long-running action shows in the dialog until the user minimizes it. */
const busyDialogState = computed(() =>
  uiLoading.value && !busyMinimized.value
    ? { message: uiLoadingMessage.value, rebooting: uiRebooting.value }
    : null,
)
const showSystemStatusDialog = computed(
  () => connectionPhase.value != null || busyDialogState.value != null || showHealthDialog.value,
)
const dialogConnectionState = computed(() => connectionPhase.value ?? connectionState.value)
const welcomeOverlayUnblocked = computed(
  () =>
    backendConnected.value
    && !uiRebooting.value
    && !showSystemStatusDialog.value
    && errorDialogMessage.value == null,
)
const showDegradedBanner = computed(() => healthView.value.showDegradedBanner)
const showHealthChip = computed(() => healthFlags.value.degraded)
const showBusyChip = computed(() => uiLoading.value && busyMinimized.value)
const healthDialogAwaitingClose = computed(
  () => connectionState.value === 'connected' && healthView.value.awaitingClose,
)
const healthDialogRecoveryTitle = computed(() => healthView.value.recoveryTitle)
const healthDialogRecoveryMessage = computed(() => healthView.value.recoveryMessage)
const degradedBanner = computed(() => degradedBannerCopy(healthProblems.value))
const degradedBannerTitle = computed(() => degradedBanner.value.title)
const degradedBannerTooltip = computed(() => {
  const hint = showDegradedBanner.value
    ? 'Click to reopen system status.'
    : 'System status is open.'
  return `${degradedBanner.value.body} ${hint}`
})

const desiredCameraUuid = useRouteQuery<string | null>('uuid', null)

// Auto-pick over the options, not the MCM list, so a configured camera that discovery has
// not listed yet is still selected: it answers its own HTTP API regardless. Never replaces
// an existing selection, so a camera leaving discovery does not steal the user's choice.
watch(
  cameraOptions,
  (options) => {
    if (selectedCameraUUID.value || options.length === 0) return
    const desired = desiredCameraUuid.value
      ? options.find((option) => option.uuid === desiredCameraUuid.value)
      : null
    selectedCameraUUID.value = (desired ?? options[0]).uuid
  },
  { immediate: true },
)

const theme = ref<'light' | 'dark'>('dark')
const configMode = ref<'basic' | 'advanced'>('basic')
const cameraControls = ref<InstanceType<typeof BasicSettings> | null>(null)
const uiLoading = ref(false)
const uiLoadingMessage = ref('Applying settings…')
const uiRebooting = ref(false)
/** Local, per-episode: a new action always shows the dialog again. */
const busyMinimized = ref(false)
const onePushAwb = ref<OnePushAwbStatus | null>(null)
const errorDialogMessage = ref<string | null>(null)
const WARNING_TOAST_ICON = 'mdi-alert-circle-outline'
const RECOVERY_TOAST_ICON = 'mdi-check-circle-outline'
const warningToastMessage = ref<string | null>(null)
const warningToastIcon = ref(WARNING_TOAST_ICON)
const showStaleBundleBanner = ref(false)
const isCockpitMode = useRouteQuery<string, boolean>('cockpit_mode', 'false', {
  transform: {
    get: (v: string) => v === 'true',
    set: (v: boolean) => String(v),
  },
})

const configButtons = [
  {
    name: 'Basic',
    tooltip: 'Basic setup for the RadCam',
    onSelected: () => (configMode.value = 'basic'),
    preSelected: true,
  },
  {
    name: 'Advanced',
    tooltip: 'Advanced camera settings',
    onSelected: () => (configMode.value = 'advanced'),
  },
]

const applyCameraUi = (ui: CameraUiState) => {
  uiLoading.value = ui.loading
  // Keep the last message while fading out — clearing it to the default
  // ("Applying settings…") mid-transition looks like a glitch.
  if (ui.loading_message) {
    uiLoadingMessage.value = ui.loading_message
  }
  uiRebooting.value = ui.rebooting
  onePushAwb.value = ui.one_push_awb ?? null
  errorDialogMessage.value = ui.error_dialog ?? null
  warningToastIcon.value = WARNING_TOAST_ICON
  warningToastMessage.value = ui.warning_toast ?? null
  // A backend too old to send connectivity must not gray out every camera control:
  // treat an absent field as 'unknown', which keeps them usable.
  cameraConnectivity.value = ui.connectivity ?? 'unknown'
  cameraStreamError.value = ui.stream_error ?? null
  cameraOnvifAuthError.value = ui.onvif_auth_error ?? null
}

const uiByCamera = new Map<string, CameraUiState>()

const applyCameraState = (body: unknown) => {
  if (typeof body !== 'object' || body === null) return
  const data = body as CameraStateEvent
  if (data.ui) {
    uiByCamera.set(data.camera_uuid, data.ui)
    if (data.camera_uuid === selectedCameraUUID.value) {
      applyCameraUi(data.ui)
    }
  }
}

watch(selectedCameraUUID, (uuid, previousUuid) => {
  if (previousUuid) {
    backendClient.unsubscribeCamera(previousUuid)
  }

  if (!uuid) {
    uiLoading.value = false
    uiRebooting.value = false
    onePushAwb.value = null
    errorDialogMessage.value = null
    warningToastMessage.value = null
    cameraConnectivity.value = 'unknown'
    cameraStreamError.value = null
    cameraOnvifAuthError.value = null
    return
  }

  backendClient.subscribeCamera(uuid)

  const ui = uiByCamera.get(uuid)
  if (ui) {
    // Don't resurrect stale loading/rebooting overlays after unsubscribe.
    applyCameraUi({
      ...ui,
      loading: false,
      loading_message: undefined,
      rebooting: false,
    })
  } else {
    uiLoading.value = false
    uiRebooting.value = false
    onePushAwb.value = null
    errorDialogMessage.value = null
    warningToastMessage.value = null
    cameraConnectivity.value = 'unknown'
    cameraStreamError.value = null
    cameraOnvifAuthError.value = null
  }
})

// Remounted Basic/Advanced tabs start empty; re-subscribe so the backend re-pushes cache.
watch([configMode, tab], () => {
  if (selectedCameraUUID.value) {
    backendClient.refreshCameraSubscription()
  }
})

const dismissErrorDialog = () => {
  if (selectedCameraUUID.value) {
    backendClient.dismissUi(selectedCameraUUID.value, 'error_dialog')
  }
  errorDialogMessage.value = null
}

const reloadPage = (): void => {
  window.location.reload()
}

const applyCameraList = (data: unknown) => {
  try {
    const camerasData = validateCameras(data)
    cameras.value = camerasData

    const labels = { ...cameraLabelByUuid.value }
    for (const camera of camerasData) {
      labels[camera.uuid] = camera.hostname
    }
    for (const ghost of expectedMissing.value) {
      if (ghost.last_hostname) {
        labels[ghost.uuid] = ghost.last_hostname
      }
    }
    cameraLabelByUuid.value = labels
  } catch (error) {
    console.error('Error processing cameras:', error)
  }
}

const onCameraForgotten = (cameraUuid: string): void => {
  const labels = { ...cameraLabelByUuid.value }
  delete labels[cameraUuid]
  cameraLabelByUuid.value = labels

  if (selectedCameraUUID.value === cameraUuid) {
    selectedCameraUUID.value = null
  }

  if (!selectedCameraUUID.value && cameras.value.length > 0) {
    selectedCameraUUID.value = cameras.value[0].uuid
  }
}

const onSystemStatusDialogMinimize = (): void => {
  // One click must clear the dialog even when an action and a health problem overlap,
  // which happens when the reboot was started from inside this dialog.
  if (busyDialogState.value) {
    busyMinimized.value = true
  }
  if (showHealthDialog.value) {
    healthDialog.value = minimizeHealthDialog(healthDialog.value)
  }
}

const onHealthDialogClose = (): void => {
  healthDialog.value = closeHealthDialog()
}

const onDegradedBannerOpen = (): void => {
  healthDialog.value = reopenHealthDialog(healthDialog.value)
}

const onBusyChipOpen = (): void => {
  busyMinimized.value = false
}

const onHealthCameraForgotten = (cameraUuid: string): void => {
  healthDialog.value = noteForgetSuccess(healthDialog.value)
  onCameraForgotten(cameraUuid)
}

const onHealthGoToSetup = async (): Promise<void> => {
  healthDialog.value = minimizeHealthDialog(healthDialog.value)
  configMode.value = 'basic'
  await nextTick()
  cameraControls.value?.scrollToHardwareSetup()
}

const validateCameras = (data: unknown): Camera[] => {
  if (typeof data !== 'object' || data === null) {
    throw new Error('Expected a map of { uuid: camera }')
  }

  const cameras: Camera[] = []
  for (const [uuid, cameraData] of Object.entries(data)) {
    if (isCamera(cameraData)) {
      cameras.push({ ...cameraData, uuid })
    }
  }
  return cameras
}

const isCamera = (data: unknown): data is Omit<Camera, 'uuid'> => {
  if (typeof data !== 'object' || data === null) return false

  const camera = data as Record<string, unknown>

  const isStreamsValid =
    typeof camera.streams === 'object' &&
    camera.streams !== null &&
    Object.values(camera.streams).every((stream) => typeof stream === 'string')

  return typeof camera.hostname === 'string' && isStreamsValid
}

const updateLuaScript = (): void => {
  if (!selectedCameraUUID.value) return

  if (cameraControls.value) {
    cameraControls.value.updateLuaScript()
    return
  }

  runAutopilotControl('exportLuaScript', 'Failed to update Lua script')
}

const applyRecommendedCameraSettings = (): void => {
  if (cameraControls.value) {
    cameraControls.value.applyRecommendedCameraSettings()
    return
  }

  runCameraControl('setRecommendedCameraSettings', 'Failed to apply recommended camera settings')
}

const rebootCamera = (): void => {
  if (!selectedCameraUUID.value) return

  if (cameraControls.value) {
    cameraControls.value.rebootCamera()
    return
  }

  runCameraControl('restart', 'Failed to reboot camera')
}

const runAutopilotControl = (action: string, errorMessage: string): void => {
  const cameraUuid = selectedCameraUUID.value
  if (!cameraUuid || uiLoading.value || uiRebooting.value) return

  backendClient
    .request('POST', '/autopilot/control', {
      camera_uuid: cameraUuid,
      action,
    })
    .then((data) => {
      if (selectedCameraUUID.value !== cameraUuid) return
      console.log(data)
    })
    .catch((error) => {
      if (selectedCameraUUID.value !== cameraUuid) return
      warningToastMessage.value = `${errorMessage}: ${formatRequestError(error)}`
    })
}

const runCameraControl = (action: string, errorMessage: string): void => {
  const cameraUuid = selectedCameraUUID.value
  if (!cameraUuid || uiLoading.value || uiRebooting.value) return

  backendClient
    .request('POST', '/camera/control', {
      camera_uuid: cameraUuid,
      action,
    })
    .then((data) => {
      if (selectedCameraUUID.value !== cameraUuid) return
      console.log(data)
    })
    .catch((error) => {
      if (selectedCameraUUID.value !== cameraUuid) return
      warningToastMessage.value = `${errorMessage}: ${formatRequestError(error)}`
    })
}

const applyConnectionStats = (body: unknown) => {
  if (typeof body !== 'object' || body === null) return
  connectionStats.value = body as ConnectionStats
}

const unsubscribeCameraList = backendClient.onEvent('camera/list', applyCameraList)
const unsubscribeCameraState = backendClient.onEvent('camera/state', applyCameraState)
const unsubscribeConnectionStats = backendClient.onEvent('connection/stats', applyConnectionStats)
const unsubscribeTransportError = backendClient.onTransportError((message) => {
  // Local-only toast; do not call dismissUi (that would clear shared backend warnings).
  warningToastMessage.value = message
})
const unsubscribeBackendVersionChanged = backendClient.onBackendVersionChanged(() => {
  showStaleBundleBanner.value = true
})
const unsubscribeConnectionState = backendClient.onConnectionState((state, previousState) => {
  if (state === 'disconnected' && previousState !== 'disconnected') {
    disconnectedSince.value = new Date()
    connectionStats.value = null
    uiByCamera.clear()
    uiLoading.value = false
    uiRebooting.value = false
    errorDialogMessage.value = null
    warningToastMessage.value = null
    cameraConnectivity.value = 'unknown'
    cameraStreamError.value = null
    cameraOnvifAuthError.value = null
    healthDialog.value = healthDialogStateOnDisconnect(healthDialog.value)
  }
  if (state === 'connected') {
    disconnectedSince.value = null
    everConnected = true
    const remaining = MIN_CONNECTION_PHASE_MS - (Date.now() - connectionPhaseSince)
    if (remaining <= 0) {
      connectionPhase.value = null
    } else if (connectionPhaseTimer == null) {
      connectionPhaseTimer = setTimeout(() => {
        connectionPhase.value = null
        connectionPhaseTimer = null
      }, remaining)
    }
  } else {
    if (connectionPhaseTimer != null) {
      clearTimeout(connectionPhaseTimer)
      connectionPhaseTimer = null
    }
    if (connectionPhase.value == null) {
      connectionPhaseSince = Date.now()
      connectionPhaseEverConnected.value = everConnected
    }
    connectionPhase.value = state
  }
  connectionState.value = state
})

watch(
  healthInputBase,
  () => {
    const flags = healthFlags.value

    const before = healthDialog.value
    let next = before
    next = noteActiveProblems(next, flags.problems, Date.now())
    next = reduceHealthDialogOnProblems(next, flags.degraded)
    if (
      before.mode === 'minimized'
      && before.episodeDegraded
      && !flags.degraded
      && next.mode === 'hidden'
    ) {
      warningToastIcon.value = RECOVERY_TOAST_ICON
      warningToastMessage.value = recoveryWhileMinimizedToast(before)
    }
    if (next !== before) {
      healthDialog.value = next
    }
  },
)

watch(uiLoading, (loading) => {
  if (!loading) {
    busyMinimized.value = false
  }
})

const needsHealthProblemsNowTick = computed(() => healthFlags.value.degraded)

let healthProblemsNowInterval: ReturnType<typeof setInterval> | null = null

const stopHealthProblemsNowTick = (): void => {
  if (healthProblemsNowInterval == null) return
  clearInterval(healthProblemsNowInterval)
  healthProblemsNowInterval = null
}

const startHealthProblemsNowTick = (): void => {
  if (healthProblemsNowInterval != null) return
  healthProblemsNowMs.value = Date.now()
  healthProblemsNowInterval = setInterval(() => {
    healthProblemsNowMs.value = Date.now()
  }, 5000)
}

watch(needsHealthProblemsNowTick, (needs) => {
  if (needs) {
    startHealthProblemsNowTick()
  } else {
    stopHealthProblemsNowTick()
  }
}, { immediate: true })

onMounted(() => {
  backendClient.connect().catch((error) => {
    console.error('Error connecting to backend:', error)
  })
})

watch(warningToastMessage, (message, _previous, onCleanup) => {
  if (!message) {
    warningToastIcon.value = WARNING_TOAST_ICON
    return
  }

  const cameraUuid = selectedCameraUUID.value
  const cachedToast =
    cameraUuid != null ? uiByCamera.get(cameraUuid)?.warning_toast ?? null : null
  // Only auto-dismissUi when the displayed text is the backend-owned toast.
  const backendOwned = cachedToast != null && cachedToast === message
  const timeout = setTimeout(() => {
    if (backendOwned && cameraUuid) {
      backendClient.dismissUi(cameraUuid, 'warning_toast')
    }
    warningToastMessage.value = null
  }, 5000)

  onCleanup(() => clearTimeout(timeout))
})

onUnmounted(() => {
  stopHealthProblemsNowTick()
  if (connectionPhaseTimer != null) {
    clearTimeout(connectionPhaseTimer)
    connectionPhaseTimer = null
  }
  if (selectedCameraUUID.value) {
    backendClient.unsubscribeCamera(selectedCameraUUID.value)
  }
  unsubscribeCameraList()
  unsubscribeCameraState()
  unsubscribeConnectionStats()
  unsubscribeTransportError()
  unsubscribeBackendVersionChanged()
  unsubscribeConnectionState()
})


</script>
<style scoped>
.connection-status-wrap {
  display: inline-flex;
  align-items: center;
  padding: 0 12px;
  cursor: default;
}

.connection-stats-tooltip {
  max-width: 280px;
}

.connection-stats-bandwidth {
  display: flex;
  align-items: center;
}

.connection-stats-label {
  display: inline-block;
  min-width: 2.5rem;
  margin-right: 0.35rem;
  opacity: 0.7;
}

.connection-status-icon {
  opacity: 0.55;
}

.connection-status-icon--spinning {
  animation: connection-status-spin 1.5s linear infinite;
}

@keyframes connection-status-spin {
  from {
    transform: rotate(0deg);
  }

  to {
    transform: rotate(360deg);
  }
}

.transparent-card {
  background-color: #10101085;
  backdrop-filter: blur(25px);
  border: 1px solid rgba(255, 255, 255, 0.08);
  box-shadow: 0px 4px 4px 0px #00000033, 0px 8px 12px 6px #00000026;
}

.health-banners-sticky {
  position: sticky;
  top: 0;
  z-index: 3;
}

.health-degraded-banner {
  background-color: #5c4a12 !important;
  color: #ffe082 !important;
  border: 1px solid #c9a22788;
  opacity: 1;
}

.stale-bundle-banner {
  background-color: #1a3a52 !important;
  color: #b3e5fc !important;
  border: 1px solid #4fc3f788;
  opacity: 1;
}

.health-focusable:focus-visible {
  outline: 2px solid #9ec9ef;
  outline-offset: 2px;
}
</style>
