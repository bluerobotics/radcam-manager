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
              <v-list-item @click="updateLuaScript">
                <v-list-item-title class="flex">
                  Update Lua script
                </v-list-item-title>
              </v-list-item>
              <v-list-item @click="applyRecommendedCameraSettings">
                <v-list-item-title class="flex">
                  Apply Recommended Camera Settings
                </v-list-item-title>
              </v-list-item>
              <v-list-item
                :disabled="selectedCameraUUID == null"
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
        <BlueButtonGroup
          :button-items="configButtons"
          :theme="theme"
          class="mr-4"
          type="switch"
        />
      </div>
      <div
        class="min-w-[650px] transition-all duration-300 ease-in-out"
      >
        <div v-if="configMode === 'basic'">
          <BasicSettings
            ref="cameraControls"
            :selected-camera-uuid="selectedCameraUUID"
            :backend-api="backendAPI"
            :disabled="selectedCameraUUID == null"
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
                :backend-api="backendAPI"
                :selected-camera-uuid="selectedCameraUUID"
                :disabled="selectedCameraUUID == null"
              />
            </v-tabs-window-item>
            <v-tabs-window-item value="streams">
              <StreamsTab
                :backend-api="backendAPI"
                :selected-camera-uuid="selectedCameraUUID"
                :disabled="selectedCameraUUID == null"
              />
            </v-tabs-window-item>
          </v-tabs-window>
        </div>
      </div>
    </v-container>
  <Loading
    :is-loading="menuActionLoading"
    :message="menuActionMessage"
  />
  <ErrorDialog
    :message="errorDialogMessage"
    @close="errorDialogMessage = null"
  />
</template>

<script setup lang="ts">
import axios from 'axios'
import { onMounted, onUnmounted, ref } from 'vue'
import { useRouteQuery } from '@vueuse/router'

import type { Camera } from '@/bindings/mcm_client'
import BasicSettings from '@/components/BasicSettings.vue'
import BlueButtonGroup from '@/components/BlueButtonGroup.vue'
import ImageTab from '@/components/ImageTab.vue'
import StreamsTab from '@/components/StreamsTab.vue'
import { formatRequestError } from '@/utils/formatRequestError'
import { endMinLoading, startMinLoading } from '@/utils/minLoadingDuration'

const tab = ref(null)
// const backendAPI = ref(`http://192.168.2.2:<radcam-extension-port>/v1`) // For local frontend development:
const backendAPI = ref('v1')
const cameras = ref<Camera[]>([])
const selectedCameraUUID = ref<string | null>(null)

const desiredCameraUuid = useRouteQuery<string | null>('uuid', null)

const theme = ref<'light' | 'dark'>('dark')
const configMode = ref<'basic' | 'advanced'>('basic')
const cameraControls = ref<InstanceType<typeof BasicSettings> | null>(null)
const menuActionLoading = ref(false)
const menuActionMessage = ref('Applying settings…')
const errorDialogMessage = ref<string | null>(null)
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

const getCameras = async () => {
  try {
    const response = await axios.get(`${backendAPI.value}/camera/list`)
    const camerasData = validateCameras(response.data)
    cameras.value = camerasData

    if (!selectedCameraUUID.value && cameras.value.length > 0) {
      const foundCamera = desiredCameraUuid.value
        ? cameras.value.find((camera) => camera.uuid === desiredCameraUuid.value)
        : null

      selectedCameraUUID.value = foundCamera ? foundCamera.uuid : cameras.value[0].uuid
    }
  } catch (error) {
    console.error('Error getting cameras:', error)
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

  return (
    typeof camera.hostname === 'string' &&
    typeof camera.credentials === 'object' &&
    camera.credentials !== null &&
    typeof (camera.credentials as Record<string, unknown>).username === 'string' &&
    typeof (camera.credentials as Record<string, unknown>).password === 'string' &&
    isStreamsValid
  )
}

const updateLuaScript = (): void => {
  if (!selectedCameraUUID.value) return

  if (cameraControls.value) {
    cameraControls.value.updateLuaScript()
    return
  }

  runAutopilotControl('exportLuaScript', 'Failed to update Lua script', 'Updating Lua script…')
}

const applyRecommendedCameraSettings = (): void => {
  if (cameraControls.value) {
    cameraControls.value.applyRecommendedCameraSettings()
    return
  }

  runCameraControl('setRecommendedCameraSettings', 'Failed to apply recommended camera settings', 'Applying recommended camera settings…')
}

const rebootCamera = (): void => {
  if (!selectedCameraUUID.value) return

  if (cameraControls.value) {
    cameraControls.value.rebootCamera()
    return
  }

  runCameraControl('restart', 'Failed to reboot camera', 'Rebooting camera…')
}

const runAutopilotControl = (action: string, errorMessage: string, loadingMessage: string): void => {
  if (!selectedCameraUUID.value || menuActionLoading.value) return

  menuActionMessage.value = loadingMessage
  const startedAt = startMinLoading(menuActionLoading)

  axios
    .post(`${backendAPI.value}/autopilot/control`, {
      camera_uuid: selectedCameraUUID.value,
      action,
    })
    .then((response) => {
      console.log(response)
      endMinLoading(menuActionLoading, startedAt)
    })
    .catch((error) => {
      errorDialogMessage.value = `${errorMessage}: ${formatRequestError(error)}`
      endMinLoading(menuActionLoading, startedAt, true)
    })
}

const runCameraControl = (action: string, errorMessage: string, loadingMessage: string): void => {
  if (!selectedCameraUUID.value || menuActionLoading.value) return

  menuActionMessage.value = loadingMessage
  const startedAt = startMinLoading(menuActionLoading)

  axios
    .post(`${backendAPI.value}/camera/control`, {
      camera_uuid: selectedCameraUUID.value,
      action,
    })
    .then((response) => {
      console.log(response)
      endMinLoading(menuActionLoading, startedAt)
    })
    .catch((error) => {
      errorDialogMessage.value = `${errorMessage}: ${formatRequestError(error)}`
      endMinLoading(menuActionLoading, startedAt, true)
    })
}

onMounted(() => {
  getCameras()

  const intervalId = setInterval(() => {
    getCameras()
  }, 5000)

  onUnmounted(() => {
    clearInterval(intervalId)
  })
})


</script>
<style scoped>
.transparent-card {
  background-color: #10101085;
  backdrop-filter: blur(25px);
  border: 1px solid rgba(255, 255, 255, 0.08);
  box-shadow: 0px 4px 4px 0px #00000033, 0px 8px 12px 6px #00000026;
}
</style>
