<template>
  <!-- Image sliders -->
  <Slider
    v-for="slider in imageSliders"
    :key="slider.name"
    :name="slider.name"
    :label="slider.label"
    :current="slider.current"
    :min="slider.min"
    :max="slider.max"
    :step="slider.step"
    :disabled="props.disabled"
    @update:current="slider.updateFunction"
  />

  <!-- White balance button -->
  <div class="ma-2 text-right">
    <v-btn
      variant="tonal"
      :disabled="props.disabled || processingWhiteBalance"
      @click="doWhiteBalance"
    >
      <v-progress-circular
        v-if="processingWhiteBalance"
        indeterminate
        color="white"
        size="20"
        class="me-2"
      />
      {{ processingWhiteBalance ? "Processing..." : "Do White Balance" }}
    </v-btn>
  </div>

  <v-divider class="ma-5" />

  <!-- Restore Image Parameters -->
  <div class="ma-2 text-right">
    <v-btn
      variant="tonal"
      :disabled="props.disabled || processingRestore"
      @click="doRestore"
    >
      <v-progress-circular
        v-if="processingRestore"
        indeterminate
        color="white"
        size="20"
        class="me-2"
      />
      {{ processingRestore ? "Processing..." : "Restore defaults" }}
    </v-btn>
  </div>  
</template>

<script setup lang="ts">

import type { BaseParameterSetting, CameraControl, AdvancedParameterSetting } from '@/bindings/radcam'
import axios from 'axios'
import { onMounted, ref, watch } from 'vue'

const props = defineProps<{
    selectedCameraUuid: string | null
    backendApi: string,
    disabled: boolean
}>()

const processingWhiteBalance = ref(false)
const processingRestore = ref(false)
const imageSliders = [
    {
        name: "brightness_slider",
        label: "Brightness",
        current: ref(128),
        min: 0,
        max: 255,
        step: 1,
        updateFunction: (value: number) => updateImageParameter("brightness", value),
    },
    {
        name: "hue_slider",
        label: "Hue",
        current: ref(128),
        min: 0,
        max: 255,
        step: 1,
        updateFunction: (value: number) => updateImageParameter("hue", value),
    },
    {
        name: "sharpness_slider",
        label: "Sharpness",
        current: ref(128),
        min: 0,
        max: 255,
        step: 1,
        updateFunction: (value: number) => updateImageParameter("sharpness", value),
    },
    {
        name: "contrast_slider",
        label: "Contrast",
        current: ref(128),
        min: 0,
        max: 255,
        step: 1,
        updateFunction: (value: number) => updateImageParameter("contrast", value),
    },
    {
        name: "saturation_slider",
        label: "Saturation",
        current: ref(128),
        min: 0,
        max: 255,
        step: 1,
        updateFunction: (value: number) => updateImageParameter("saturation", value),
    },
    {
        name: "gamma_slider",
        label: "Gamma",
        current: ref(128),
        min: 0,
        max: 255,
        step: 1,
        updateFunction: (value: number) => updateImageParameter("gamma", value),
    },
]

onMounted(() => {
    getImageParameters()
})

watch(
    () => props.selectedCameraUuid,
    async (newValue) => {
        if (newValue) {
            getImageParameters()
        }
    }
)

const updateImageParameter = (param: keyof BaseParameterSetting, value: number) => {
    if (!props.selectedCameraUuid) {
        return
    }

    const payload = {
        camera_uuid: props.selectedCameraUuid,
        action: "setImageAdjustment",
        json: {
            [param]: value,
        },
    }

    axios
        .post(`${props.backendApi}/control`, payload)
        .then(response => {
            const settings: BaseParameterSetting = response.data as BaseParameterSetting

            update_sliders_values(settings)
        })
        .catch((error) =>
            console.error(`Error sending ${String(param)} control with value '${value}':`, error.message)
        )
}

const getImageParameters = () => {
    if (!props.selectedCameraUuid) {
        return
    }

    const payload = {
        camera_uuid: props.selectedCameraUuid,
        action: "getImageAdjustment",
    }

    axios
        .post(`${props.backendApi}/control`, payload)
        .then(response => {
            const settings: BaseParameterSetting = response.data as BaseParameterSetting

            update_sliders_values(settings)
        })
        .catch(error =>
            console.error(`Error sending getImageAdjustment request:`, error.message)
        )

}

const update_sliders_values = (settings: BaseParameterSetting) => {
    imageSliders.forEach(slider => {
        const sliderKey = slider.name.replace("_slider", "")
        const value = settings[sliderKey as keyof BaseParameterSetting] as number | undefined

        if (value !== undefined) {
            slider.current.value = value // Update only if a new value exists
        }
    })
}

const doWhiteBalance = async () => {
    if (!props.selectedCameraUuid) {
        return
    }

    processingWhiteBalance.value = true

    const payload: CameraControl = {
        camera_uuid: props.selectedCameraUuid,
        action: "setImageAdjustmentEx",
        json: {
            onceAWB: 1,
        } as AdvancedParameterSetting,
    }

    axios.post(`${props.backendApi}/control`, payload)
        .catch(error => {
            console.error("Error sending onceAWB control:", error.message)
        }).finally(() => {
            processingWhiteBalance.value = false
        })
}

const doRestore = async () => {
    if (!props.selectedCameraUuid) {
        return
    }

    processingRestore.value = true

    const payload: CameraControl = {
        camera_uuid: props.selectedCameraUuid,
        action: "setImageAdjustment",
        json: {
            set_default: 1,
        } as BaseParameterSetting,
    }

    axios
        .post(`${props.backendApi}/control`, payload)
        .then(response => {
            const settings: BaseParameterSetting = response.data as BaseParameterSetting

            update_sliders_values(settings)
        })
        .catch(error => {
            console.error("Error sending base image restore control:", error.message)
        })

        .finally(() => {
            processingRestore.value = false
        })
}

</script>
