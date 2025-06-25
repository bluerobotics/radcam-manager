<template>
  <h4 class="mb-5">
    Zoom & Focus Controls
  </h4>
  
  <Slider
    name="focus"
    label="Focus"
    :current="focus_value ?? 0"
    :min="0"
    :max="1"
    :step="0.01"
    :disabled="props.disabled || processingAutopilotConfig"
    @update:current="setFocus('k_focus', $event)"
  />

  <Slider
    name="zoom"
    label="Zoom"
    :current="zoom_value ?? 0"
    :min="0"
    :max="1"
    :step="0.01"
    :disabled="props.disabled || processingAutopilotConfig"
    @update:current="setZoom('k_focus', $event)"
  />

  <v-expansion-panels>
    <v-expansion-panel>
      <v-expansion-panel-title><h4>Script Variables</h4></v-expansion-panel-title>
      <v-expansion-panel-text>
        <Slider
          name="k_focus"
          label="K Focus"
          :current="autopilotConfig.k_focus ?? 0"
          :min="0"
          :max="255"
          :step="1"
          :disabled="props.disabled || processingAutopilotConfig"
          @update:current="updateAutopilotConfig('k_focus', $event)"
        />

        <Slider
          name="k_zoom"
          label="K Zoom"
          :current="autopilotConfig.k_zoom ?? 0"
          :min="0"
          :max="255"
          :step="1"
          :disabled="props.disabled || processingAutopilotConfig"
          @update:current="updateAutopilotConfig('k_zoom', $event)"
        />

        <Slider
          name="k_scripting1"
          label="K Scripting 1"
          :current="autopilotConfig.k_scripting1 ?? 0"
          :min="0"
          :max="255"
          :step="1"
          :disabled="props.disabled || processingAutopilotConfig"
          @update:current="updateAutopilotConfig('k_scripting1', $event)"
        />

        <Slider
          name="margin_gain"
          label="Margin Gain"
          :current="autopilotConfig.margin_gain ?? 0"
          :min="0"
          :max="10"
          :step="0.01"
          :disabled="props.disabled || processingAutopilotConfig"
          @update:current="updateAutopilotConfig('margin_gain', $event)"
        />

        <Slider
          name="focus_channel"
          label="Focus Channel"
          :current="autopilotConfig.focus_channel ?? 0"
          :min="0"
          :max="255"
          :step="1"
          :disabled="props.disabled || processingAutopilotConfig"
          @update:current="updateAutopilotConfig('focus_channel', $event)"
        />

        <Slider
          name="zoom_channel"
          label="Zoom Channel"
          :current="autopilotConfig.zoom_channel ?? 0"
          :min="0"
          :max="255"
          :step="1"
          :disabled="props.disabled || processingAutopilotConfig"
          @update:current="updateAutopilotConfig('zoom_channel', $event)"
        />

        <Slider
          name="custom1_channel"
          label="Custom1 Channel"
          :current="autopilotConfig.custom1_channel ?? 0"
          :min="0"
          :max="255"
          :step="1"
          :disabled="props.disabled || processingAutopilotConfig"
          @update:current="updateAutopilotConfig('custom1_channel', $event)"
        />

        <Slider
          name="zoom_output_pwm"
          label="Zoom Output PWM"
          :current="autopilotConfig.zoom_output_pwm ?? 0"
          :min="1000"
          :max="2000"
          :step="1"
          :disabled="props.disabled || processingAutopilotConfig"
          @update:current="updateAutopilotConfig('zoom_output_pwm', $event)"
        />

        <Slider
          name="zoom_range"
          label="Zoom Range"
          :current="autopilotConfig.zoom_range ?? 0"
          :min="0"
          :max="3000"
          :step="1"
          :disabled="props.disabled || processingAutopilotConfig"
          @update:current="updateAutopilotConfig('zoom_range', $event)"
        />

        <Slider
          name="zoom_scaled"
          label="Zoom Scaled"
          :current="autopilotConfig.zoom_scaled ?? 0"
          :min="0"
          :max="1"
          :step="0.01"
          :disabled="props.disabled || processingAutopilotConfig"
          @update:current="updateAutopilotConfig('zoom_scaled', $event)"
        />

        <!-- Closest Points Editor -->
        <div class="mt-5">
          <h4>Closest Focus-Zoom Points</h4>
          <div v-if="autopilotConfig.closest_points?.length">
            <div
              v-for="(point, index) in autopilotConfig.closest_points ?? []"
              :key="index"
              class="d-flex align-center mb-2"
            >
              <v-text-field
                v-model.number="point.zoom"
                density="compact"
                type="number"
                width="90px"
                :min="0"
                :max="10000"
                :step="1"
                placeholder="Zoom"
                variant="outlined"
                hide-details
                single-line
                class="mr-2"
              />
              <v-text-field
                v-model.number="point.focus"
                density="compact"
                type="number"
                width="90px"
                :min="0"
                :max="10000"
                :step="1"
                placeholder="Foom"
                variant="outlined"
                hide-details
                single-line
                class="mr-2"
              />
              <v-btn
                variant="tonal"
                icon="$minus"
                class="ml-2"
                :disabled="autopilotConfig.closest_points.length === 1"
                @click="removePoint(autopilotConfig.closest_points, index)"
              />
              <v-btn
                variant="tonal"
                icon="$plus"
                :disabled="autopilotConfig.closest_points.length === 10"
                @click="addPoint(autopilotConfig.closest_points, index)"
              />
            </div>
          </div>
          <div v-else>
            <v-btn
              variant="tonal"
              icon="$plus"
              :disabled="autopilotConfig.closest_points?.length === 10"
              @click="addPoint(autopilotConfig.closest_points, 0)"
            />
          </div>
        </div>

        <!-- Furthest Points Editor -->
        <div>
          <h4>Furthest Focus-Zoom Points</h4>
          <div v-if="autopilotConfig.furthest_points?.length">
            <div
              v-for="(point, index) in autopilotConfig.furthest_points ?? []"
              :key="index"
              class="d-flex align-center mb-2"
            >
              <v-text-field
                v-model.number="point.zoom"
                density="compact"
                type="number"
                width="90px"
                :min="0"
                :max="10000"
                :step="1"
                placeholder="Zoom"
                variant="outlined"
                hide-details
                single-line
                class="mr-2"
              />
              <v-text-field
                v-model.number="point.focus"
                density="compact"
                type="number"
                width="90px"
                :min="0"
                :max="10000"
                :step="1"
                placeholder="Foom"
                variant="outlined"
                hide-details
                single-line
                class="mr-2"
              />
              <v-btn
                variant="tonal"
                icon="$minus"
                class="ml-2"
                :disabled="autopilotConfig.furthest_points.length === 1"
                @click="removePoint(autopilotConfig.furthest_points, index)"
              />
              <v-btn
                variant="tonal"
                icon="$plus"
                :disabled="autopilotConfig.furthest_points.length === 10"
                @click="addPoint(autopilotConfig.furthest_points, index)"
              />
            </div>
          </div>
          <div v-else>
            <v-btn
              variant="tonal"
              icon="$plus"
              :disabled="autopilotConfig.furthest_points?.length === 10"
              @click="addPoint(autopilotConfig.furthest_points, 0)"
            />
          </div>
        </div>
      </v-expansion-panel-text>
    </v-expansion-panel>
  </v-expansion-panels>
