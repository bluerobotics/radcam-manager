<template>
  <!-- ImageParameters Sliders -->
  <Slider
    name="hue"
    label="Hue"
    :current="baseParams.hue ?? 0"
    :min="0"
    :max="255"
    :step="1"
    :disabled="props.disabled"
    @update:current="updateBaseParameter('hue', $event)"
  />
  <Slider
    name="brightness"
    label="Brightness"
    :current="baseParams.brightness ?? 0"
    :min="0"
    :max="255"
    :step="1"
    :disabled="props.disabled"
    @update:current="updateBaseParameter('brightness', $event)"
  />
  <Slider
    name="sharpness"
    label="Sharpness"
    :current="baseParams.sharpness ?? 0"
    :min="0"
    :max="255"
    :step="1"
    :disabled="props.disabled"
    @update:current="updateBaseParameter('sharpness', $event)"
  />
  <Slider
    name="contrast"
    label="Contrast"
    :current="baseParams.contrast ?? 0"
    :min="0"
    :max="255"
    :step="1"
    :disabled="props.disabled"
    @update:current="updateBaseParameter('contrast', $event)"
  />
  <Slider
    name="saturation"
    label="Saturation"
    :current="baseParams.saturation ?? 0"
    :min="0"
    :max="255"
    :step="1"
    :disabled="props.disabled"
    @update:current="updateBaseParameter('saturation', $event)"
  />
  <Slider
    name="gamma"
    label="Gamma"
    :current="baseParams.gamma ?? 0"
    :min="0"
    :max="255"
    :step="1"
    :disabled="props.disabled"
    @update:current="updateBaseParameter('gamma', $event)"
  />
  <Slider
    name="blc_level"
    label="Backlight Compensation"
    :current="baseParams.blc_level ?? 0"
    :min="0"
    :max="255"
    :step="1"
    :disabled="props.disabled"
    @update:current="updateBaseParameter('blc_level', $event)"
  />

  <!-- Restore Image Parameters -->
  <div class="ma-2 text-right">
    <v-btn
      variant="tonal"
      :disabled="props.disabled || processingBaseRestore"
      @click="doRestoreBase"
    >
      <v-progress-circular
        v-if="processingBaseRestore"
        indeterminate
        color="white"
        size="20"
        class="me-2"
      />
      {{ processingBaseRestore ? "Processing..." : "Restore defaults" }}
    </v-btn>
  </div>
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

  <v-expansion-panels>
    <!-- White Balance -->
    <v-expansion-panel>
      <v-expansion-panel-title>White Balance</v-expansion-panel-title>
      <v-expansion-panel-text>
        <v-select
          :model-value="baseParams.auto_awb"
          :items="autoWhiteBalanceModeOptions"
          label="White Balance Mode"
          :disabled="props.disabled"
          item-title="text"
          item-value="value"
          class="mb-4"
          @update:model-value="updateBaseParameter('auto_awb', $event)"
        />
        <v-select
          v-if="baseParams.auto_awb === BaseAutoWhiteBalanceModeValue.Auto"
          :model-value="baseParams.awb_auto_mode"
          :items="autoWhiteBalanceSceneOptions"
          label="White Balance Scene"
          :disabled="props.disabled"
          item-title="text"
          item-value="value"
          class="mb-4"
          @update:model-value="updateBaseParameter('awb_auto_mode', $event)"
        />
        <div
          v-if="baseParams.auto_awb === BaseAutoWhiteBalanceModeValue.Manual"
        >
          <Slider
            name="awb_red"
            label="White Balance Red"
            :current="baseParams.awb_red ?? 0"
            :min="0"
            :max="255"
            :step="1"
            :disabled="props.disabled"
            @update:current="updateBaseParameter('awb_red', $event)"
          />
          <Slider
            name="awb_green"
            label="White Balance Green"
            :current="baseParams.awb_green ?? 0"
            :min="0"
            :max="255"
            :step="1"
            :disabled="props.disabled"
            @update:current="updateBaseParameter('awb_green', $event)"
          />
          <Slider
            name="awb_blue"
            label="White Balance Blue"
            :current="baseParams.awb_blue ?? 0"
            :min="0"
            :max="255"
            :step="1"
            :disabled="props.disabled"
            @update:current="updateBaseParameter('awb_blue', $event)"
          />
        </div>
        <Slider
          name="awb_style_red"
          label="White Balance Style Red"
          :current="baseParams.awb_style_red ?? 0"
          :min="0"
          :max="255"
          :step="1"
          :disabled="props.disabled"
          @update:current="updateBaseParameter('awb_style_red', $event)"
        />
        <Slider
          name="awb_style_green"
          label="White Balance Style Green"
          :current="baseParams.awb_style_green ?? 0"
          :min="0"
          :max="255"
          :step="1"
          :disabled="props.disabled"
          @update:current="updateBaseParameter('awb_style_green', $event)"
        />
        <Slider
          name="awb_style_blue"
          label="White Balance Style Blue"
          :current="baseParams.awb_style_blue ?? 0"
          :min="0"
          :max="255"
          :step="1"
          :disabled="props.disabled"
          @update:current="updateBaseParameter('awb_style_blue', $event)"
        />
      </v-expansion-panel-text>
    </v-expansion-panel>
    
    <!-- Exposure & Gain -->
    <v-expansion-panel>
      <v-expansion-panel-title>Exposure & Gain</v-expansion-panel-title>
      <v-expansion-panel-text>
        <v-switch
          :model-value="advancedParams.low_farme_rate === AdvancedDisplayLowFramerateValue.Open"
          label="Slow Shutter"
          :disabled="props.disabled"
          @update:model-value="updateAdvancedParam('low_farme_rate', $event ? AdvancedDisplayLowFramerateValue.Open : AdvancedDisplayLowFramerateValue.Close)"
        />
        <v-select
          :model-value="baseParams.auto_gain_mode"
          :items="autoGainModeOptions"
          label="Automatic Gain Mode"
          :disabled="props.disabled"
          item-title="text"
          item-value="value"
          class="mb-4"
          @update:model-value="updateBaseParameter('auto_gain_mode', $event)"
        />
        <div
          v-if="baseParams.auto_gain_mode === BaseAutoGainModeValue.Auto"
        >
          <Slider
            name="auto_d_gain_max"
            label="Automatic Maximum D Gain"
            :current="baseParams.auto_DGain_max ?? 0"
            :min="0"
            :max="255"
            :step="1"
            :disabled="props.disabled"
            @update:current="updateBaseParameter('auto_DGain_max', $event)"
          />
          <Slider
            name="auto_a_gain_max"
            label="Automatic Maximum A Gain"
            :current="baseParams.auto_AGain_max ?? 0"
            :min="0"
            :max="255"
            :step="1"
            :disabled="props.disabled"
            @update:current="updateBaseParameter('auto_AGain_max', $event)"
          />
        </div>
        <div
          v-if="baseParams.auto_gain_mode === BaseAutoGainModeValue.Manual"
        >
          <v-switch
            :model-value="baseParams.manual_AGain_enable === BaseManualAGainEnableValue.Open"
            label="Enable Manual A Gain"
            :disabled="props.disabled"
            @update:model-value="updateBaseParameter('manual_AGain_enable', $event ? BaseManualAGainEnableValue.Open : BaseManualAGainEnableValue.Close)"
          />
          <Slider
            v-if="baseParams.manual_AGain_enable === BaseManualAGainEnableValue.Open"
            name="manual_a_gain"
            label="Manual A Gain"
            :current="baseParams.manual_AGain ?? 0"
            :min="0"
            :max="255"
            :step="1"
            :disabled="props.disabled"
            @update:current="updateBaseParameter('manual_AGain', $event)"
          />
          <v-switch
            :model-value="baseParams.manual_DGain_enable === BaseManualDGainEnableValue.Open"
            label="Enable Manual D Gain"
            :disabled="props.disabled"
            @update:model-value="updateBaseParameter('manual_DGain_enable', $event ? BaseManualDGainEnableValue.Open : BaseManualDGainEnableValue.Close)"
          />
          <Slider
            v-if="baseParams.manual_DGain_enable === BaseManualDGainEnableValue.Open"
            name="manual_d_gain"
            label="Manual D Gain"
            :current="baseParams.manual_DGain ?? 0"
            :min="0"
            :max="255"
            :step="1"
            :disabled="props.disabled"
            @update:current="updateBaseParameter('manual_DGain', $event)"
          />
        </div>
        <Slider
          name="max_sys_gain"
          label="Max System Gain"
          :current="baseParams.max_sys_gain ?? 0"
          :min="0"
          :max="255"
          :step="1"
          :disabled="props.disabled"
          @update:current="updateBaseParameter('max_sys_gain', $event)"
        />

        <v-select
          :model-value="baseParams.AE_strategy_mode"
          :items="autoExposureStrategyModeOptions"
          label="Exposure Strategy Mode"
          :disabled="props.disabled"
          item-title="text"
          item-value="value"
          class="mb-4"
          @update:model-value="updateBaseParameter('AE_strategy_mode', $event)"
        />
        <v-select
          :model-value="baseParams.auto_exposureEx"
          :items="exposureModeOptions"
          label="Exposure Mode"
          :disabled="props.disabled"
          item-title="text"
          item-value="value"
          class="mb-4"
          @update:model-value="updateBaseParameter('auto_exposureEx', $event)"
        />
        <v-select
          v-if="baseParams.auto_exposureEx === BaseExposureModeValue.Auto"
          :model-value="baseParams.max_exposure"
          :items="maxExposureOptions"
          label="Maximum Exposure Time"
          :disabled="props.disabled"
          item-title="text"
          item-value="value"
          class="mb-4"
          @update:model-value="updateBaseParameter('max_exposure', $event)"
        />
        <v-select
          v-if="baseParams.auto_exposureEx === BaseExposureModeValue.Manual"
          :model-value="baseParams.exposure_time"
          :items="exposureTimeOptions"
          label="Manual Exposure Time"
          :disabled="props.disabled"
          item-title="text"
          item-value="value"
          class="mb-4"
          @update:model-value="updateBaseParameter('exposure_time', $event)"
        />
      </v-expansion-panel-text>
    </v-expansion-panel>

    <!-- Infrared -->
    <v-expansion-panel>
      <v-expansion-panel-title>Infrared</v-expansion-panel-title>
      <v-expansion-panel-text>
        <v-select
          :model-value="advancedParams.ircut_level"
          :items="ircutLevelOptions"
          label="IRCUT Level"
          :disabled="props.disabled"
          item-title="text"
          item-value="value"
          @update:model-value="updateAdvancedParam('ircut_level', $event)"
        />
        <v-select
          :model-value="advancedParams.ldr_level"
          :items="ldrLevelOptions"
          label="Photosensitive Level"
          :disabled="props.disabled"
          item-title="text"
          item-value="value"
          @update:model-value="updateAdvancedParam('ldr_level', $event)"
        />
        <v-select
          :model-value="advancedParams.lamp_type"
          :items="lampTypeOptions"
          label="Lamp Type"
          :disabled="props.disabled"
          item-title="text"
          item-value="value"
          @update:model-value="updateAdvancedParam('lamp_type', $event)"
        />
        <v-select
          :model-value="advancedParams.led_control_avail"
          :items="ledControlAvailOptions"
          label="Light Enable Level"
          :disabled="props.disabled"
          item-title="text"
          item-value="value"
          @update:model-value="updateAdvancedParam('led_control_avail', $event)"
        />
        <v-select
          :model-value="advancedParams.led_control"
          :items="ledControlOptions"
          label="IR Control"
          :disabled="props.disabled"
          item-title="text"
          item-value="value"
          @update:model-value="updateAdvancedParam('led_control', $event)"
        />
        <Slider
          name="sens_day_to_night"
          label="Day to Night Sensitivity"
          :current="advancedParams.sens_day_to_night ?? 0"
          :min="0"
          :max="255"
          :step="1"
          :disabled="props.disabled"
          @update:current="updateAdvancedParam('sens_day_to_night', $event)"
        />
        <Slider
          name="sens_night_to_day"
          label="Night to Day Sensitivity"
          :current="advancedParams.sens_night_to_day ?? 0"
          :min="0"
          :max="255"
          :step="1"
          :disabled="props.disabled"
          @update:current="updateAdvancedParam('sens_night_to_day', $event)"
        />
        <Slider
          name="infr_day_h"
          label="Infrared Day Start Hour"
          :current="advancedParams.infr_day_h ?? 0"
          :min="0"
          :max="23"
          :step="1"
          :disabled="props.disabled"
          @update:current="updateAdvancedParam('infr_day_h', $event)"
        />
        <Slider
          name="infr_day_m"
          label="Infrared Day Start Minute"
          :current="advancedParams.infr_day_m ?? 0"
          :min="0"
          :max="59"
          :step="1"
          :disabled="props.disabled"
          @update:current="updateAdvancedParam('infr_day_m', $event)"
        />
        <Slider
          name="infr_night_h"
          label="Infrared Night Start Hour"
          :current="advancedParams.infr_night_h ?? 0"
          :min="0"
          :max="23"
          :step="1"
          :disabled="props.disabled"
          @update:current="updateAdvancedParam('infr_night_h', $event)"
        />
        <Slider
          name="infr_night_m"
          label="Infrared Night Start Minute"
          :current="advancedParams.infr_night_m ?? 0"
          :min="0"
          :max="59"
          :step="1"
          :disabled="props.disabled"
          @update:current="updateAdvancedParam('infr_night_m', $event)"
        />
        <Slider
          name="ir_level"
          label="Infrared Lamp Brightness"
          :current="advancedParams.ir_level ?? 0"
          :min="0"
          :max="255"
          :step="1"
          :disabled="props.disabled"
          @update:current="updateAdvancedParam('ir_level', $event)"
        />
        <Slider
          name="led_level"
          label="White Light Brightness"
          :current="advancedParams.led_level ?? 0"
          :min="0"
          :max="255"
          :step="1"
          :disabled="props.disabled"
          @update:current="updateAdvancedParam('led_level', $event)"
        />
        <Slider
          name="iris_level"
          label="Aperture PWM Duty Cycle"
          :current="advancedParams.irisLevel ?? 0"
          :min="0"
          :max="255"
          :step="1"
          :disabled="props.disabled"
          @update:current="updateAdvancedParam('irisLevel', $event)"
        />
      </v-expansion-panel-text>
    </v-expansion-panel>
    


    <!-- Advanced Parameters -->
    <v-expansion-panel>
      <v-expansion-panel-title>Advanced</v-expansion-panel-title>
      <v-expansion-panel-text>
        <v-select
          :model-value="advancedParams.mirror"
          :items="mirrorOptions"
          label="Mirror"
          :disabled="props.disabled"
          item-title="text"
          item-value="value"
          @update:model-value="updateAdvancedParam('mirror', $event)"
        />
        <v-select
          :model-value="advancedParams.flip"
          :items="flipOptions"
          label="Flip"
          :disabled="props.disabled"
          item-title="text"
          item-value="value"
          @update:model-value="updateAdvancedParam('flip', $event)"
        />
        <v-select
          :model-value="advancedParams.power_freq"
          :items="powerFreqOptions"
          label="Power Frequency"
          :disabled="props.disabled"
          item-title="text"
          item-value="value"
          @update:model-value="updateAdvancedParam('power_freq', $event)"
        />
        <v-switch
          :model-value="advancedParams.color_black === AdvancedDisplayColorBlackValue.Auto"
          label="Auto Color Black"
          :disabled="props.disabled"
          @update:model-value="updateAdvancedParam('color_black', $event ? AdvancedDisplayColorBlackValue.Auto : AdvancedDisplayColorBlackValue.Color)"
        />
        <v-select
          :model-value="advancedParams.infr_detect_mode"
          :items="infrDetectModeOptions"
          label="Video Detection Mode"
          :disabled="props.disabled"
          item-title="text"
          item-value="value"
          @update:model-value="updateAdvancedParam('infr_detect_mode', $event)"
        />
        <v-select
          :model-value="advancedParams.lens_correction"
          :items="lensCorrectionOptions"
          label="Lens Correction"
          :disabled="props.disabled"
          item-title="text"
          item-value="value"
          @update:model-value="updateAdvancedParam('lens_correction', $event)"
        />
        
        <v-select
          :model-value="advancedParams.auto_iris"
          :items="autoIrisOptions"
          label="Aperture Mode"
          :disabled="props.disabled"
          item-title="text"
          item-value="value"
          @update:model-value="updateAdvancedParam('auto_iris', $event)"
        />
        <v-select
          :model-value="advancedParams.noiseReduction"
          :items="noiseReductionOptions"
          label="3D Noise Reduction"
          :disabled="props.disabled"
          item-title="text"
          item-value="value"
          @update:model-value="updateAdvancedParam('noiseReduction', $event)"
        />
        <Slider
          name="wdr_level_sensor"
          label="WDR Strength"
          :current="advancedParams.wdr_level_sensor ?? 0"
          :min="0"
          :max="255"
          :step="1"
          :disabled="props.disabled"
          @update:current="updateAdvancedParam('wdr_level_sensor', $event)"
        />
        <Slider
          name="wdr_level"
          label="Wide Dynamic Strength"
          :current="advancedParams.wdr_level ?? 0"
          :min="0"
          :max="255"
          :step="1"
          :disabled="props.disabled"
          @update:current="updateAdvancedParam('wdr_level', $event)"
        />
        <v-switch
          :model-value="advancedParams.wdr_sensor === AdvancedDisplayWDRSensorValue.Open"
          label="WDR Enable"
          :disabled="props.disabled"
          @update:model-value="updateAdvancedParam('wdr_sensor', $event ? AdvancedDisplayWDRSensorValue.Open : AdvancedDisplayWDRSensorValue.Close)"
        />
        <v-switch
          :model-value="advancedParams.hlc_enable === AdvancedDisplayHlcEnableValue.Open"
          label="HLC Enable"
          :disabled="props.disabled"
          @update:model-value="updateAdvancedParam('hlc_enable', $event ? AdvancedDisplayHlcEnableValue.Open : AdvancedDisplayHlcEnableValue.Close)"
        />

        <v-select
          :model-value="advancedParams._2DNR_level"
          :items="_2dNrLevelOptions"
          label="2D Noise Reduction"
          :disabled="props.disabled"
          item-title="text"
          item-value="value"
          @update:model-value="updateAdvancedParam('_2DNR_level', $event)"
        />
        <v-select
          :model-value="advancedParams.anti_flicker"
          :items="antiFlickerOptions"
          label="Anti Flicker"
          :disabled="props.disabled"
          item-title="text"
          item-value="value"
          @update:model-value="updateAdvancedParam('anti_flicker', $event)"
        />
        <!-- Scene Mode. Note: this is not working, use the `AdvanceParameterSetting::sene_mode` instead -->
        <!-- <v-select
          :model-value="baseParams.sceneMode"
          :items="sceneModeOptions"
          label="Scene Mode"
          :disabled="props.disabled"
          item-title="text"
          item-value="value"
          class="mb-4"
          @update:model-value="updateBaseParameter('sceneMode', $event)"
        /> -->
        <v-select
          :model-value="advancedParams.scene_mode"
          :items="sceneModeOptions"
          label="Scene Mode"
          :disabled="props.disabled"
          item-title="text"
          item-value="value"
          @update:model-value="updateAdvancedParam('scene_mode', $event)"
        />

        
        <v-select
          v-tooltip="'note: This restarts the camera'"
          :model-value="baseParams.rotate"
          :items="rotateOptions"
          label="Image Rotation"
          :disabled="props.disabled"
          item-title="text"
          item-value="value"
          class="mb-4"
          @update:model-value="updateBaseParameter('rotate', $event)"
        />

        <v-select
          :model-value="baseParams.frameTurbo_pro"
          :items="frameTurboOptions"
          label="Frame Turbo"
          :disabled="props.disabled"
          item-title="text"
          item-value="value"
          class="mb-4"
          @update:model-value="updateBaseParameter('frameTurbo_pro', $event)"
        />

        <v-select
          :model-value="baseParams.antiFog"
          :items="antiFogOptions"
          label="Dehaze"
          :disabled="props.disabled"
          item-title="text"
          item-value="value"
          class="mb-4"
          @update:model-value="updateBaseParameter('antiFog', $event)"
        />

        <!-- Restore Advanced Image Parameters -->
        <div class="ma-2 text-right">
          <v-btn
            variant="tonal"
            :disabled="props.disabled || processingAdvancedRestore"
            @click="doRestoreAdvanced"
          >
            <v-progress-circular
              v-if="processingAdvancedRestore"
              indeterminate
              color="white"
              size="20"
              class="me-2"
            />
            {{ processingAdvancedRestore ? "Processing..." : "Restore defaults" }}
          </v-btn>
        </div>
      </v-expansion-panel-text>
    </v-expansion-panel>

  </v-expansion-panels>
