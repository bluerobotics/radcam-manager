<template>
  <div class="px-6 py-4 gap-y-6">
    <ExpansiblePanel
      title="Image"
      expanded
      theme="dark"
    >
      <BlueButtonGroup
        label="Water environment for One-Push White Balance"
        :button-items="[
          { name: 'Green', onSelected: () => (OPWBMode = 'green') },
          { name: 'Blue', onSelected: () => (OPWBMode = 'blue') },
        ]"
        theme="dark"
        type="switch"
      />

      <BlueButtonGroup
        label="RGB setpoints"
        :button-items="RGBSetpointProfiles"
        :buttons-menu="[
          { name: 'Add new profile', action: () => (openRGBSetpointForm = true), menuItemDisabled: RGBSetpointProfiles.length > 3 },
          { name: 'Delete profile', action: () => openRGBSetpointDelete = true, menuItemDisabled: RGBSetpointProfiles.length === 1 },
        ]"
        theme="dark"
        class="mt-6"
        type="switch"
      />
      <ExpansibleOptions
        :is-open="openRGBSetpointOptions"
        button-class="mt-[-25px] ml-[155px]"
        content-class="mt-4"
        :class="{ 'border-b-[1px] border-[#ffffff11] pb-2': openRGBSetpointOptions }"
      >
        <div class="flex flex-col justify-end items-end">
          <BlueSlider
            v-model="currentRGBSetpointValue[0]"
            name="awb_red"
            label="WB Red"
            color="red"
            :min="0"
            :max="255"
            :step="1"
            width="320px"
            theme="dark"
            class="scale-80 origin-right"
            @update:model-value="applyRGBSetpointColor('red', $event ?? 0)"
          />
          <BlueSlider
            v-model="currentRGBSetpointValue[1]"
            name="green-setpoint"
            label="WB Green"
            color="green"
            :min="0"
            :max="255"
            :step="1"
            width="320px"
            theme="dark"
            class="scale-80 origin-right ml-4"
            @update:model-value="applyRGBSetpointColor('green', $event ?? 0)"
          />
          <BlueSlider
            v-model="currentRGBSetpointValue[2]"
            name="blue-setpoint"
            label="WB Blue"
            color="#0B5087"
            :min="0"
            :max="255"
            :step="1"
            width="320px"
            theme="dark"
            class="scale-80 origin-right ml-4"
            @update:model-value="applyRGBSetpointColor('blue', $event ?? 0)"
          />
        </div>
      </ExpansibleOptions>
      <BlueSwitch
        v-model="focusAndZoomParams.enable_focus_and_zoom_correlation"
        name="focus-zoom-correlation"
        label="Focus and zoom correlation"
        theme="dark"
        class="mt-7"
        @update:model-value="updateActuatorsConfig('enable_focus_and_zoom_correlation', $event)"
      />
      <BlueSlider
        v-model="focusOffsetUI"
        name="focus-offset"
        label="Focus offset"
        :min="-10"
        :max="10"
        :step="0.1"
        width="400px"
        theme="dark"
        class="mt-5"
        @update:model-value="onFocusOffsetChange($event ?? 0)"
      />
    </ExpansiblePanel>
    <ExpansiblePanel
      title="Video"
      :expanded="cockpitMode ? false : true"
      theme="dark"
    >
      <BlueSelect
        v-model="selectedVideoResolution"
        label="Cockpit display"
        :items="resolutionOptions || [{ name: 'No resolutions available', value: null }]"
        theme="dark"
        @update:model-value="(value: any) => handleVideoChanges('resolution', value)"
      />
      <BlueSelect
        v-model="selectedVideoBitrate"
        label="Bitrate"
        :items="bitrateOptions || [{ name: 'No bitrates available', value: null }]"
        theme="dark"
        class="mt-5"
        @update:model-value="(value: any) => handleVideoChanges('bitrate', value)"
      >
        <template #insetElement>
          <div class="flex items-center justify-end w-full">
            <v-menu
              offset-y
              transition="scale-transition"
              theme="dark"
            >
              <template #activator="{ props: activatorProps }">
                <v-icon
                  v-bind="activatorProps"
                  class="ml-2 cursor-pointer text-[18px] mr-6"
                >
                  mdi-information-outline
                </v-icon>
              </template>
              <v-card class="w-[550px] text-white pa-0 rounded-lg border-[1px] border-[#ffffff33]">
                <div class="text-[sm] font-bold bg-[#4C4C4C22] text-center pa-1 pt-2">
                  H.264 Bitrate Options
                </div>
                <v-divider class="mb-2" />
                <div class="pr-0 pb-0">
                  <table class="border-collapse w-full text-[16px]">
                    <thead>
                      <tr>
                        <th class="border-b border-gray-600 pb-1 text-left pl-4 text-[14px]">
                          Resolution
                        </th>
                        <th class="border-b border-gray-600 pb-1 text-center text-[14px]">
                          High
                        </th>
                        <th class="border-b border-gray-600 pb-1 text-center text-[14px]">
                          Medium
                        </th>
                        <th class="border-b border-gray-600 pb-1 text-center text-[14px]">
                          Low
                        </th>
                      </tr>
                    </thead>
                    <tbody>
                      <tr
                        v-for="row in h264BitrateTable"
                        :key="row.resolution"
                        class="border-t-[1px] border-[#ffffff11]"
                      >
                        <td class="pl-4 py-1 text-[16px] pt-2">
                          {{ row.resolution }}<br>
                          <span class="opacity-70 text-[14px] align-center">Disk usage</span>
                        </td>
                        <td class="mt-1 text-center">
                          {{ row.high.bitrate }} kbps<br>
                          <span class="opacity-70">{{ row.high.storage }} Gb/h</span>
                        </td>
                        <td class="mt-1 text-center">
                          {{ row.medium.bitrate }} kbps<br>
                          <span class="opacity-70">{{ row.medium.storage }} Gb/h</span>
                        </td>
                        <td class="mt-1 text-center">
                          {{ row.low.bitrate }} kbps<br>
                          <span class="opacity-70">{{ row.low.storage }} Gb/h</span>
                        </td>
                      </tr>
                    </tbody>
                  </table>
                </div>
              </v-card>
            </v-menu>
          </div>
        </template>
      </BlueSelect>
      <div
        v-if="hasUnsavedVideoChanges"
        class="flex justify-end mt-8 mb-[-20px]"
      >
        <v-btn
          class="py-1 px-3 rounded-md bg-[#0B5087] text-white hover:bg-[#0A3E6B]"
          :class="{ 'opacity-50 pointer-events-none': !hasUnsavedVideoChanges }"
          size="small"
          variant="elevated"
          @click="saveVideoDataAndRestart"
        >
          SAVE AND RESTART CAMERA
        </v-btn>
      </div>
    </ExpansiblePanel>
    <ExpansiblePanel
      title="Actuators"
      expanded
      theme="dark"
    >
      <BlueSlider
        v-if="actuatorsState"
        v-model="actuatorsState.focus"
        name="focus"
        label="Focus"
        :min="0"
        :max="100"
        :step="1"
        :format-value="formatFocusValue"
        label-min="0.5m"
        label-max="∞"
        width="400px"
        theme="dark"
        class="mt-5"
        @update:model-value="updateActuatorsState('focus', $event as number)"
      />
      <BlueSlider
        v-if="actuatorsState"
        v-model="actuatorsState.zoom"
        name="zoom"
        label="Zoom"
        :min="0"
        :max="100"
        :step="1"
        :format-value="formatZoomValue"
        label-min="1x"
        label-max="3x"
        width="400px"
        theme="dark"
        class="mt-5"
        @update:model-value="updateActuatorsState('zoom', $event as number)"
      />
      <BlueSlider
        v-if="actuatorsState && false"
        v-model="actuatorsState.tilt"
        name="tilt"
        label="Tilt"
        :min="0"
        :max="100"
        :step="1"
        :format-value="formatTiltValue"
        :label-min="`${focusAndZoomParams.tilt_mnt_pitch_min !== null ? focusAndZoomParams.tilt_mnt_pitch_min : -90}°`"
        :label-max="`${focusAndZoomParams.tilt_mnt_pitch_max !== null ? focusAndZoomParams.tilt_mnt_pitch_max : 90}°`"
        width="400px"
        theme="dark"
        class="mt-5"
        @update:model-value="updateActuatorsState('tilt', $event as number)"
      />
    </ExpansiblePanel>
    <ExpansiblePanel
      title="Hardware setup"
      :expanded="cockpitMode ? false : true"
      theme="dark"
    >
      <div class="mb-4 p-3">
        Please select below each correspondent channel pin from the Navigator Board each cable is connected to. 
        The recommended is focus 10, zoom 11, script 12, tilt 16.
      </div>

      <BlueSelect
        v-model="tempChannelChanges.focus_channel"
        label="Focus PWM output"
        :items="availableServoChannelOptions"
        theme="dark"
        @update:model-value="handleChannelChanges('focus_channel', $event)"
      />
      <BlueSelect
        v-model="tempChannelChanges.zoom_channel"
        label="Zoom PWM output"
        :items="availableServoChannelOptions"
        theme="dark"
        class="mt-6"
        @update:model-value="handleChannelChanges('zoom_channel', $event)"
      />
      <BlueSelect
        v-model="tempChannelChanges.script_channel"
        label="Script PWM input"
        :items="availableServoChannelOptions"
        theme="dark"
        class="mt-6"
        @update:model-value="handleChannelChanges('script_channel', $event)"
      />
      <BlueSelect
        v-model="tempChannelChanges.tilt_channel"
        label="Tilt PWM output"
        :items="availableServoChannelOptions"
        theme="dark"
        class="mt-6"
        @update:model-value="handleChannelChanges('tilt_channel', $event)"
      />
      <ExpansibleOptions
        :is-open="openRGBSetpointOptions"
        button-class="mt-[-24px] ml-[180px]"
        content-class="mt-4"
        :class="{ 'border-b-[1px] border-[#ffffff11] pb-2': openRGBSetpointOptions }"
      >
        <BlueSwitch
          v-model="tempChannelChanges.tilt_channel_reversed"
          name="tilt-channel-reversed"
          label="Tilt channel reversed"
          theme="dark"
          class="scale-90 origin-right"
          @update:model-value="handleChannelChanges('tilt_channel_reversed', $event)"
        />
      </ExpansibleOptions>
      <div class="flex justify-end mt-6">
        <v-btn
          class="py-1 px-3 rounded-md bg-[#0B5087] text-white hover:bg-[#0A3E6B]"
          size="small"
          variant="elevated"
          :disabled="!isHardwareSetupComplete || hasDuplicateChannels || isLoading"
          :loading="isLoading"
          @click="saveHardwareSetup"
        >
          SAVE HARDWARE SETUP
        </v-btn>
      </div>
    </ExpansiblePanel>
  </div>
  <v-dialog
    v-model="openRGBSetpointForm"
    width="400px"
  >
    <v-card class="bg-[#363636] text-white">
      <v-card-title class="text-h6 text-center py-4">
        RGB Setpoint Profile
      </v-card-title>
      <v-card-text>
        <v-text-field
          v-model="newRGBSetpointProfileName"
          label="Profile Name"
          required
          class="my-3 mx-2"
          counter="10"
          maxlength="10"
        />
      </v-card-text>
      <v-card-actions class="px-4">
        <v-btn
          variant="text"
          class="opacity-70"
          @click="openRGBSetpointForm = false"
        >
          Cancel
        </v-btn>
        <v-spacer />
        <v-btn
          color="white"
          @click="saveRGBSetpointProfile"
        >
          Save
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
  <v-dialog
    v-model="openRGBSetpointDelete"
    width="400px"
  >
    <v-card class="bg-[#363636] text-white">
      <v-card-title class="text-h6 text-center py-4">
        Delete RGB Setpoint Profile
      </v-card-title>
      <v-card-text>
        Are you sure you want to delete the profile "{{ currentRGBSetpointProfile }}"?
      </v-card-text>
      <v-card-actions class="px-4">
        <v-btn
          variant="text"
          class="opacity-70"
          @click="openRGBSetpointDelete = false"
        >
          Cancel
        </v-btn>
        <v-spacer />
        <v-btn
          color="red"
          @click="() => {
            RGBSetpointProfiles = RGBSetpointProfiles.filter(profile => profile.name !== currentRGBSetpointProfile)
            openRGBSetpointDelete = false
          }"
        >
          Delete
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
  <Loading :is-loading="isLoading" />
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import BlueButtonGroup from './BlueButtonGroup.vue'
import BlueSlider from './BlueSlider.vue'
import BlueSwitch from './BlueSwitch.vue'
import ExpansiblePanel from './ExpansiblePanel.vue'
import BlueSelect from './BlueSelect.vue'
import ExpansibleOptions from './ExpansibleOptions.vue'
import Loading from './Loading.vue'
import { VideoChannelValue, type BaseParameterSetting, type VideoParameterSettings, type VideoResolutionValue } from '@/bindings/radcam'
import axios from 'axios'
import type { ActuatorsConfig, ActuatorsControl, ActuatorsParametersConfig, ActuatorsState } from '@/bindings/autopilot'
import { applyNonNull } from '@/utils/jsonUtils'