</template>


<script setup lang="ts">

import type { ApiConfig } from '@/bindings/autopilot'

import axios from 'axios'
import { onMounted, ref } from 'vue'

const props = defineProps<{
  selectedCameraUuid: string | null
  backendApi: string,
  disabled: boolean
}>()

const processingAutopilotConfig = ref(false)

const focus_value = ref<number>
const zoom_value = ref<number>

const autopilotConfig = ref<ApiConfig>({
  k_focus: null,
  k_zoom: null,
  k_scripting1: null,
  margin_gain: null,
  closest_points: [],
  furthest_points: [],
  focus_channel: null,
  zoom_channel: null,
  custom1_channel: null,
  zoom_output_pwm: null,
  zoom_range: null,
  zoom_scaled: null,
})

onMounted(() => {
  getAutopilotConfig()
})

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const updateAutopilotConfig = (param: keyof ApiConfig, value: any) => {
  if (!props.selectedCameraUuid) {
    return
  }

  const payload = {
    [param]: value,
  }

  console.log(payload)

  processingAutopilotConfig.value = true

  axios.post(`${props.backendApi}/config`, payload)
    .then(response => {
      autopilotConfig.value = response.data as ApiConfig
    })
    .catch(error => {
      console.error(`Error sending ${String(param)} control with value '${value}':`, error.message)
    })
    .finally(() => {
      processingAutopilotConfig.value = false
    })
}

const getAutopilotConfig = () => {
  if (!props.selectedCameraUuid) {
    return
  }

  processingAutopilotConfig.value = true

  axios.get(`${props.backendApi}/config`)
    .then(response => {
      autopilotConfig.value = response.data as ApiConfig
      console.log(response.data)
    })
    .catch(error => {
      console.error(`Error getting autopilot config:`, error.message)
    })
    .finally(() => {
      processingAutopilotConfig.value = false
    })
}

const addPoint = (points: {
  zoom: number
  focus: number
}[] | null, index: number) => {
  if (points === null) {
    points = []
  }

  if (points.length === 0) {
    points.push({zoom: 0, focus: 0})
    return
  }

  points.splice(index, 0, { ...points[index] })
}

const removePoint = (points: {
  zoom: number
  focus: number
}[] | null, index: number) => {
  if (points === null) {
    points = []
    return
  }

  if (points.length === 1) { // ensure it's never empty
    return
  }

  points.splice(index, 1)
}

const setZoom = (value: number) => {
  if (!props.selectedCameraUuid) {
    return
  }

  axios.post(`${props.backendApi}/config`, payload)
    .then(response => {
      autopilotConfig.value = response.data as ApiConfig
    })
    .catch(error => {
      console.error(`Error sending ${String(param)} control with value '${value}':`, error.message)
    })
}

</script>