</template>

<script setup lang="ts">

import {
    type BaseParameterSetting,
    type CameraControl,
    type AdvancedParameterSetting,
    BaseManualAGainEnableValue,
    BaseManualDGainEnableValue,
    AdvancedDisplayColorBlackValue,
    BaseAntiFogValue,
    BaseFrameTurboValue,
    BaseAutoExposureStrategyModeValue,
    BaseExposureModeValue,
    BaseExposureTimeValue,
    BaseAutoGainModeValue,
    BaseAutoWhiteBalanceModeValue,
    BaseAutoWhiteBalanceSceneValue,
    BaseMaxExposureValue,
    BaseRotateValue,
    AdvancedDisplayFlipValue,
    AdvancedDisplayInfrDetectModeValue,
    AdvancedDisplayLensCorrectionValue,
    AdvancedDisplayMirrorValue,
    AdvancedDisplayPowerFreqValue,
    AdvancedDisplayIRCUTLevelValue,
    AdvancedDisplayAntiflickerValue,
    AdvancedDisplay2dNrLevelValue,
    AdvancedDisplayAutoIrisValue,
    AdvancedDisplayLampTypeValue,
    AdvancedDisplayLDRLevelValue,
    AdvancedDisplayLedControlAvailValue,
    AdvancedDisplayNoiseReductionValue,
    AdvancedDisplayWDRSensorValue,
    AdvancedDisplayHlcEnableValue,
    AdvancedDisplayLowFramerateValue,
    AdvancedDisplayLedControlValue,  
    AdvancedDisplaySceneModeValue
} from '@/bindings/radcam'


