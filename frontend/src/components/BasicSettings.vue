<template>
  <div class="px-6 py-4">
    <ExpansiblePanel
      title="Image"
      :expanded="isConfigured"
      theme="dark"
    >
      <BlueButtonGroup
        label="Water environment White Balance"
        :disabled="!isConfigured || props.disabled"
        :button-items="WhiteBalanceSceneButtonItems"
        theme="dark"
        type="switch"
      />
      <BlueButtonGroup
        label="White Balance Mode"
        :disabled="!isConfigured || props.disabled"
        :button-items="whiteBalanceModeButtonItems"
        theme="dark"
        type="switch"
        class="mt-6"
      />
      <div 
        class="d-flex flex-col align-end mt-6"
      >
        <v-btn
          :disabled="props.disabled"
          class="py-1 px-3 ml-4 rounded-md bg-[#414141] hover:bg-[#0A3E6B]"
          size="small"
          variant="elevated"
          theme="dark"
          @click="doWhiteBalance"
        >
          <v-progress-circular
            v-if="processingWhiteBalance"
            indeterminate
            color="white"
            size="20"
            class="me-2"
          />
          {{ processingWhiteBalance ? "Processing..." : "One-Push White Balance" }}
        </v-btn>
      </div>
      <div
        v-if="baseParams.auto_awb"
        class="d-flex flex-column align-end mt-6"
      >
        <BlueSlider
          v-model="baseParams.awb_red"
          :disabled="!isConfigured || props.disabled"
          name="red"
          label="Red"
          :min="0"
          :max="255"
          :step="1"
          width="400px"
          theme="dark"
          @update:model-value="updateBaseParameter('awb_red', $event as number)"
        />
        <BlueSlider
          v-model="baseParams.awb_blue"
          :disabled="!isConfigured || props.disabled"
          name="blue"
          label="Blue"
          :min="0"
          :max="255"
          :step="1"
          width="400px"
          theme="dark"
          class="mt-6"
          @update:model-value="updateBaseParameter('awb_blue', $event as number)"
        />
      </div>
    </ExpansiblePanel>
    <ExpansiblePanel
      title="Actuators"
      :expanded="isConfigured"
      theme="dark"
    >
      <BlueSlider
        v-if="actuatorsState"
        v-model="actuatorsState.focus"
        :disabled="!isConfigured || props.disabled"
        name="focus"
        label="Focus"
        :min="0"
        :max="100"
        :step="0.1"
        :format-display="formatFocusValue"
        :scale-fn="scaleFocus"
        :unscale-fn="unscaleFocus"
        label-min="0.5m"
        label-max="∞"
        width="400px"
        theme="dark"
        @update:model-value="updateActuatorsState('focus', $event as number)"
      />
      <BlueSlider
        v-if="actuatorsState"
        v-model="actuatorsState.zoom"
        :disabled="!isConfigured || props.disabled"
        name="zoom"
        label="Zoom"
        :min="0"
        :max="100"
        :step="1"
        :format-display="formatZoomValue"
        :scale-fn="scaleZoom"
        :unscale-fn="unscaleZoom"
        label-min="1x"
        label-max="3x"
        width="400px"
        theme="dark"
        class="mt-6"
        @update:model-value="updateActuatorsState('zoom', $event as number)"
      />
      <BlueSlider
        v-if="actuatorsState && false"
        v-model="actuatorsState.tilt"
        :disabled="!isConfigured || props.disabled"
        name="tilt"
        label="Tilt"
        :min="0"
        :max="100"
        :step="1"
        :format-display="formatTiltValue"
        :scale-fn="scaleTilt"
        :unscale-fn="unscaleTilt"
        :label-min="`${currentFocusAndZoomParams.tilt_mnt_pitch_min}` || ''"
        :label-max="`${currentFocusAndZoomParams.tilt_mnt_pitch_max}` || ''"
        width="400px"
        theme="dark"
        class="mt-6"
        @update:model-value="updateActuatorsState('tilt', $event as number)"
      />
      <ExpansiblePanel
        class="d-flex flex-col align-end mt-4"
        title="more"
        :expanded="isConfigured && !cockpitMode"
        theme="dark"
      >
        <div>
          <BlueSwitch
            v-model="currentFocusAndZoomParams.enable_focus_and_zoom_correlation"
            :disabled="!isConfigured || props.disabled"
            name="focus-zoom-correlation"
            label="Enable focus and zoom correlation"
            theme="dark"
            @update:model-value="updateActuatorsConfig('enable_focus_and_zoom_correlation', $event)"
          />
          <!-- <BlueSlider
            v-model="focusOffsetUI"
            :disabled="!isConfigured || props.disabled"
            name="focus-offset"
            label="Focus compensation"
            :min="-10"
            :max="10"
            :step="0.1"
            width="400px"
            theme="dark"
            class="mt-6"
            @update:model-value="onFocusOffsetChange($event ?? 0)"
          /> -->
        </div>
      </ExpansiblePanel>
    </ExpansiblePanel>
    <ExpansiblePanel
      title="Video"
      :expanded="isConfigured && !cockpitMode"
      theme="dark"
    >
      <BlueSelect
        v-model="selectedVideoResolution"
        :disabled="!isConfigured || props.disabled"
        label="Resolution"
        :items="resolutionOptions || [{ name: 'No resolutions available', value: null }]"
        theme="dark"
        @update:model-value="(value: any) => handleVideoChanges('resolution', value)"
      />
      <BlueSelect
        v-model="selectedVideoBitrate"
        :disabled="!isConfigured || props.disabled"
        label="Bitrate"
        :items="bitrateOptions || [{ name: 'No bitrates available', value: null }]"
        theme="dark"
        class="mt-6"
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
          :disabled="!isConfigured || props.disabled"
          class="py-1 px-3 rounded-md bg-[#0B5087] hover:bg-[#0A3E6B]"
          :class="{ 'opacity-50 pointer-events-none': !hasUnsavedVideoChanges }"
          size="small"
          variant="elevated"
          theme="dark"
          @click="saveVideoDataAndRestart"
        >
          SAVE AND RESTART CAMERA
        </v-btn>
      </div>
    </ExpansiblePanel>
    <ExpansiblePanel
      title="Hardware setup"
      :expanded="!isConfigured"
      theme="dark"
    >
      <div>
        <p class="mb-3">
          Assign Navigator PWM output channels to your camera functions below. The recommended setup is:
        </p>
        <ul class="list-disc pl-5 mb-4 text-sm">
          <li><b>Focus</b>: Connect the camera's Focus cable to Navigator's <b>PWM Channel 10</b></li>
          <li><b>Zoom</b>: Connect the camera's Zoom cable to Navigator's <b>PWM Channel 11</b></li>
          <li><b>Script</b>: Navigator's <b>PWM Channel 12</b> is used as an <i>input</i> used by the internal Lua script that enables Focus/Zoom correlation (no physical cable connects here)</li>
          <li><b>Tilt</b>: Connect the camera's Tilt cable to Navigator's <b>PWM Channel 16</b></li>
        </ul>
        <p class="mb-3">
          Click <b>APPLY DEFAULT HARDWARE SETUP</b> to use the recommended configuration above, or click <b>ADVANCED SETUP</b> to customize your channel assignments and parameters.
        </p>
      </div>

      <!-- Default Simple Setup -->
      <div
        v-if="!showAdvancedHardware"
        class="mb-4 p-3"
      >
        <div class="d-flex flex-row ga-3 mt-5 justify-end">
          <v-btn
            :disabled="props.disabled"
            class="py-1 px-3 ml-4 rounded-md bg-[#414141] hover:bg-[#0A3E6B]"
            size="small"
            variant="elevated"
            theme="dark"
            @click="showAdvancedHardware = true"
          >
            Advanced setup
          </v-btn>
          <v-btn
            class="py-1 px-3 ml-4 rounded-md bg-[#0B5087] hover:bg-[#0A3E6B]"
            size="small"
            variant="elevated"
            :disabled="isLoading || props.disabled"
            :loading="isLoading"
            theme="dark"
            @click="saveHardwareSetup"
          >
            APPLY DEFAULT HARDWARE SETUP
          </v-btn>
        </div>
      </div>

      <!-- Advanced Setup -->
      <div v-else>
        <!-- Focus Group -->
        <ExpansiblePanel
          title="Focus"
          expanded
          theme="dark"
        >
          <BlueSelect
            v-model="intendedFocusAndZoomParams.focus_channel"
            :disabled="props.disabled"
            label="PWM Output Channel"
            :items="availableServoChannelOptions"
            :error-messages="channelErrors.focus_channel ? [channelErrors.focus_channel] : []"
            theme="dark"
            @update:model-value="handleChannelChanges('focus_channel', $event)"
          />
          <div class="d-flex flex-row ga-3 mt-5">
            <v-text-field
              v-model.number="intendedFocusAndZoomParams.focus_channel_min"
              :disabled="props.disabled"
              label="Min (µs)"
              type="number"
              density="compact"
              hide-details
              theme="dark"
              variant="outlined"
            />
            <v-text-field
              v-model.number="intendedFocusAndZoomParams.focus_channel_trim"
              :disabled="props.disabled"
              label="Trim (µs)"
              type="number"
              density="compact"
              hide-details
              theme="dark"
              variant="outlined"
            />
            <v-text-field
              v-model.number="intendedFocusAndZoomParams.focus_channel_max"
              :disabled="props.disabled"
              label="Max (µs)"
              type="number"
              density="compact"
              hide-details
              theme="dark"
              variant="outlined"
            />
          </div>
          <v-text-field
            v-model.number="intendedFocusAndZoomParams.focus_margin_gain"
            :disabled="props.disabled"
            type="number"
            label="Focus Margin Gain"
            density="compact"
            hide-details
            theme="dark"
            variant="outlined"
            class="mt-5"
          />
        </ExpansiblePanel>

        <!-- Zoom Group -->
        <ExpansiblePanel
          title="Zoom"
          expanded
          theme="dark"
        >
          <BlueSelect
            v-model="intendedFocusAndZoomParams.zoom_channel"
            :disabled="props.disabled"
            label="PWM Output Channel"
            :items="availableServoChannelOptions"
            :error-messages="channelErrors.zoom_channel ? [channelErrors.zoom_channel] : []"
            theme="dark"
            @update:model-value="handleChannelChanges('zoom_channel', $event)"
          />
          <div class="d-flex flex-row ga-3 mt-5">
            <v-text-field
              v-model.number="intendedFocusAndZoomParams.zoom_channel_min"
              :disabled="props.disabled"
              label="Min (µs)"
              type="number"
              density="compact"
              hide-details
              theme="dark"
              variant="outlined"
            />
            <v-text-field
              v-model.number="intendedFocusAndZoomParams.zoom_channel_trim"
              :disabled="props.disabled"
              label="Trim (µs)"
              type="number"
              density="compact"
              hide-details
              theme="dark"
              variant="outlined"
            />
            <v-text-field
              v-model.number="intendedFocusAndZoomParams.zoom_channel_max"
              :disabled="props.disabled"
              label="Max (µs)"
              type="number"
              density="compact"
              hide-details
              theme="dark"
              variant="outlined"
            />
          </div>
        </ExpansiblePanel>

        <!-- Script Group -->
        <ExpansiblePanel
          title="Script"
          expanded
          theme="dark"
        >
          <BlueSelect
            v-model="intendedFocusAndZoomParams.script_channel"
            :disabled="props.disabled"
            label="PWM Input Channel"
            :items="availableServoChannelOptions"
            :error-messages="channelErrors.script_channel ? [channelErrors.script_channel] : []"
            theme="dark"
            @update:model-value="handleChannelChanges('script_channel', $event)"
          />
          <div class="d-flex flex-row ga-3 mt-5">
            <v-text-field
              v-model.number="intendedFocusAndZoomParams.script_channel_min"
              :disabled="props.disabled"
              label="Min (µs)"
              type="number"
              density="compact"
              hide-details
              theme="dark"
              variant="outlined"
            />
            <v-text-field
              v-model.number="intendedFocusAndZoomParams.script_channel_trim"
              :disabled="props.disabled"
              label="Trim (µs)"
              type="number"
              density="compact"
              hide-details
              theme="dark"
              variant="outlined"
            />
            <v-text-field
              v-model.number="intendedFocusAndZoomParams.script_channel_max"
              :disabled="props.disabled"
              label="Max (µs)"
              type="number"
              density="compact"
              hide-details
              theme="dark"
              variant="outlined"
            />
          </div>
          <div class="d-flex flex-column ga-4 mt-4">
            <BlueSelect
              v-model="intendedFocusAndZoomParams.script_function"
              :disabled="props.disabled"
              label="Script Function"
              :items="scriptFunctionOptions"
              theme="dark"
              item-title="name"
              item-value="value"
            />
            <BlueSelect
              v-model="intendedFocusAndZoomParams.camera_id"
              :disabled="props.disabled"
              label="Camera ID"
              :items="cameraIdOptions"
              theme="dark"
              item-title="name"
              item-value="value"
            />
            <BlueSwitch
              v-model="intendedFocusAndZoomParams.enable_focus_and_zoom_correlation"
              :disabled="props.disabled"
              name="focus-zoom-correlation"
              label="Focus/Zoom Correlation"
              theme="dark"
            />
          </div>
        </ExpansiblePanel>

        <!-- Tilt Group -->
        <ExpansiblePanel
          title="Tilt"
          expanded
          theme="dark"
        >
          <BlueSelect
            v-model="intendedFocusAndZoomParams.tilt_channel"
            :disabled="props.disabled"
            label="PWM Output Channel"
            :items="availableServoChannelOptions"
            :error-messages="channelErrors.tilt_channel ? [channelErrors.tilt_channel] : []"
            theme="dark"
            @update:model-value="handleChannelChanges('tilt_channel', $event)"
          />
          <div class="d-flex flex-row ga-3 mt-5">
            <v-text-field
              v-model.number="intendedFocusAndZoomParams.tilt_channel_min"
              :disabled="props.disabled"
              label="Min (µs)"
              type="number"
              density="compact"
              hide-details
              theme="dark"
              variant="outlined"
            />
            <v-text-field
              v-model.number="intendedFocusAndZoomParams.tilt_channel_trim"
              :disabled="props.disabled"
              label="Trim (µs)"
              type="number"
              density="compact"
              hide-details
              theme="dark"
              variant="outlined"
            />
            <v-text-field
              v-model.number="intendedFocusAndZoomParams.tilt_channel_max"
              :disabled="props.disabled"
              label="Max (µs)"
              type="number"
              density="compact"
              hide-details
              theme="dark"
              variant="outlined"
            />
          </div>
          <div class="d-flex flex-row ga-3 pt-4">
            <v-text-field
              v-model.number="intendedFocusAndZoomParams.tilt_mnt_pitch_min"
              :disabled="props.disabled"
              label="Pitch Min (°)"
              type="number"
              density="compact"
              hide-details
              theme="dark"
              variant="outlined"
            />
            <v-text-field
              v-model.number="intendedFocusAndZoomParams.tilt_mnt_pitch_max"
              :disabled="props.disabled"
              label="Pitch Max (°)"
              type="number"
              density="compact"
              hide-details
              theme="dark"
              variant="outlined"
            />
          </div>
          <div class="d-flex flex-column ga-4 mt-4">
            <BlueSwitch
              v-model="intendedFocusAndZoomParams.tilt_channel_reversed"
              :disabled="props.disabled"
              name="tilt-channel-reversed"
              label="Reverse Direction"
              theme="dark"
            />
            <BlueSelect
              v-model="intendedFocusAndZoomParams.tilt_mnt_type"
              :disabled="props.disabled"
              label="Mount Type"
              :items="mountTypeOptions"
              theme="dark"
              item-title="name"
              item-value="value"
            />
          </div>
        </ExpansiblePanel>

        <!-- Action Buttons -->
        <div class="d-flex flex-row ga-3 mt-5 justify-end">
          <v-btn
            :disabled="props.disabled"
            class="py-1 px-3 ml-4 rounded-md bg-[#414141] hover:bg-[#0A3E6B]"
            size="small"
            variant="elevated"
            theme="dark"
            @click="showAdvancedHardware = false"
          >
            Back to simple
          </v-btn>
          <v-btn
            class="py-1 px-3 ml-4 rounded-md bg-[#0B5087] hover:bg-[#0A3E6B]"
            size="small"
            variant="elevated"
            :disabled="hasChannelErrors || isLoading || props.disabled"
            :loading="isLoading"
            theme="dark"
            @click="saveHardwareSetup"
          >
            APPLY CUSTOM HARDWARE SETUP
          </v-btn>
        </div>
      </div>
    </ExpansiblePanel>
  </div>
  
  <WelcomeDialog
    :show="(!isConfigured) && showWelcomeDialog"
    @close="showWelcomeDialog = false"
  />
  <Loading :is-loading="isLoading" />
  <ErrorDialog
    :message="errorDialogMessage"
    @close="errorDialogMessage = null"
  />
  <WarningToast :message="warningToastMessage" />
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import BlueButtonGroup from './BlueButtonGroup.vue'
import BlueSlider from './BlueSlider.vue'
import BlueSwitch from './BlueSwitch.vue'
import ExpansiblePanel from './ExpansiblePanel.vue'
import BlueSelect from './BlueSelect.vue'
import Loading from './Loading.vue'
import { VideoChannelValue, type BaseParameterSetting, type VideoParameterSettings, type VideoResolutionValue, BaseAutoWhiteBalanceModeValue, BaseAutoWhiteBalanceSceneValue, type AdvancedParameterSetting, type CameraControl } from '@/bindings/radcam'
import axios from 'axios'
import type { ActuatorsConfig, ActuatorsControl, ActuatorsParametersConfig, ActuatorsState, CameraID, MountType, ScriptFunction, ServoChannel } from '@/bindings/autopilot'
import { applyNonNull } from '@/utils/jsonUtils'
import ErrorDialog from './ErrorDialog.vue'
import WelcomeDialog from './WelcomeDialog.vue'
import { OneMoreTime } from '@/utils/oneMoreTime'


