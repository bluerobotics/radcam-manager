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
                :disabled="!backendConnected || selectedCameraUUID == null"
                @click="applyRecommendedCameraSettings"
              >
                <v-list-item-title class="flex">
                  Apply Recommended Camera Settings
                </v-list-item-title>
              </v-list-item>
              <v-list-item
                :disabled="!backendConnected || selectedCameraUUID == null"
                @click="rebootCamera"
              >
                <v-list-item-title class="flex">
                  Reboot Camera
                </v-list-item-title>
              </v-list-item>
              <v-divider />
              <v-divider />
            </v-list>
          </v-menu>
          <v-select
            v-model="selectedCameraUUID"
            :items="cameras"
            item-title="hostname"
            item-value="uuid"
            label="Camera"
            hide-details
            theme="dark"
            class="bg-[#15151577] ml-3 -mb-[1px]"
          >
            <template #item="{ props, item }">
              <v-list-item
                v-bind="props"
                :subtitle="item.raw.uuid"
              />
            </template>
          </v-select>
        </div>
        <div class="flex items-center mr-4">
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
      <div
        class="min-w-[650px] transition-all duration-300 ease-in-out"
      >
        <div v-if="configMode === 'basic'">
          <BasicSettings
            ref="cameraControls"
            :selected-camera-uuid="selectedCameraUUID"
            :disabled="!backendConnected || selectedCameraUUID == null || uiLoading || uiRebooting"
            :loading="uiLoading"
            :cockpit-mode="isCockpitMode"
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
                :disabled="!backendConnected || selectedCameraUUID == null || uiLoading || uiRebooting"
              />
            </v-tabs-window-item>
            <v-tabs-window-item value="streams">
              <StreamsTab
                :selected-camera-uuid="selectedCameraUUID"
                :disabled="!backendConnected || selectedCameraUUID == null || uiLoading || uiRebooting"
              />
            </v-tabs-window-item>
          </v-tabs-window>
        </div>
      </div>
    </v-container>
  <Loading
    :is-loading="uiLoading"
    :message="uiLoadingMessage"
  />
  <ErrorDialog
    :message="errorDialogMessage"
    @close="dismissErrorDialog"
  />
  <WarningToast :message="warningToastMessage" />
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRouteQuery } from '@vueuse/router'

import type { Camera } from '@/bindings/mcm_client'
import type { CameraStateEvent, CameraUiState } from '@/bindings/radcam_api'
import BasicSettings from '@/components/BasicSettings.vue'
import BlueButtonGroup from '@/components/BlueButtonGroup.vue'
import ImageTab from '@/components/ImageTab.vue'
import StreamsTab from '@/components/StreamsTab.vue'
import WarningToast from '@/components/WarningToast.vue'
import { backendClient, type ConnectionState, type ConnectionStats } from '@/utils/backendClient'
import { formatRequestError } from '@/utils/formatRequestError'

const tab = ref(null)
const cameras = ref<Camera[]>([])
const selectedCameraUUID = ref<string | null>(null)
const connectionState = ref<ConnectionState>('disconnected')
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

const desiredCameraUuid = useRouteQuery<string | null>('uuid', null)

const theme = ref<'light' | 'dark'>('dark')
const configMode = ref<'basic' | 'advanced'>('basic')
const cameraControls = ref<InstanceType<typeof BasicSettings> | null>(null)
const uiLoading = ref(false)
const uiLoadingMessage = ref('Applying settings…')
const uiRebooting = ref(false)
const errorDialogMessage = ref<string | null>(null)
const warningToastMessage = ref<string | null>(null)
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
  errorDialogMessage.value = ui.error_dialog ?? null
  warningToastMessage.value = ui.warning_toast ?? null
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
    errorDialogMessage.value = null
    warningToastMessage.value = null
    return
  }

  backendClient.subscribeCamera(uuid)

  const ui = uiByCamera.get(uuid)
  if (ui) {
    // Don't resurrect stale loading/rebooting overlays after unsubscribe.
    applyCameraUi({
      ...ui,
      loading: false,
      loading_message: null,
      rebooting: false,
    })
  } else {
    uiLoading.value = false
    uiRebooting.value = false
    errorDialogMessage.value = null
    warningToastMessage.value = null
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

const applyCameraList = (data: unknown) => {
  try {
    const camerasData = validateCameras(data)
    cameras.value = camerasData

    if (
      selectedCameraUUID.value
      && !cameras.value.some((camera) => camera.uuid === selectedCameraUUID.value)
    ) {
      selectedCameraUUID.value = null
    }

    if (!selectedCameraUUID.value && cameras.value.length > 0) {
      const foundCamera = desiredCameraUuid.value
        ? cameras.value.find((camera) => camera.uuid === desiredCameraUuid.value)
        : null

      selectedCameraUUID.value = foundCamera ? foundCamera.uuid : cameras.value[0].uuid
    }
  } catch (error) {
    console.error('Error processing cameras:', error)
  }
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
const unsubscribeConnectionState = backendClient.onConnectionState((state, previousState) => {
  if (state === 'disconnected' && previousState !== 'disconnected') {
    disconnectedSince.value = new Date()
    connectionStats.value = null
    uiByCamera.clear()
    uiLoading.value = false
    uiRebooting.value = false
    errorDialogMessage.value = null
    warningToastMessage.value = null
  }
  if (state === 'connected') {
    disconnectedSince.value = null
  }
  connectionState.value = state
})

onMounted(() => {
  backendClient.connect().catch((error) => {
    console.error('Error connecting to backend:', error)
  })
})

watch(warningToastMessage, (message, _previous, onCleanup) => {
  if (!message) return

  const cameraUuid = selectedCameraUUID.value
  const backendOwned =
    cameraUuid != null && !!uiByCamera.get(cameraUuid)?.warning_toast
  const timeout = setTimeout(() => {
    if (backendOwned && cameraUuid) {
      backendClient.dismissUi(cameraUuid, 'warning_toast')
    }
    warningToastMessage.value = null
  }, 5000)

  onCleanup(() => clearTimeout(timeout))
})

onUnmounted(() => {
  if (selectedCameraUUID.value) {
    backendClient.unsubscribeCamera(selectedCameraUUID.value)
  }
  unsubscribeCameraList()
  unsubscribeCameraState()
  unsubscribeConnectionStats()
  unsubscribeTransportError()
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
</style>
