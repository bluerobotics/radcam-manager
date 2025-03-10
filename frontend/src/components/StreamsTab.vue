<template>
  <v-select
    v-model="selectedVideoParameters.channel"
    :items="channelOptions"
    :disabled="props.disabled || processingUpdate"
    label="Stream Channel"
    item-title="text"
    item-value="value"
  />
  <v-select
    v-model="selectedVideoParameters.encode_profile"
    :items="encodeProfileOptions"
    :disabled="props.disabled || processingUpdate"
    label="Encode Profile"
    item-title="text"
    item-value="value"
  />
  <v-select
    v-model="selectedVideoParameters.encode_type"
    :items="encodeTypeOptions"
    :disabled="props.disabled || processingUpdate"
    label="Encode Type"
    item-title="text"
    item-value="value"
  />
  <v-select
    v-model="selectedVideoResolution"
    :items="resolutionOptions"
    :disabled="props.disabled || processingUpdate"
    label="Resolution"
    item-title="text"
    item-value="value"
  />
  <v-select
    v-model="selectedVideoParameters.rc_mode"
    :items="rcModeOptions"
    :disabled="props.disabled || processingUpdate"
    label="Bitrate Type"
    item-title="text"
    item-value="value"
  />
  <v-text-field
    v-model.number="selectedVideoParameters.bitrate"
    :disabled="props.disabled || processingUpdate"
    label="Bitrate (kbps)"
    type="number"
    :min="1"
    max="128000"
  />
  <v-text-field
    v-model.number="selectedVideoParameters.frame_rate"
    :disabled="props.disabled || processingUpdate"
    label="Frame Rate"
    type="number"
    :min="1"
    :max="selectedVideoParameters.max_framerate"
  />
  <v-text-field
    v-model.number="selectedVideoParameters.gop"
    :disabled="props.disabled || processingUpdate"
    label="I-Frame Interval (GOP)"
    type="number"
    :min="1"
    max="10000"
  />

  <v-divider class="ma-5" />

  <div class="ma-2 text-right">
    <v-btn
      variant="tonal"
      :disabled="props.disabled || processingUpdate"
      @click="updateVideoParameters"
    >
      <v-progress-circular
        v-if="processingUpdate"
        indeterminate
        color="white"
        size="20"
        class="me-2"
      />
      {{ processingUpdate ? "Processing..." : (needs_restart ? "Apply and Restart" : "Apply") }}
    </v-btn>
  </div>
</template>

<script setup lang="ts">
import axios from "axios"
import { computed, onMounted, ref, watch } from "vue"
import { enumToOptions } from '@/utils/enumUtils'
import { ChannelValue, EncodeTypeValue, EncodingProfileValue, RcModeValue, type VideoParameterSettings, type VideoResolutionValue } from "@/bindings/radcam"


const props = defineProps<{
  selectedCameraUuid: string | null
  backendApi: string
  disabled: boolean
}>()

const processingUpdate = ref<boolean>(false)

const channelOptions = enumToOptions(ChannelValue)
const encodeProfileOptions = enumToOptions(EncodingProfileValue)
const encodeTypeOptions = enumToOptions(EncodeTypeValue)
const rcModeOptions = enumToOptions(RcModeValue)

const selectedVideoParameters = ref<VideoParameterSettings>({})
const lastVideoParameters = ref<VideoParameterSettings>({})
const selectedVideoResolution = ref<VideoResolutionValue | null>(null)
const pixelListOptions = ref<VideoResolutionValue[]>([])
const needs_restart = ref<boolean>(false)

const resolutionOptions = computed(() => {
  return pixelListOptions.value.map((res: VideoResolutionValue) => ({
    text: `${res.width}x${res.height}`,
    value: res
  }))
})

watch(
  () => props.selectedCameraUuid,
  async (newValue) => {
    if (newValue) {
      getVideoParameters(true)
    }
  }
)

watch(
  () => selectedVideoParameters.value.channel,
  async (newValue, oldValue) => {
    if (newValue && newValue !== oldValue) {
         getVideoParameters(true)
    }
  }
)

watch(
  () => selectedVideoParameters.value.encode_profile,
  async (newValue) => {
    needs_restart.value = newValue !== lastVideoParameters.value.encode_profile
  }
)
watch(
  () => selectedVideoParameters.value.encode_type,
  async (newValue) => {
    needs_restart.value = newValue !== lastVideoParameters.value.encode_type
  }
)
watch(
  () => selectedVideoParameters.value.pic_width,
  async (newValue) => {
    needs_restart.value = newValue !== lastVideoParameters.value.pic_width
  }
)
watch(
  () => selectedVideoParameters.value.pic_height,
  async (newValue) => {
    needs_restart.value = newValue !== lastVideoParameters.value.pic_height
  }
)