import { enumToOptions } from '@/utils/enumUtils'
import axios from 'axios'
import { onMounted, ref, watch } from 'vue'

const props = defineProps<{
  selectedCameraUuid: string | null
  backendApi: string,
  disabled: boolean
}>()

const processingWhiteBalance = ref(false)
const processingBaseRestore = ref(false)
const processingAdvancedRestore = ref(false)

// Advanced parameters
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
  rotate: null
})
const advancedParams = ref<AdvancedParameterSetting>({
  mirror: null,
  flip: null,
  power_freq: null,
  color_black: null,
  infr_detect_mode: null,
  sens_day_to_night: null,
  sens_night_to_day: null,
  infr_day_h: null,
  infr_day_m: null,
  infr_night_h: null,
  infr_night_m: null,
  lens_correction: null,
  wdr_level: null,
  ircut_level: null,
  ldr_level: null,
  led_control_mode: null,
  lamp_type: null,
  led_control_avail: null,
  ir_level: null,
  led_level: null,
  led_control: null,
  auto_iris: null,
  irisLevel: null,
  noiseReduction: null,
  wdr_sensor: null,
  wdr_level_sensor: null,
  hlc_enable: null,
  low_farme_rate: null,
  _2DNR_level: null,
  anti_flicker: null,
  scene_mode: null,
  onceAWB: null,
  set_default: null
})