const props = defineProps<{
  selectedCameraUuid: string | null
  backendApi: string
  disabled: boolean
  cockpitMode: boolean
}>()

interface ServoChannelOption {
  name: string
  value: ServoChannel
}

const servoChannelOptions: ServoChannelOption[] = Array.from({ length: 16 }, (_, i) => ({
  name: `Channel ${i + 1}`,
  value: `SERVO${i + 1}` as ServoChannel,
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

const currentFocusAndZoomParams = ref<ActuatorsParametersConfig>({
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

const selectedVideoResolution = ref<VideoResolutionValue | null>(null)
const selectedVideoBitrate = ref<number | null>(null)
const selectedVideoParameters = ref<VideoParameterSettings>({})
const downloadedVideoParameters = ref<VideoParameterSettings>({})
const actuatorsState = ref<ActuatorsState>({
  focus: 0,
  zoom: 0,
  tilt: 0,
})
const isConfigured = ref<boolean>(true)
const showWelcomeDialog = ref<boolean>(true)
const isLoading = ref<boolean>(false)
const errorDialogMessage = ref<string | null>(null)
const warningToastMessage = ref<string | null>(null)
const showAdvancedHardware = ref(false)
const intendedFocusAndZoomParams = ref<ActuatorsParametersConfig>({
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
const defaultFocusAndZoomParams = ref<ActuatorsParametersConfig>({
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
const processingWhiteBalance = ref(false)

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

const WhiteBalanceSceneButtonItems = computed(() => [
  { 
    name: 'Green',
    preSelected: baseParams.value.awb_auto_mode === BaseAutoWhiteBalanceSceneValue.Scene1,
    onSelected: () => (updateBaseParameter('awb_auto_mode', BaseAutoWhiteBalanceSceneValue.Scene1))
  },
  { 
    name: 'Blue',
    preSelected: baseParams.value.awb_auto_mode === BaseAutoWhiteBalanceSceneValue.Scene2,
    onSelected: () => (updateBaseParameter('awb_auto_mode', BaseAutoWhiteBalanceSceneValue.Scene2))
  }
])

const whiteBalanceModeButtonItems = computed(() => [
  { 
    name: 'Auto',
    preSelected: baseParams.value.auto_awb === BaseAutoWhiteBalanceModeValue.Auto,
    onSelected: () => (updateBaseParameter('auto_awb', BaseAutoWhiteBalanceModeValue.Auto))
  },
  { 
    name: 'Manual',
    preSelected: baseParams.value.auto_awb === BaseAutoWhiteBalanceModeValue.Manual,
    onSelected: () => (updateBaseParameter('auto_awb', BaseAutoWhiteBalanceModeValue.Manual))
  }
])


const hasUnsavedChanges = computed(() => {
  const current = currentFocusAndZoomParams.value
  const intended = intendedFocusAndZoomParams.value

  // Compare every field
  for (const key in current) {
    if (Object.prototype.hasOwnProperty.call(current, key)) {
      const currentVal = current[key as keyof ActuatorsParametersConfig]
      const intendedVal = intended[key as keyof ActuatorsParametersConfig]

      // Handle null/undefined equality
      if (currentVal !== intendedVal) {
        return true
      }
    }
  }
  return false
})

const channelErrors = computed(() => {
  const errors: Record<keyof Pick<ActuatorsParametersConfig, 'focus_channel' | 'zoom_channel' | 'tilt_channel' | 'script_channel'>, string | null> = {
    focus_channel: null,
    zoom_channel: null,
    tilt_channel: null,
    script_channel: null,
  }

  const channels = [
    'focus_channel',
    'zoom_channel',
    'tilt_channel',
    'script_channel',
  ] as const

  // Check required
  for (const key of channels) {
    if (intendedFocusAndZoomParams.value[key] == null) {
      errors[key] = 'Required'
    }
  }

  // Check duplicates (only if all are selected)
  const selected = channels.map(k => intendedFocusAndZoomParams.value[k]).filter(c => c !== null)
  if (new Set(selected).size !== selected.length) {
    // Mark duplicates
    const seen = new Set<ServoChannel>()
    for (const key of channels) {
      const val = intendedFocusAndZoomParams.value[key]
      if (val === null) continue
      if (seen.has(val)) {
        errors[key] = 'Duplicate channel'
      } else {
        seen.add(val)
      }
    }
  }

  return errors
})

const hasChannelErrors = computed(() => 
  Object.values(channelErrors.value).some(err => err !== null)
)

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

const cameraIdOptions = [
  { name: 'CAM1', value: 'CAM1' },
  { name: 'CAM2', value: 'CAM2' },
] satisfies { name: string; value: CameraID }[];

const scriptFunctionOptions = Array.from({ length: 16 }, (_, i) => ({
  name: `SCRIPT${i + 1}`,
  value: `SCRIPT${i + 1}` as ScriptFunction,
}));

const mountTypeOptions = [
  { name: 'Servo', value: 'Servo' },
  { name: 'Brushless PWM', value: 'BrushlessPWM' },
] satisfies { name: string; value: MountType }[];

// Focus: raw [0–100] → distance [0.5 – 50]
const scaleFocus = (raw: number): number => {
  const ratio = raw / 100
  const distance = (0.5 / (1 - ratio))
  return Math.min(distance, 50)
}

const unscaleFocus = (scaled: number): number => {
  return 100 * (1 - 0.5 / scaled)
}

const formatFocusValue = (distance: number): string => {
  if (distance >= 50) {
    return '50m+'
  }

  if (distance < 1) {
    return `${distance.toFixed(2)}m`
  }
  if (distance < 10) {
    return `${distance.toFixed(1)}m`
  } else {
    return `${Math.round(distance)}m`
  }
}

const scaleZoom = (raw: number): number => 1.0 + (raw / 100) * 2.0
const unscaleZoom = (scaled: number): number => ((scaled - 1.0) / 2.0) * 100
const formatZoomValue = (zoomLevel: number): string => {
  return `${zoomLevel.toFixed(1)}x`
}

const scaleTilt = (raw: number): number => {
  const minAngle = currentFocusAndZoomParams.value.tilt_mnt_pitch_min ?? -90
  const maxAngle = currentFocusAndZoomParams.value.tilt_mnt_pitch_max ?? 90
  return minAngle + (raw / 100) * (maxAngle - minAngle)
}

const unscaleTilt = (scaled: number): number => {
  const minAngle = currentFocusAndZoomParams.value.tilt_mnt_pitch_min ?? -90
  const maxAngle = currentFocusAndZoomParams.value.tilt_mnt_pitch_max ?? 90
  if (maxAngle === minAngle) return 0
  return 100 * (scaled - minAngle) / (maxAngle - minAngle)
}

const formatTiltValue = (angle: number): string => {
  return `${angle.toFixed(1)}°`
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

  axios
    .post(`${props.backendApi}/camera/control`, payload)
    .then((response) => {
      baseParams.value = response.data as BaseParameterSetting
    })
    .catch((error) => {
      const message = `Error sending ${String(param)} control with value '${value}'`
      console.log(message, error.message)
      showWarningToast(message, error)
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

  axios
    .post(`${props.backendApi}/autopilot/control`, payload)
    .then((response) => {
      const newParams = (response.data as ActuatorsConfig)?.parameters
      if (newParams) {
        currentFocusAndZoomParams.value = { ...newParams }

        // Only update intended if user hasn't made changes
        if (!hasUnsavedChanges.value) {
          intendedFocusAndZoomParams.value = { ...newParams }
        }
      } else {
        console.warn("Received null 'parameters' from response:", response.data)
      }
      console.log('# - getActuatorsConfig response:', response.data)

    })
    .catch((error) => {
      const message = 'Error getting actuator configuration'
      console.log(message, error.message)
      showWarningToast(message, error)
    })
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const checkIfConfigured = (error: any) => {
  const details = error.response?.data?.message || error.response?.data || error.response || error.message || 'Unknown error'

    if (typeof details === 'string' && details.toLowerCase().includes('actuators not configured')) {
      isConfigured.value = false
    } else {
      isConfigured.value = true
    }

    return isConfigured.value
}

const getActuatorsDefaultConfig = () => {
  // Only fetch once: if we don't have defaults yet
  if (!props.selectedCameraUuid || defaultFocusAndZoomParams.value.camera_id !== null) {
    return
  }

  const payload = {
    camera_uuid: props.selectedCameraUuid,
    action: "getActuatorsDefaultConfig",
  }

  axios
    .post(`${props.backendApi}/autopilot/control`, payload)
    .then((response) => {
      const newParams = (response.data as ActuatorsConfig)?.parameters
      if (newParams) {
        defaultFocusAndZoomParams.value = { ...newParams }
        // Initialize intended only if not already set
        if (intendedFocusAndZoomParams.value.camera_id === null) {
          intendedFocusAndZoomParams.value = { ...newParams }
        }
      }
      console.log('# - getActuatorsDefaultConfig response:', response.data)

    })
    .catch((error) => {
      const message = 'Error getting actuators default configuration'
      console.log(message, error.message)
      showWarningToast(message, error)
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

  axios
    .post(`${props.backendApi}/autopilot/control`, payload)
    .then((response) => {
      const newParams = (response.data as ActuatorsConfig)?.parameters
      if (newParams) {
        currentFocusAndZoomParams.value = { ...newParams }
      } else {
        console.warn("Received null 'parameters' from response:", response.data)
      }
    })
    .catch((error) => {
      const message = `Error sending ${String(param)} control with value '${value}'`
      console.log(message, error.message)
      showErrorDialog(message, error)
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

  axios
    .post(`${props.backendApi}/autopilot/control`, payload)
    .then((response) => {
      const state = response.data as ActuatorsState

      applyNonNull(actuatorsState.value, state)
      console.log(state)
    })
    .catch((error) => {
      const message = 'Error getting actuators state'
      console.log(message, error.message)
      showWarningToast(message, error)
    })
}

const updateActuatorsState = (param: keyof ActuatorsState, value: number) => {
  if (!props.selectedCameraUuid) return

  const payload: ActuatorsControl = {
    camera_uuid: props.selectedCameraUuid,
    action: "setActuatorsState",
    json: { [param]: value } as ActuatorsState
  }

  axios
    .post(`${props.backendApi}/autopilot/control`, payload)
    .then((response) => { 
      const state = response.data as ActuatorsState;

      applyNonNull(actuatorsState.value, state)
      console.log(state)
    })
    .catch((error) => {
      const message = `Error updating ${param}`
      console.log(message, error.message)
      showWarningToast(message, error)
    })
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const showErrorDialog = (message: string, error: any) => {
  if (error.response?.status === 404 || !checkIfConfigured(error)) return

  const details = error.response?.data?.message || error.response?.data || error.response || error.message || error || 'Unknown error'
  errorDialogMessage.value = `${message}: ${details}`
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const showWarningToast = (message: string, error: any) => {
  if (error.response?.status === 404 || !checkIfConfigured(error)) return
  
  const details = error.response?.data?.message || error.response?.data || error.response || error.message || error || 'Unknown error'
  warningToastMessage.value = `${message}: ${details}`
}

const isHardwareSetupComplete = computed<boolean>(() => {
  return (
    intendedFocusAndZoomParams.value.focus_channel !== null &&
    intendedFocusAndZoomParams.value.zoom_channel !== null &&
    intendedFocusAndZoomParams.value.tilt_channel !== null &&
    intendedFocusAndZoomParams.value.script_channel !== null
  )
})

const availableServoChannelOptions = computed(() => {
  const selectedChannels = new Set([
    intendedFocusAndZoomParams.value.focus_channel,
    intendedFocusAndZoomParams.value.zoom_channel,
    intendedFocusAndZoomParams.value.tilt_channel,
    intendedFocusAndZoomParams.value.script_channel
  ].filter(channel => channel !== null))

  return servoChannelOptions.map(option => ({
    ...option,
    disabled: selectedChannels.has(option.value) &&
      option.value !== intendedFocusAndZoomParams.value.focus_channel &&
      option.value !== intendedFocusAndZoomParams.value.zoom_channel &&
      option.value !== intendedFocusAndZoomParams.value.tilt_channel &&
      option.value !== intendedFocusAndZoomParams.value.script_channel
  }))
})

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const handleChannelChanges = (param: keyof ActuatorsParametersConfig, value: any): void => {
  if (!props.selectedCameraUuid) return

  // Optional: prevent duplicates (though UI should disable them)
  const isAlreadySelected = Object.entries(intendedFocusAndZoomParams.value).some(
    ([key, val]) => key !== param && val === value
  )

  if (isAlreadySelected && value !== null) {
    console.warn(`Channel ${value} is already in use`)
    return
  }

  intendedFocusAndZoomParams.value[param] = value
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
    .catch((error) => {
      const message = 'Error getting video parameters'
      console.log(message, error.message)
      showWarningToast(message, error)
    })
    
}

const getBaseParameters = () => {
  if (!props.selectedCameraUuid) {
    return
  }

  const payload = {
    camera_uuid: props.selectedCameraUuid,
    action: "getImageAdjustment",
  }

  axios.post(`${props.backendApi}/camera/control`, payload)
    .then(response => {
      baseParams.value = response.data as BaseParameterSetting
      console.log(response.data)
    })
    .catch(error => {
      console.error(`Error sending getImageAdjustment request:`, error.message)
    })
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
      const message = `Error sending partial video params '${JSON.stringify(partial)}'`
      console.log(message, error.message)
      showWarningToast(message, error)
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

const doWhiteBalance = async () => {
  if (!props.selectedCameraUuid) {
    return
  }

  // Prevent multiple concurrent white balance operations
  if (processingWhiteBalance.value) return
  processingWhiteBalance.value = true

  const payload: CameraControl = {
    camera_uuid: props.selectedCameraUuid,
    action: "setImageAdjustmentEx",
    json: {
      onceAWB: 1,
    } as AdvancedParameterSetting,
  }

  axios.post(`${props.backendApi}/camera/control`, payload)
    .catch(error => {
      console.error("Error sending onceAWB control:", error.message)
    }).finally(() => {
      processingWhiteBalance.value = false
      getBaseParameters()
    })
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
    .catch((error) => {
      const message = 'Error sending restart'
      console.log(message, error.message)
      showErrorDialog(message, error)
    })
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
  if (!props.selectedCameraUuid || !defaultFocusAndZoomParams.value.camera_id || !intendedFocusAndZoomParams.value) return

  if (!isHardwareSetupComplete.value) {
    console.error('All channel selections are required')
    return
  }

  isLoading.value = true
  
  let payloadParams: ActuatorsParametersConfig
  payloadParams = { ...defaultFocusAndZoomParams.value }
  if (showAdvancedHardware.value) {
    payloadParams = { ...intendedFocusAndZoomParams.value }
  }

  const payload: ActuatorsControl = {
    camera_uuid: props.selectedCameraUuid,
    action: "setActuatorsConfig",
    json: { parameters: payloadParams } as ActuatorsConfig
  }

  console.log('Saving hardware setup:', payload)

  axios
    .post(`${props.backendApi}/autopilot/control`, payload)
    .then((response) => {
      console.log("Got an answer from the setActuatorsConfig request", response.data)

      const newParams = (response.data as ActuatorsConfig)?.parameters
      if (newParams) {
        currentFocusAndZoomParams.value = { ...newParams }
        intendedFocusAndZoomParams.value = { ...newParams }
      }
    })
    .catch((error) => {
      const message = 'Error saving hardware setup'
      console.log(message, error.message)
      showErrorDialog(message, error)
    })
    .finally(() => {
      isLoading.value = false
    })
}

const getCameraStates = () => {
  getActuatorsDefaultConfig()
  getActuatorsConfig()
  getActuatorsState()
  getVideoParameters(true)
  getBaseParameters()
}

defineExpose({ getCameraStates: getCameraStates })

watch(
  () => props.selectedCameraUuid,
  async (newValue) => {
    if (newValue) {
      getCameraStates()
    }
  }
)

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

watch(warningToastMessage, (newVal) => {
  if (newVal) {
    setTimeout(() => {
      warningToastMessage.value = null
    }, 5000)
  }
})

new OneMoreTime({ delay: 1000, errorDelay: 5000, autostart: true, disposeWith: this }, getCameraStates);

</script>