const props = defineProps<{
  selectedCameraUuid: string | null
  backendApi: string
  disabled: boolean
  cockpitMode: boolean
}>()

const servoChannelOptions = Array.from({ length: 16 }, (_, i) => ({
  name: `Channel ${i + 1}`,
  value: `SERVO${i + 1}`,
}))

const baseParams = ref<BaseParameterSetting>({
  hue: null,
  brightness: null,
  sharpness: null,
  contrast: null,
  saturation: null,
  gamma: null,
  blc_level: null,
  max_exposure: null,
  set_default: null,
  antiFog: null,
  frameTurbo_pro: null,
  sceneMode: null,
  AE_strategy_mode: null,
  auto_exposureEx: null,
  exposure_time: null,
  auto_awb: null,
  awb_red: null,
  awb_green: null,
  awb_blue: null,
  awb_auto_mode: null,
  awb_style_red: null,
  awb_style_green: null,
  awb_style_blue: null,
  auto_gain_mode: null,
  auto_DGain_max: null,
  auto_AGain_max: null,
  max_sys_gain: null,
  manual_AGain_enable: null,
  manual_AGain: null,
  manual_DGain_enable: null,
  manual_DGain: null,
  rotate: null,
})

const focusAndZoomParams = ref<ActuatorsParametersConfig>({
  camera_id: null,
  focus_channel: null,
  focus_channel_min: null,
  focus_channel_trim: null,
  focus_channel_max: null,
  focus_margin_gain: null,
  script_function: null,
  script_channel: null,
  script_channel_min: null,
  script_channel_trim: null,
  script_channel_max: null,
  enable_focus_and_zoom_correlation: null,
  zoom_channel: null,
  zoom_channel_min: null,
  zoom_channel_trim: null,
  zoom_channel_max: null,
  tilt_channel: null,
  tilt_channel_min: null,
  tilt_channel_trim: null,
  tilt_channel_max: null,
  tilt_channel_reversed: null,
  tilt_mnt_type: null,
  tilt_mnt_pitch_min: null,
  tilt_mnt_pitch_max: null,
})

