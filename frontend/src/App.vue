<template>
  <v-app>
    <v-main>
      <v-container
        no-gutters
        max-width="500px"
      >
        <v-select
          v-model="selectedCameraUUID"
          :items="cameras"
          item-title="hostname"
          item-value="uuid"
          label="Camera"
        >
          <template #item="{ props, item }">
            <v-list-item
              v-bind="props"
              :subtitle="item.raw.uuid"
            />
          </template>
        </v-select>

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
      </v-container>
    </v-main>
  </v-app>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue"
import type { Camera } from '@/bindings/mcm_client'
import axios from 'axios'
import { useRoute } from "vue-router"

const tab = ref(null)
const backendAddress = window.location.host.toString()
const backendAPI = ref(`http://${backendAddress}/v1/camera`)
const cameras = ref<Camera[]>([])
const selectedCameraUUID = ref<string | null>(null)

const route = useRoute()
const desiredCameraUuid = ref<string | null>(null)

const getCameras = async () => {
  try {
    const response = await axios.get(`${backendAPI.value}/list`)
    const camerasData = validateCameras(response.data)
    cameras.value = camerasData

    if (!selectedCameraUUID.value && cameras.value.length > 0) {
      const foundCamera = desiredCameraUuid.value
        ? cameras.value.find(camera => camera.uuid === desiredCameraUuid.value)
        : null

      selectedCameraUUID.value = foundCamera ? foundCamera.uuid : cameras.value[0].uuid
    }
  } catch (error) {
    console.error("Error getting cameras:", error)
  }
}

const validateCameras = (data: unknown): Camera[] => {
  if (typeof data !== "object" || data === null) {
    throw new Error("Expected a map of { uuid: camera }")
  }

  const cameras: Camera[] = []
  for (const [uuid, cameraData] of Object.entries(data)) {
    if (isCamera(cameraData)) {
      cameras.push({ ...cameraData, uuid })
    }
  }
  return cameras
}

const isCamera = (data: unknown): data is Omit<Camera, "uuid"> => {
  if (typeof data !== "object" || data === null) return false

  const camera = data as Record<string, unknown>

  const isStreamsValid =
    typeof camera.streams === "object" &&
    camera.streams !== null &&
    Object.values(camera.streams).every((stream) => typeof stream === "string")

  return (
    typeof camera.hostname === "string" &&
    typeof camera.credentials === "object" &&
    camera.credentials !== null &&
    typeof (camera.credentials as Record<string, unknown>).username === "string" &&
    typeof (camera.credentials as Record<string, unknown>).password === "string" &&
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
    clearInterval(intervalId);
  });
})
</script>