// Generate options using enum utilities
const antiFogOptions = enumToOptions(BaseAntiFogValue)
const frameTurboOptions = enumToOptions(BaseFrameTurboValue)
const sceneModeOptions = enumToOptions(AdvancedDisplaySceneModeValue)
const autoExposureStrategyModeOptions = enumToOptions(BaseAutoExposureStrategyModeValue)
const exposureModeOptions = enumToOptions(BaseExposureModeValue)
const exposureTimeOptions = enumToOptions(BaseExposureTimeValue).map(option => ({
  ...option,
  text: `1/${option.value} s`
}));
const maxExposureOptions = enumToOptions(BaseMaxExposureValue).map(option => ({
  ...option,
  text: `1/${option.value} s`
}));
const autoWhiteBalanceModeOptions = enumToOptions(BaseAutoWhiteBalanceModeValue)
const autoWhiteBalanceSceneOptions = enumToOptions(BaseAutoWhiteBalanceSceneValue)
const autoGainModeOptions = enumToOptions(BaseAutoGainModeValue)
const rotateOptions = enumToOptions(BaseRotateValue).map(option => ({
  ...option,
  text: `${option.value} º`
}));
const mirrorOptions = enumToOptions(AdvancedDisplayMirrorValue)
const flipOptions = enumToOptions(AdvancedDisplayFlipValue)
const powerFreqOptions = enumToOptions(AdvancedDisplayPowerFreqValue)
const infrDetectModeOptions = enumToOptions(AdvancedDisplayInfrDetectModeValue)
const lensCorrectionOptions = enumToOptions(AdvancedDisplayLensCorrectionValue)
const ircutLevelOptions = enumToOptions(AdvancedDisplayIRCUTLevelValue)
const ldrLevelOptions = enumToOptions(AdvancedDisplayLDRLevelValue)
const lampTypeOptions = enumToOptions(AdvancedDisplayLampTypeValue)
const ledControlAvailOptions = enumToOptions(AdvancedDisplayLedControlAvailValue)
const ledControlOptions = enumToOptions(AdvancedDisplayLedControlValue)
const autoIrisOptions = enumToOptions(AdvancedDisplayAutoIrisValue)
const noiseReductionOptions = enumToOptions(AdvancedDisplayNoiseReductionValue)
const _2dNrLevelOptions = enumToOptions(AdvancedDisplay2dNrLevelValue)
const antiFlickerOptions = enumToOptions(AdvancedDisplayAntiflickerValue)

