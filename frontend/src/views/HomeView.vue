<template>
  <div class="min-w-[650px] border-[1px] border-[#ffffff22] rounded-[8px]">
    <v-container
      no-gutters
      class="max-w-[800px] text-white pa-0  rounded-[8px] elevation-5 no-user-select"
      :class="[
        theme === 'dark' ? 'bg-[#363636]' : 'bg-[#F5F5F5]',
        {
          'transparent-card': isCockpitMode,
        },
      ]"
    >
      <div
        class="flex  items-center justify-between rounded-t-[8px]"
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
              <v-divider />
              <v-list-item @click="resetCameraSettings">
                <v-list-item-title class="flex">
                  Reset camera settings
                </v-list-item-title>
              </v-list-item>
              <v-divider />
              <v-list-item @click="isCockpitMode = !isCockpitMode">
                <v-list-item-title class="flex">
                  Cockpit mode
                </v-list-item-title>
              </v-list-item>
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
        class="transition-all duration-300 ease-in-out"
      >
        <div v-if="configMode === 'basic'">
          <BasicSettings
            ref="cameraControls"
            :selected-camera-uuid="selectedCameraUUID"
            :backend-api="backendAPI"
            :disabled="false"
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
  </div>
  <v-snackbar
    v-model="showSnackbar"
    :timeout="3000"
    color="green"
  >
    {{ snackbarMessage }}
  </v-snackbar>
</template>

<script setup lang="ts">
import axios from 'axios'
import { onMounted, onUnmounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import type { Camera } from '@/bindings/mcm_client'
import BasicSettings from '@/components/BasicSettings.vue'
import BlueButtonGroup from '@/components/BlueButtonGroup.vue'
import ImageTab from '@/components/ImageTab.vue'
import StreamsTab from '@/components/StreamsTab.vue'

const tab = ref(null)
// const backendAPI = ref(`http://192.168.2.2:<radcam-extension-port>/v1`) // For local frontend development:
const backendAPI = ref('v1')
const cameras = ref<Camera[]>([])
const selectedCameraUUID = ref<string | null>(null)

const route = useRoute()
const router = useRouter()
const desiredCameraUuid = ref<string | null>(null)

const theme = ref<'light' | 'dark'>('dark')
const configMode = ref<'basic' | 'advanced'>('basic')
const cameraControls = ref<InstanceType<typeof BasicSettings> | null>(null)
const showSnackbar = ref(false)
const snackbarMessage = ref('')
const isCockpitMode = ref(window.self !== window.top)

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

const refreshCameraStates = () => {
  cameraControls.value?.getInitialCameraStates()
}

const updateLuaScript = (): void => {
  if (!selectedCameraUUID.value) return

  const payload = {
    camera_uuid: selectedCameraUUID.value,
    action: 'exportLuaScript',
  }

  console.log(payload)

  axios
    .post(`${backendAPI.value}/autopilot/control`, payload)
    .then((response) => {
      console.log(`Lua script download initiated:`, response)
      snackbarMessage.value = `Lua script updated.`
      showSnackbar.value = true
    })
    .catch((error) => {
      console.error(`Error sending exportLuaScript request:`, error.message)
    })
}

const resetCameraSettings = (): void => {
  if (!selectedCameraUUID.value) return

  const payload = {
    camera_uuid: selectedCameraUUID.value,
    action: 'resetActuatorsConfig',
  }

  console.log(payload)

  axios
    .post(`${backendAPI.value}/autopilot/control`, payload)
    .then((response) => {
      refreshCameraStates()
      snackbarMessage.value = `Camera settings reset.`
      showSnackbar.value = true
      console.log(`Reset actuators config initiated:`, response)
    })
    .catch((error) => {
      console.error(`Error sending resetActuatorsConfig request:`, error.message)
    })
}

onMounted(() => {
  desiredCameraUuid.value = route.query.uuid ? (route.query.uuid as string) : null
  getCameras()

  const intervalId = setInterval(() => {
    getCameras()
  }, 5000)

  onUnmounted(() => {
    clearInterval(intervalId)
  })
})

watch(selectedCameraUUID, (newUUID) => {
  if (!newUUID) return

  // Avoid pushing the same query again
  if (route.query.uuid !== newUUID) {
    router.replace({
      query: {
        ...route.query,
        uuid: newUUID,
      },
    })
  }
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
