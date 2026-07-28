import { onMounted, onUnmounted, watch, type Ref } from 'vue'

import { backendClient } from './backendClient'

export function useCameraState(
  selectedCameraUuid: Ref<string | null>,
  handler: (body: unknown) => void,
) {
  const unsubscribe = backendClient.onEvent('camera/state', handler)

  const subscribe = () => {
    if (selectedCameraUuid.value) {
      backendClient.subscribeCamera(selectedCameraUuid.value)
    }
  }

  onMounted(subscribe)

  onUnmounted(() => {
    unsubscribe()
    if (selectedCameraUuid.value) {
      backendClient.unsubscribeCamera(selectedCameraUuid.value)
    }
  })

  watch(selectedCameraUuid, (uuid, previousUuid) => {
    if (previousUuid) {
      backendClient.unsubscribeCamera(previousUuid)
    }
    if (uuid) {
      subscribe()
    }
  })
}