onMounted(() => {
  getBaseParameters()
  getAdvancedParameters()
})

watch(
  () => props.selectedCameraUuid,
  async (newValue) => {
    if (newValue) {
      getBaseParameters()
      getAdvancedParameters()
    }
  }
)

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const updateBaseParameter = (param: keyof BaseParameterSetting, value: any) => {
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

  console.log(payload)

  axios.post(`${props.backendApi}/control`, payload)
    .then(response => {
      baseParams.value = response.data as BaseParameterSetting
    })
    .catch(error => {
      console.error(`Error sending ${String(param)} control with value '${value}':`, error.message)
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

  axios.post(`${props.backendApi}/control`, payload)
    .then(response => {
      baseParams.value = response.data as BaseParameterSetting
      console.log(response.data)
    })
    .catch(error => {
      console.error(`Error sending getImageAdjustment request:`, error.message)
    })
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

  axios.post(`${props.backendApi}/control`, payload)
    .catch(error => {
      console.error("Error sending onceAWB control:", error.message)
    }).finally(() => {
      processingWhiteBalance.value = false
      getBaseParameters()
    })
}

const doRestoreBase = async () => {
  if (!props.selectedCameraUuid) {
    return
  }

  processingBaseRestore.value = true

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
      baseParams.value = response.data as BaseParameterSetting
    })
    .catch(error => {
      console.error("Error sending base image restore control:", error.message)
    })
    .finally(() => {
      processingBaseRestore.value = false
    })
}