const OPWBMode = ref('green')
const openRGBSetpointOptions = ref(false)
const openRGBSetpointForm = ref(false)
const newRGBSetpointProfileName = ref('')
const currentRGBSetpointValue = ref<number[]>([
  baseParams.value.awb_red || 0,
  baseParams.value.awb_green || 0,
  baseParams.value.awb_blue || 0,
])
const currentRGBSetpointProfile = ref<string | null>('Custom 1')
const selectedVideoResolution = ref<VideoResolutionValue | null>(null)
const selectedVideoBitrate = ref<number | null>(null)
const selectedVideoParameters = ref<VideoParameterSettings>({})
const downloadedVideoParameters = ref<VideoParameterSettings>({})
const openRGBSetpointDelete = ref(false)
const actuatorsState = ref<ActuatorsState>({
  focus: 0,
  zoom: 0,
  tilt: 0,
})
const isLoading = ref<boolean>(false)
const hasUnsavedChannelChanges = ref<boolean>(false)
const tempChannelChanges = ref<{
  focus_channel: string | null
  zoom_channel: string | null
  tilt_channel: string | null
  tilt_channel_reversed: boolean | null
  script_channel: string | null
}>({
  focus_channel: null,
  zoom_channel: null,
  tilt_channel: null,
  tilt_channel_reversed: null,
  script_channel: null,
})
const hasUnsavedVideoChanges = ref<boolean>(false)
const tempVideoChanges = ref<{
  pic_width: number | null
  pic_height: number | null
  bitrate: number | null
}>({
  pic_width: null,
  pic_height: null,
  bitrate: null,
})

