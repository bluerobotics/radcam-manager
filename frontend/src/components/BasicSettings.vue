<template>
  <div class="px-6 py-4 gap-y-6">
    <ExpansiblePanel
      title="Image"
      expanded
      theme="dark"
    >
      <BlueButtonGroup
        label="Water environment for OPWB"
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
        class="mt-5"
        @update:model-value="updateActuatorsConfig('enable_focus_and_zoom_correlation', $event)"
      />
      <BlueSlider
        v-model="focusAndZoomParams.focus_channel_trim"
        name="focus-offset"
        label="Focus offset"
        :min="-10"
        :max="10"
        :step="0.1"
        width="400px"
        theme="dark"
        class="mt-5"
        @update:model-value="updateActuatorsConfig('focus_channel_trim', $event)"
      />
    </ExpansiblePanel>
    <ExpansiblePanel
      title="Video"
      expanded
      theme="dark"
    >
      <BlueSelect
        v-model="selectedVideoResolution"
        label="Cockpit display"
        :items="resolutionOptions || [{ name: 'No resolutions available', value: null }]"
        theme="dark"
      />
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
        width="400px"
        theme="dark"
        class="mt-5"
        @update:model-value="updateActuatorsState('zoom', $event as number)"
      />
      <BlueSlider
        v-if="actuatorsState"
        v-model="actuatorsState.tilt"
        name="tilt"
        label="Tilt"
        :min="0"
        :max="100"
        :step="1"
        width="400px"
        theme="dark"
        class="mt-5"
        @update:model-value="updateActuatorsState('tilt', $event as number)"
      />
    </ExpansiblePanel>
    <ExpansiblePanel
      title="Hardware setup"
      expanded
      theme="dark"
    >
      <BlueSelect
        v-model="focusAndZoomParams.focus_channel"
        label="Focus PWM output"
        :items="servoChannelOptions"
        theme="dark"
        @update:model-value="updateActuatorsConfig('focus_channel', $event)"
      />
      <BlueSelect
        v-model="focusAndZoomParams.zoom_channel"
        label="Zoom PWM output"
        :items="servoChannelOptions"
        theme="dark"
        class="mt-6"
        @update:model-value="updateActuatorsConfig('zoom_channel', $event)"
      />
      <BlueSelect
        v-model="focusAndZoomParams.tilt_channel"
        label="Tilt PWM output"
        :items="servoChannelOptions"
        theme="dark"
        class="mt-6"
        @update:model-value="updateActuatorsConfig('tilt_channel', $event)"
      />
      <ExpansibleOptions
        :is-open="openRGBSetpointOptions"
        button-class="mt-[-24px] ml-[180px]"
        content-class="mt-4"
        :class="{ 'border-b-[1px] border-[#ffffff11] pb-2': openRGBSetpointOptions }"
      >
        <BlueSwitch
          v-model="focusAndZoomParams.tilt_channel_reversed"
          name="tilt-channel-reversed"
          label="Tilt channel reversed"
          theme="dark"
          class="scale-90 origin-right"
          @update:model-value="updateActuatorsConfig('tilt_channel_reversed', $event)"
        />
      </ExpansibleOptions>
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
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import BlueButtonGroup from './BlueButtonGroup.vue'
import BlueSlider from './BlueSlider.vue'
import BlueSwitch from './BlueSwitch.vue'
import ExpansiblePanel from './ExpansiblePanel.vue'
import BlueSelect from './BlueSelect.vue'
import ExpansibleOptions from './ExpansibleOptions.vue'
import { VideoChannelValue, type BaseParameterSetting, type VideoParameterSettings, type VideoResolutionValue } from '@/bindings/radcam'
import axios from 'axios'
import type { ActuatorsConfig, ActuatorsControl, ActuatorsParametersConfig, ActuatorsState, ServoChannel } from '@/bindings/autopilot'

const props = defineProps<{
  selectedCameraUuid: string | null
  backendApi: string
  disabled: boolean
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
const currentRGBSetpointProfile = ref<string | null>('Custom 2')
const selectedVideoResolution = ref<VideoResolutionValue | null>(null)
const selectedVideoParameters = ref<VideoParameterSettings>({})
const downloadedVideoParameters = ref<VideoParameterSettings>({})
const openRGBSetpointDelete = ref(false)
const actuatorsState = ref<ActuatorsState | null>({
  focus: 0,
  zoom: 0,
  tilt: 0,
})

const RGBSetpointProfiles = ref([
  {
    name: 'Custom 1',
    onSelected: () => applyRGBSetpointProfile('Custom 1'),
    options: { awb_red: 100, awb_green: 100, awb_blue: 100 },
    preSelected: currentRGBSetpointProfile.value === 'Custom 1',
  },
  {
    name: 'Custom 2',
    onSelected: () => applyRGBSetpointProfile('Custom 2'),
    options: { awb_red: 150, awb_green: 150, awb_blue: 150 },
    preSelected: currentRGBSetpointProfile.value === 'Custom 2',
  },
])

const resolutionOptions = computed(() => {
  return downloadedVideoParameters.value.pixel_list?.map((res: VideoResolutionValue) => ({
    name: `${res.width}x${res.height}`,
    value: res,
  }))
})

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

  console.log(payload)

  axios
    .post(`${props.backendApi}/autopilot/control`, payload)
    .then(response => {
      const newParams = (response.data as ActuatorsConfig)?.parameters
      if (newParams) {
        focusAndZoomParams.value = { ...newParams }
      } else {
        console.warn("Received null 'parameters' from response:", response.data)
      }
      console.log(response.data)
    })
    .catch(error => {
      console.error(`Error sending getActuatorsConfig request:`, error.message)
    })
}

watch(() => focusAndZoomParams.value.focus_channel, (val) => {
  console.log('focus_channel changed to', val)
})


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
      // the return type is a ActuatorsState
      // actuatorsState.value = { ...response.data }
      console.log(response.data)
    })
    .catch(error => {
      console.error(`Error sending getImageAdjustment request:`, error.message)
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
      actuatorsState.value = { ...response.data }
      console.log(response.data)
    })
    .catch(error => {
      console.error(`Error updating ${param}:`, error.message)
    })
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

const update_video_parameter_values = (settings: VideoParameterSettings) => {
  downloadedVideoParameters.value = { ...settings }

  selectedVideoParameters.value = { ...settings }
  selectedVideoParameters.value.pixel_list = undefined

  selectedVideoResolution.value = {
    width: settings.pic_width!,
    height: settings.pic_height!,
  } as VideoResolutionValue
}

onMounted(() => {
  getActuatorsConfig()
  getActuatorsState()
})

watch(
  () => props.selectedCameraUuid,
  async (newValue) => {
    if (newValue) {
      getActuatorsConfig()
      getActuatorsState()
      getVideoParameters(true)
    }
  }
)

watch(
  () => selectedVideoResolution.value,
  async (newValue) => {
    if (newValue) {
      selectedVideoParameters.value.pic_width = newValue.width
      selectedVideoParameters.value.pic_height = newValue.height
    }
  }
)
</script>