const getAdvancedParameters = () => {
  if (!props.selectedCameraUuid) return

  const payload = {
    camera_uuid: props.selectedCameraUuid,
    action: "getImageAdjustmentEx",
  }

  axios.post(`${props.backendApi}/control`, payload)
    .then(response => {
      advancedParams.value = response.data as AdvancedParameterSetting
    })
    .catch(error => {
      console.error("Error fetching advanced parameters:", error.message)
    })
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const updateAdvancedParam = (param: keyof AdvancedParameterSetting, value: any) => {
  if (!props.selectedCameraUuid) return

  const payload: CameraControl = {
    camera_uuid: props.selectedCameraUuid,
    action: "setImageAdjustmentEx",
    json: { [param]: value } as AdvancedParameterSetting
  }

  axios.post(`${props.backendApi}/control`, payload)
    .then(response => {
      advancedParams.value = { ...advancedParams.value, ...response.data }
    })
    .catch(error => {
      console.error(`Error updating ${param}:`, error.message)
    })
}

const doRestoreAdvanced = async () => {
  if (!props.selectedCameraUuid) return

  processingAdvancedRestore.value = true

  const payload: CameraControl = {
    camera_uuid: props.selectedCameraUuid,
    action: "setImageAdjustmentEx",
    json: { set_default: 1 } as AdvancedParameterSetting
  }

  axios.post(`${props.backendApi}/control`, payload)
    .then(response => {
      advancedParams.value = response.data as AdvancedParameterSetting
    })
    .catch(error => {
      console.error("Error restoring advanced parameters:", error.message)
    })
    .finally(() => {
      processingAdvancedRestore.value = false
    })
}

</script>