const RGBSetpointProfiles = ref([
  {
    name: 'Custom 1',
    onSelected: () => applyRGBSetpointProfile('Custom 1'),
    options: { awb_red: 0, awb_green: 0, awb_blue: 0 },
    preSelected: currentRGBSetpointProfile.value === 'Custom 1',
  },
  {
    name: 'Custom 2',
    onSelected: () => applyRGBSetpointProfile('Custom 2'),
    options: { awb_red: 150, awb_green: 150, awb_blue: 150 },
    preSelected: currentRGBSetpointProfile.value === 'Custom 2',
  },
])

const resolutionOptions = ref([
  { name: '3840x2160', value: { width: 3840, height: 2160 } },
  { name: '1920x1080', value: { width: 1920, height: 1080 } },
])

const resolutionsToBitrate: Record<string, number[]> = {
  '3840x2160': [16384, 8192, 4096],
  '1920x1080': [8192, 4096, 2048],
}

const h264BitrateTable = [
  { resolution: '3840x2160', high: { bitrate: 16384, storage: 7.2 }, medium: { bitrate: 8192, storage: 3.6 }, low: { bitrate: 4096, storage: 1.8 } },
  { resolution: '1920x1080', high: { bitrate: 8192, storage: 3.6 }, medium: { bitrate: 4096, storage: 1.8 }, low: { bitrate: 2048, storage: 0.9 } }
]

const bitrateOptions = computed(() => {
  const res = selectedVideoResolution.value
  if (!res) return null

  const key = `${res.width}x${res.height}`
  const allowed = resolutionsToBitrate[key]
  if (!allowed) return null

  return allowed.map((bitrate) => ({
    name: `${bitrate} kbps`,
    value: bitrate,
  }))
})

const mapFocusUiToRaw = (ui: number, min: number, max: number): number => {
  if (max === min) return min
  const ratio = (ui + 10) / 20 
  return Math.round(min + ratio * (max - min))
}

