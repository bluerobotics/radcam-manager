import { onUnmounted, type Ref } from 'vue'

import { backendClient } from './backendClient'

/**
 * Bind a handler to `camera/state` for the selected camera.
 *
 * backendClient fans out to multiple handlers; HomeView owns UI while tabs
 * sync params. Subscribe/unsubscribe wire is owned by HomeView (refcounts).
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
