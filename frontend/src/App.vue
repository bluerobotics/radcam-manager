<template>
  <v-app class="bg-[#0F5892]">
    <v-main>
      <v-container
        no-gutters
        width="800px"
        class="text-white pa-0 mt-6 rounded-[8px] elevation-5"
        :class="theme === 'dark' ? 'bg-[#363636]' : 'bg-[#f5f5f5]'"
      >
        <div class="flex items-center justify-between bg-[#2C2C2C] rounded-t-[8px]">
          <div class="flex items-center justify-around w-[400px] pl-5 border-b-[1px] border-[#ffffff66]">
            <v-icon>mdi-camera-party-mode</v-icon>
            <v-select
              v-model="selectedCameraUUID"
              :items="cameras"
              item-title="hostname"
              item-value="uuid"
              label="Camera"
              hide-details
              theme="dark"
              class="bg-[#232323] ml-3 -mb-[1px]"
            >
              <template #item="{ props, item }">
                <v-list-item v-bind="props" :subtitle="item.raw.uuid" />
              </template>
            </v-select>
          </div>
          <BlueButtonGroup
            :button-items="[
              { name: 'Basic', onSelected: () => (configMode = 'basic'), tooltip: 'Basic setup for the RadCam' },
              { name: 'Advanced', onSelected: () => (configMode = 'advanced'), tooltip: 'Advanced camera settings' },
            ]"
            :theme="theme"
            class="mr-4"
            type="switch"
          />
        </div>
        <template v-if="configMode === 'basic'">
          <BasicSettings :selected-camera-uuid="selectedCameraUUID" :backend-api="backendAPI" :disabled="false" />
        </template>

        <template v-if="configMode === 'advanced'">
          <v-tabs v-model="tab" align-tabs="center" class="mb-5">
            <v-tab value="image"> Image </v-tab>
            <v-tab value="streams"> Streams </v-tab>
            <v-tab value="configs" :disabled="true"> Configs </v-tab>
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
        </template>
      </v-container>
    </v-main>
  </v-app>
</template>

<script setup lang="ts">
import axios from 'axios'
import { onMounted, onUnmounted, ref } from 'vue'
import { useRoute } from 'vue-router'

import type { Camera } from '@/bindings/mcm_client'
import BasicSettings from '@/components/BasicSettings.vue'
import BlueButtonGroup from '@/components/BlueButtonGroup.vue'

const tab = ref(null)
const backendAddress = window.location.host.toString()
const backendAPI = ref(`http://${backendAddress}/v1/camera`)
const cameras = ref<Camera[]>([])
const selectedCameraUUID = ref<string | null>(null)

const route = useRoute()
const desiredCameraUuid = ref<string | null>(null)

const theme = ref<'light' | 'dark'>('dark')
const configMode = ref<'basic' | 'advanced'>('basic')

const getCameras = async () => {
  try {
    const response = await axios.get(`${backendAPI.value}/list`)
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
</script>