const mapFocusRawToUi = (raw: number, min: number, max: number): number => {
  if (max === min) return 0
  const ratio = (raw - min) / (max - min)
  return ratio * 20 - 10
}

// Convert focus_channel_trim (raw, user defined on BlueOS) to UI value (-10 to 10) and vice versa
const focusOffsetUI = computed<number>({
  get: () => {
    const min = focusAndZoomParams.value.focus_channel_min
    const max = focusAndZoomParams.value.focus_channel_max
    let raw = focusAndZoomParams.value.focus_channel_trim
    if (raw! < min! || raw! > max!) {
      const averageRaw = Math.round((min! + max!) / 2)
      raw = averageRaw
    }
    return mapFocusRawToUi(raw!, min!, max!)
  },
  set: (uiVal: number) => {
    const min = focusAndZoomParams.value.focus_channel_min
    const max = focusAndZoomParams.value.focus_channel_max
    if (min == null || max == null) return
    focusAndZoomParams.value.focus_channel_trim = mapFocusUiToRaw(uiVal, min, max)
  },
})

const onFocusOffsetChange = (uiVal: number): void => {
  const min = focusAndZoomParams.value.focus_channel_min
  const max = focusAndZoomParams.value.focus_channel_max
  if (min == null || max == null) {
    return
  }
  const raw = mapFocusUiToRaw(uiVal, min, max)
  updateActuatorsConfig('focus_channel_trim', raw)
}

const formatFocusValue = (raw: number): string => {
  if (raw >= 100) {
    return '∞'
  }
  // Map 0–99.99 → 0.5m – ~50m
  const ratio = raw / 100 // [0, 1)
  const distance = 0.5 / (1 - ratio)

  // Optional: cap for readability (e.g., don't show 1234.56m)
  if (distance >= 50) {
    return '50m+'
  }

  // Round to 1 decimal for <10m, whole number otherwise
  if (distance < 10) {
    return `${distance.toFixed(1)}m`
  } else {
    return `${Math.round(distance)}m`
  }
}

const formatZoomValue = (raw: number): string => {
  const zoomLevel = 1.0 + (raw / 100) * 2.0
  return `${zoomLevel.toFixed(1)}x`
}

const formatTiltValue = (raw: number): string => {
  const minAngle = focusAndZoomParams.value.tilt_mnt_pitch_min ?? -90
  const maxAngle = focusAndZoomParams.value.tilt_mnt_pitch_max ?? 90

  // Map raw [0, 100] → [minAngle, maxAngle]
  const angle = minAngle + (raw / 100) * (maxAngle - minAngle)
  return `${angle.toFixed(1)}°`
}

const saveRGBSetpointProfile = () => {
  if (!newRGBSetpointProfileName.value) {
    console.error('Profile name is required')
    return
  }
  const profileName = newRGBSetpointProfileName.value
  RGBSetpointProfiles.value.push({
    name: profileName,
    onSelected: () => applyRGBSetpointProfile(profileName),
    options: {
      awb_red: 0,
      awb_green: 0,
      awb_blue: 0,
    },
    preSelected: currentRGBSetpointProfile.value === profileName,
  })
  openRGBSetpointForm.value = false
  newRGBSetpointProfileName.value = ''
}

const applyRGBSetpointColor = (color: 'red' | 'green' | 'blue', value: number) => {
  console.log(`Applying RGB setpoint color: ${color} with value: ${value}`)
  switch (color) {
    case 'red':
      currentRGBSetpointValue.value[0] = value
      updateBaseParameter('awb_red', value)
      RGBSetpointProfiles.value.forEach((profile) => {
        if (profile.name === currentRGBSetpointProfile.value) {
          profile.options.awb_red = value
        }
      })
      break
    case 'green':
      currentRGBSetpointValue.value[1] = value
      updateBaseParameter('awb_green', value)
      RGBSetpointProfiles.value.forEach((profile) => {
        if (profile.name === currentRGBSetpointProfile.value) {
          profile.options.awb_green = value
        }
      })
      break
    case 'blue':
      currentRGBSetpointValue.value[2] = value
      updateBaseParameter('awb_blue', value)
      RGBSetpointProfiles.value.forEach((profile) => {
        if (profile.name === currentRGBSetpointProfile.value) {
          profile.options.awb_blue = value
        }
      })
      break
  }
}