onMounted(() => {
  getVideoParameters(true)
})

const updateVideoParameters = () => {
  if (!props.selectedCameraUuid) {
    return
  }

  processingUpdate.value = true

  const video_parameter_settings = selectedVideoParameters.value
  video_parameter_settings.pic_height = selectedVideoResolution.value?.height
  video_parameter_settings.pic_width = selectedVideoResolution.value?.width

  const payload = {
      camera_uuid: props.selectedCameraUuid,
      action: "setVencConf",
      json: video_parameter_settings
  }

  axios
      .post(`${props.backendApi}/control`, payload)
      .then(response => {
          if (!needs_restart.value) {
            const settings: VideoParameterSettings = response.data as VideoParameterSettings
            update_video_parameter_values(settings)
          }
      })
      .catch((error) =>
          console.error(`Error sending ${video_parameter_settings}':`, error.message)
      )
      .finally(() => {
        if (needs_restart.value) {
            doRestart()
        } else {
            processingUpdate.value = false
        }
      })
}

const doRestart = () => {
  if (!props.selectedCameraUuid) {
    return
  }

  console.log("Restarting...")

  processingUpdate.value = true

  const video_parameter_settings = selectedVideoParameters.value
  video_parameter_settings.pic_height = selectedVideoResolution.value?.height
  video_parameter_settings.pic_width = selectedVideoResolution.value?.width

  const payload = {
      camera_uuid: props.selectedCameraUuid,
      action: "restart",
  }

  axios
      .post(`${props.backendApi}/control`, payload)
      .then(response => {
          console.log("Got an answer from the restarting request", response.data)
          needs_restart.value = false
      })
      .catch((error) =>
          console.error(`Error sending ${video_parameter_settings}':`, error.message)
      )
      .finally(() => {
        processingUpdate.value = false
      })
}


const getVideoParameters = (update: boolean) => {
  if (!props.selectedCameraUuid) {
    return
  }

  const video_parameter_settings = {
    channel: selectedVideoParameters.value.channel ?? ChannelValue.MainStream,
  }

  const payload = {
    camera_uuid: props.selectedCameraUuid,
    action: "getVencConf",
    json: video_parameter_settings,
  }

  axios
    .post(`${props.backendApi}/control`, payload)
    .then((response) => {
      const settings: VideoParameterSettings =
        response.data as VideoParameterSettings

        if (update) {
        update_video_parameter_values(settings)
        }
    })
    .catch((error) =>
      console.error(`Error sending getVencConf request:`, error.message)
    )
}

const update_video_parameter_values = (settings: VideoParameterSettings) => {
  pixelListOptions.value = settings.pixel_list ?? pixelListOptions.value
  selectedVideoResolution.value = {
    width: settings.pic_width!,
    height: settings.pic_height!,
  } as VideoResolutionValue

  selectedVideoParameters.value.channel =
    settings.channel ?? selectedVideoParameters.value.channel
  selectedVideoParameters.value.encode_profile =
    settings.encode_profile ?? selectedVideoParameters.value.encode_profile
  selectedVideoParameters.value.encode_type =
    settings.encode_type ?? selectedVideoParameters.value.encode_type
  selectedVideoParameters.value.pixel_list =
    settings.pixel_list ?? selectedVideoParameters.value.pixel_list
  selectedVideoParameters.value.pic_width =
    settings.pic_width ?? selectedVideoParameters.value.pic_width
  selectedVideoParameters.value.pic_height =
    settings.pic_height ?? selectedVideoParameters.value.pic_height
  selectedVideoParameters.value.rc_mode =
    settings.rc_mode ?? selectedVideoParameters.value.rc_mode
  selectedVideoParameters.value.bitrate =
    settings.bitrate ?? selectedVideoParameters.value.bitrate
  selectedVideoParameters.value.max_framerate =
    settings.max_framerate ?? selectedVideoParameters.value.max_framerate
  selectedVideoParameters.value.frame_rate =
    settings.frame_rate ?? selectedVideoParameters.value.frame_rate
  selectedVideoParameters.value.gop = settings.gop ?? selectedVideoParameters.value.gop

  lastVideoParameters.value =  { ...selectedVideoParameters.value }
}

</script>
