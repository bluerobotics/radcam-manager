<template>
  <v-tabs
    v-model="selectedVideoParameters.channel"
    align-tabs="center"
  >
    <v-tab
      v-for="option in channelOptions.filter(opt => opt.value < 1)"
      :key="option.value"
      :value="option.value"
      :disabled="props.disabled || processingUpdate"
    >
      {{ option.text }}
    </v-tab>
  </v-tabs>
  <v-window
    v-model="selectedVideoParameters.channel"
    class="pt-5"
  >
    <v-window-item
      v-for="option in channelOptions"
      :key="option.value"
      :value="option.value"
    >
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
        v-model.number="adjustedBitrate"
        :disabled="props.disabled || processingUpdate"
        label="Bitrate (kbps)"
        type="number"
        min="1024"
        max="40960"
        step="1024"
      />
      <v-text-field
        v-model.number="selectedVideoParameters.frame_rate"
        :disabled="props.disabled || processingUpdate"
        label="Frame Rate"
        type="number"
        min="1"
        :max="selectedVideoParameters.max_framerate"
      />
      <v-text-field
        v-model.number="selectedVideoParameters.gop"
        :disabled="props.disabled || processingUpdate"
        label="I-Frame Interval (GOP)"
        type="number"
        min="1"
        max="100"
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
          {{
            processingUpdate
              ? "Processing..."
              : needs_restart
                ? "Apply and Restart"
                : "Apply"
          }}
        </v-btn>
      </div>
    </v-window-item>
  </v-window>
</template>

<script setup lang="ts">
import axios from "axios"
import { computed, onMounted, ref, watch } from "vue"
import { enumToOptions } from "@/utils/enumUtils"
import {
  VideoChannelValue,
  VideoEncodeTypeValue,
  VideoEncodingProfileValue,
  VideoRcModeValue,
  type VideoParameterSettings,
  type VideoResolutionValue,
} from "@/bindings/radcam"

const props = defineProps<{
  selectedCameraUuid: string | null
  backendApi: string
  disabled: boolean
}>()

const processingUpdate = ref<boolean>(false)

const channelOptions = enumToOptions(VideoChannelValue)
const encodeProfileOptions = enumToOptions(VideoEncodingProfileValue)
const encodeTypeOptions = enumToOptions(VideoEncodeTypeValue)
const rcModeOptions = enumToOptions(VideoRcModeValue)
const resolutionOptions = computed(() => {
  return downloadedVideoParameters.value.pixel_list?.map(
    (res: VideoResolutionValue) => ({
      text: `${res.width}x${res.height}`,
      value: res,
    })
  )
})

const selectedVideoParameters = ref<VideoParameterSettings>({})
const downloadedVideoParameters = ref<VideoParameterSettings>({})
const selectedVideoResolution = ref<VideoResolutionValue | null>(null)
const needs_restart = ref<boolean>(false)

watch(
  () => props.selectedCameraUuid,
  async (newValue) => {
    if (newValue) {
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

watch(
  () => selectedVideoParameters.value.channel,
  async (newValue, oldValue) => {
    if (newValue !== oldValue) {
      getVideoParameters(true)
    }
  }
)

watch(
  () => selectedVideoParameters.value.encode_profile,
  async (newValue) => {
    needs_restart.value = newValue !== downloadedVideoParameters.value.encode_profile
  }
)
watch(
  () => selectedVideoParameters.value.encode_type,
  async (newValue) => {
    needs_restart.value = newValue !== downloadedVideoParameters.value.encode_type
  }
)
watch(
  () => selectedVideoParameters.value.pic_width,
  async (newValue) => {
    needs_restart.value = newValue !== downloadedVideoParameters.value.pic_width
  }
)
watch(
  () => selectedVideoParameters.value.pic_height,
  async (newValue) => {
    needs_restart.value = newValue !== downloadedVideoParameters.value.pic_height
  }
)

onMounted(() => {
  getVideoParameters(true)
})

const adjustedBitrate = computed({
  get: () => selectedVideoParameters.value.bitrate,
  set: (newValue: number) => {
    const rounded = Math.round(newValue / 1024) * 1024
    selectedVideoParameters.value.bitrate = rounded
  }
})

const updateVideoParameters = () => {
  if (!props.selectedCameraUuid) {
    return
  }

  processingUpdate.value = true

  console.debug(selectedVideoParameters.value)

  const video_parameter_settings = selectedVideoParameters.value

  const payload = {
    camera_uuid: props.selectedCameraUuid,
    action: "setVencConf",
    json: video_parameter_settings,
  }

  axios
    .post(`${props.backendApi}/camera/control`, payload)
    .then((response) => {
      if (!needs_restart.value) {
        const settings: VideoParameterSettings =
          response.data as VideoParameterSettings
        update_video_parameter_values(settings)
      }
    })
    .catch((error) =>
      console.error(
        `Error sending ${video_parameter_settings}':`,
        error.message
      )
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

  const payload = {
    camera_uuid: props.selectedCameraUuid,
    action: "restart",
  }

  axios
    .post(`${props.backendApi}/camera/control`, payload)
    .then((response) => {
      console.log("Got an answer from the restarting request", response.data)
      needs_restart.value = false
    })
    .catch((error) =>
      console.error(
        `Error sending restart':`,
        error.message
      )
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
    channel: selectedVideoParameters.value.channel ?? VideoChannelValue.MainStream,
  }

  const payload = {
    camera_uuid: props.selectedCameraUuid,
    action: "getVencConf",
    json: video_parameter_settings,
  }

  axios
    .post(`${props.backendApi}/camera/control`, payload)
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
  downloadedVideoParameters.value = { ...settings }

  selectedVideoParameters.value = { ...settings }
  selectedVideoParameters.value.pixel_list = undefined

  selectedVideoResolution.value = {
    width: settings.pic_width!,
    height: settings.pic_height!,
  } as VideoResolutionValue
}
</script>
