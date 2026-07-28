import { onUnmounted, type Ref } from 'vue'

import { backendClient } from './backendClient'

/**
 * Bind a handler to `camera/state` for the selected camera.
 *
 * Wire subscribe/unsubscribe is owned by HomeView so tabs do not fight over
 * refcounts. `selectedCameraUuid` is part of the API for callers that close over it.
 */
export function useCameraState(
  _selectedCameraUuid: Ref<string | null>,
  handler: (body: unknown) => void,
) {
  const unsubscribe = backendClient.onEvent('camera/state', handler)

  onUnmounted(() => {
    unsubscribe()
  })
}