const applyRGBSetpointProfile = (profileName: string) => {
  const profile = RGBSetpointProfiles.value.find((profile) => profile.name === profileName)
  if (!profile) {
    console.error('Profile not found')
    return
  }
  console.log(`Applying RGB setpoint profile: ${profileName}`)

  currentRGBSetpointProfile.value = profileName
  currentRGBSetpointValue.value = [
    profile.options.awb_red || 0,
    profile.options.awb_green || 0,
    profile.options.awb_blue || 0,
  ]

  updateBaseParameter('awb_red', profile.options.awb_red)
  updateBaseParameter('awb_green', profile.options.awb_green)
  updateBaseParameter('awb_blue', profile.options.awb_blue)
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const updateBaseParameter = (param: keyof BaseParameterSetting, value: any) => {
  if (!props.selectedCameraUuid) {
    return
  }

  const payload = {
    camera_uuid: props.selectedCameraUuid,
    action: 'setImageAdjustment',
    json: {
      [param]: value,
    },
  }

  console.log(payload)

  axios
    .post(`${props.backendApi}/camera/control`, payload)
    .then((response) => {
      baseParams.value = response.data as BaseParameterSetting
    })
    .catch((error) => {
      console.error(`Error sending ${String(param)} control with value '${value}':`, error.message)
    })
}

const getActuatorsConfig = () => {
  if (!props.selectedCameraUuid) {
    return
  }

  const payload = {
    camera_uuid: props.selectedCameraUuid,
    action: "getActuatorsConfig",
  }
  
  console.log('# - getActuatorsConfig payload:', payload)

  axios
    .post(`${props.backendApi}/autopilot/control`, payload)
    .then(response => {
      const newParams = (response.data as ActuatorsConfig)?.parameters
      if (newParams) {
        focusAndZoomParams.value = { ...newParams }
        tempChannelChanges.value = {
          focus_channel: newParams.focus_channel,
          zoom_channel: newParams.zoom_channel,
          tilt_channel: newParams.tilt_channel,
          tilt_channel_reversed: newParams.tilt_channel_reversed,
          script_channel: newParams.script_channel,
        }
        hasUnsavedChannelChanges.value = false
      } else {
        console.warn("Received null 'parameters' from response:", response.data)
      }
      console.log('# - getActuatorsConfig response:', response.data)

    })
    .catch(error => {
      console.error(`Error sending getActuatorsConfig request:`, error.message)
    })
}


// eslint-disable-next-line @typescript-eslint/no-explicit-any
const updateActuatorsConfig = (param: keyof ActuatorsParametersConfig, value: any) => {
  if (!props.selectedCameraUuid) {
    return
  }

   const payload: ActuatorsControl = {
    camera_uuid: props.selectedCameraUuid,
    action: "setActuatorsConfig",
    json: { "parameters": { [param]: value } as ActuatorsParametersConfig} as ActuatorsConfig
  }

  console.log(payload)

  axios
    .post(`${props.backendApi}/autopilot/control`, payload)
    .then((response) => {
      const newParams = (response.data as ActuatorsConfig)?.parameters
      if (newParams) {
        focusAndZoomParams.value = { ...newParams }
      } else {
        console.warn("Received null 'parameters' from response:", response.data)
      }
    })
    .catch((error) => {
      console.error(`Error sending ${String(param)} control with value '${value}':`, error.message)
    })
}

const getActuatorsState = () => {
  if (!props.selectedCameraUuid) {
    return
  }

  const payload = {
    camera_uuid: props.selectedCameraUuid,
    action: "getActuatorsState",
  }

  console.log(payload)

  axios
    .post(`${props.backendApi}/autopilot/control`, payload)
    .then(response => {
      const state = response.data as ActuatorsState

      applyNonNull(actuatorsState.value, state)
      console.log(state)
    })
    .catch(error => {
      console.error(`Error sending getActuatorsState request:`, error.message)
    })
}

const updateActuatorsState = (param: keyof ActuatorsState, value: number) => {
  if (!props.selectedCameraUuid) return

  const payload: ActuatorsControl = {
    camera_uuid: props.selectedCameraUuid,
    action: "setActuatorsState",
    json: { [param]: value } as ActuatorsState
  }

  console.log(payload)

  axios
    .post(`${props.backendApi}/autopilot/control`, payload)
    .then(response => { 
      const state = response.data as ActuatorsState;

      applyNonNull(actuatorsState.value, state)
      console.log(state)
    })
    .catch(error => {
      console.error(`Error updating ${param}:`, error.message)
    })
}

const isHardwareSetupComplete = computed<boolean>(() => {
  return (
    tempChannelChanges.value.focus_channel !== null &&
    tempChannelChanges.value.zoom_channel !== null &&
    tempChannelChanges.value.tilt_channel !== null &&
    tempChannelChanges.value.script_channel !== null
  )
})

const hasDuplicateChannels = computed<boolean>(() => {
  const channels = [
    tempChannelChanges.value.focus_channel,
    tempChannelChanges.value.zoom_channel,
    tempChannelChanges.value.tilt_channel,
    tempChannelChanges.value.script_channel
  ].filter(channel => channel !== null)
  
  return new Set(channels).size !== channels.length
})

const availableServoChannelOptions = computed(() => {
  const selectedChannels = new Set([
    tempChannelChanges.value.focus_channel,
    tempChannelChanges.value.zoom_channel,
    tempChannelChanges.value.tilt_channel,
    tempChannelChanges.value.script_channel
  ].filter(channel => channel !== null))

  return servoChannelOptions.map(option => ({
    ...option,
    disabled: selectedChannels.has(option.value) && 
              option.value !== tempChannelChanges.value.focus_channel &&
              option.value !== tempChannelChanges.value.zoom_channel &&
              option.value !== tempChannelChanges.value.tilt_channel &&
              option.value !== tempChannelChanges.value.script_channel
  }))
})

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const handleChannelChanges = ( param: keyof typeof tempChannelChanges.value, value: any): void => {
  if (!props.selectedCameraUuid) return

  // Check if this value is already selected by another channel
  const isAlreadySelected = Object.entries(tempChannelChanges.value).some(
    ([key, channelValue]) => key !== param && channelValue === value
  )

  if (isAlreadySelected && value !== null) {
    console.warn(`Channel ${value} is already selected for another function`)
    // Don't update if duplicate
    return
  }

  tempChannelChanges.value[param] = value                                 
  hasUnsavedChannelChanges.value = (
    (focusAndZoomParams.value)[param] !== value                    
  ) || Object.entries(tempChannelChanges.value).some(                    
    ([k, v]) =>
      (focusAndZoomParams.value)[k as keyof ActuatorsParametersConfig] !== v,
  )
}


const getVideoParameters = (update: boolean) => {
  if (!props.selectedCameraUuid) {
    return
  }

  const video_parameter_settings = {
    channel: selectedVideoParameters.value.channel ?? VideoChannelValue.MainStream,
  }

  const payload = {
    camera_uuid: props.selectedCameraUuid,
    action: 'getVencConf',
    json: video_parameter_settings,
  }

  axios
    .post(`${props.backendApi}/camera/control`, payload)
    .then((response) => {
      const settings: VideoParameterSettings = response.data as VideoParameterSettings

      if (update) {
        update_video_parameter_values(settings)
      }
    })
    .catch((error) => console.error(`Error sending getVencConf request:`, error.message))
}

const updateVideoParameters = (partial: Partial<VideoParameterSettings>): void => {
  if (!props.selectedCameraUuid) return

  const payload = {
    camera_uuid: props.selectedCameraUuid,
    action: 'setVencConf',
    json: partial as VideoParameterSettings,
  }

  axios
    .post(`${props.backendApi}/camera/control`, payload)
    .then((response) => {
      const settings = response.data as VideoParameterSettings
      update_video_parameter_values(settings)
    })
    .catch((error) => {
      console.error(`Error sending partial video params '${JSON.stringify(partial)}':`, error.message)
  })
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const handleVideoChanges = (what: 'resolution' | 'bitrate', value: any): void => {
  if (!props.selectedCameraUuid) return

  if (what === 'resolution' && value) {
    selectedVideoResolution.value = value as VideoResolutionValue
    tempVideoChanges.value.pic_width  = value.width
    tempVideoChanges.value.pic_height = value.height
    const key = `${value.width}x${value.height}`
    const allowed = resolutionsToBitrate[key]

    if (allowed?.length) {
      selectedVideoBitrate.value = allowed[0]      
      tempVideoChanges.value.bitrate = allowed[0]  
    }
  }

  if (what === 'bitrate') {
    selectedVideoBitrate.value = value as number   
    tempVideoChanges.value.bitrate = value as number
  }

  const cfg = selectedVideoParameters.value as Pick<VideoParameterSettings, keyof typeof tempVideoChanges.value>
  const videoTempChanges = (Object.entries(tempVideoChanges.value) as [keyof typeof tempVideoChanges.value, number | null][])
    .some(([k, v]) => {
      if (v === null) return false
      return (cfg[k] ?? null) !== v
  })

  hasUnsavedVideoChanges.value = videoTempChanges   
}


const update_video_parameter_values = (settings: VideoParameterSettings) => {
  downloadedVideoParameters.value = { ...settings }

  selectedVideoParameters.value = { ...settings }
  selectedVideoParameters.value.pixel_list = undefined

  const width = settings.pic_width
  const height = settings.pic_height

  let matchOption = resolutionOptions.value.find(                   
    o => o.value.width === width && o.value.height === height                      
  )

  if (!matchOption && width && height) {
    const injectedValue = { width, height }
    resolutionOptions.value.push({ name: `${width}x${height}`, value: injectedValue })
    matchOption = resolutionOptions.value[resolutionOptions.value.length - 1]
  }

  if (matchOption) {
    selectedVideoResolution.value = matchOption.value
  } else {
    selectedVideoResolution.value = null
  }

  selectedVideoBitrate.value = settings.bitrate ?? null
  
  tempVideoChanges.value = {
    pic_width: null,
    pic_height: null,
    bitrate: null,
  }
}


const doRestart = () => {
  if (!props.selectedCameraUuid) {
    return
  }

  console.log("Restarting...")

  isLoading.value = true

  const payload = {
    camera_uuid: props.selectedCameraUuid,
    action: "restart",
  }

  axios
    .post(`${props.backendApi}/camera/control`, payload)
    .then((response) => {
      console.log("Got an answer from the restarting request", response.data)
    })
    .catch((error) =>
      console.error(
        `Error sending restart':`,
        error.message
      )
    )
    .finally(() => {
      isLoading.value = false
    })
}

const saveVideoDataAndRestart = async (): Promise<void> => {
  if (!props.selectedCameraUuid) return

  const videoPartial: Partial<VideoParameterSettings> = {}
  const curr = selectedVideoParameters.value
  const tmp  = tempVideoChanges.value

  type VideoMutableKeys = 'pic_width' | 'pic_height' | 'bitrate'
  const videoKeys: VideoMutableKeys[] = ['pic_width', 'pic_height', 'bitrate']

  videoKeys.forEach((k) => {
    const newVal: number | null = tmp[k]
    const currVal: number | null | undefined = (curr as Record<VideoMutableKeys, number | null | undefined>)[k]
    if (newVal !== null && newVal !== currVal) {
      ;(videoPartial as Record<VideoMutableKeys, number>)[k] = newVal
    }
  })

  if (Object.keys(videoPartial).length > 0) {
    await updateVideoParameters(videoPartial)
    doRestart()
    Object.assign(selectedVideoParameters.value, videoPartial)
  }

  tempVideoChanges.value = { pic_width: null, pic_height: null, bitrate: null }
  hasUnsavedVideoChanges.value = false
}

const saveHardwareSetup = async (): Promise<void> => {
  if (!props.selectedCameraUuid) return

  if (!isHardwareSetupComplete.value) {
    console.error('All channel selections are required')
    return
  }

  isLoading.value = true
  
  const payload: ActuatorsControl = {
    camera_uuid: props.selectedCameraUuid,
    action: "setActuatorsConfig",
    json: { 
      parameters: {
        focus_channel: tempChannelChanges.value.focus_channel,
        zoom_channel: tempChannelChanges.value.zoom_channel,
        tilt_channel: tempChannelChanges.value.tilt_channel,
        script_channel: tempChannelChanges.value.script_channel,
        tilt_channel_reversed: tempChannelChanges.value.tilt_channel_reversed,
      } as ActuatorsParametersConfig
    } as ActuatorsConfig
  }

  console.log('Saving hardware setup:', payload)

  axios
    .post(`${props.backendApi}/autopilot/control`, payload)
    .then((response) => {
      console.log("Got an answer from the setActuatorsConfig request", response.data)

      const newParams = (response.data as ActuatorsConfig)?.parameters
      if (newParams) {
        focusAndZoomParams.value = { ...newParams }

        hasUnsavedChannelChanges.value = false
      }
    })
    .catch((error) =>
      console.error(
        `Error saving hardware setup':`,
        error.message
      )
    )
    .finally(() => {
      isLoading.value = false
    })
}

onMounted(() => {
  getInitialCameraStates()
})

const getInitialCameraStates = () => {
  getActuatorsConfig()
  getActuatorsState()
  getVideoParameters(true)
}

defineExpose({ getInitialCameraStates })

watch(
  () => props.selectedCameraUuid,
  async (newValue) => {
    if (newValue) {
      getInitialCameraStates()
    }
  }
)

watch(
  () => selectedVideoResolution.value,
  (newRes) => {
    if (!newRes) return
    const key = `${newRes.width}x${newRes.height}`
    const allowed = resolutionsToBitrate[key]
    if (!allowed?.length) {
      selectedVideoBitrate.value = null
      tempVideoChanges.value.bitrate = null
      return
    }
    if (!selectedVideoBitrate.value || !allowed.includes(selectedVideoBitrate.value)) {
      selectedVideoBitrate.value = allowed[0]
      tempVideoChanges.value.bitrate = allowed[0]
    }
  }
)

// keep bitrate options in sync with resolution changes
watch(
  () => selectedVideoResolution.value,
  (newRes) => {
    if (!newRes) return
    const key = `${newRes.width}x${newRes.height}`
    const allowed = resolutionsToBitrate[key]
    if (!allowed || allowed.length === 0) {
      selectedVideoBitrate.value = null
      return
    }
    if (!selectedVideoBitrate.value || !allowed.includes(selectedVideoBitrate.value)) {
      selectedVideoBitrate.value = allowed[0]                                  
      updateVideoParameters({ bitrate: allowed[0] })                          
    }
  }
)
</script>