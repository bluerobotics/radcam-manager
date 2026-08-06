import { computed, onUnmounted, shallowRef, type ComputedRef, type ShallowRef } from 'vue'

import type { AutopilotHealth, Diagnostics, ExpectedCamera, SystemHealth } from '@/bindings/radcam_api'
import { backendClient } from './backendClient'

const systemHealth = shallowRef<SystemHealth | null>(null)
let refCount = 0
let subscribed = false
let unsubscribeHealth: (() => void) | null = null
let unsubscribeConnection: (() => void) | null = null

function ensureSubscribed(): void {
  if (subscribed) return
  subscribed = true

  unsubscribeHealth = backendClient.onEvent('system/health', (body) => {
    systemHealth.value = body as SystemHealth
  })
  unsubscribeConnection = backendClient.onConnectionState((state) => {
    if (state === 'disconnected') {
      systemHealth.value = null
    }
  })
}

function releaseSubscription(): void {
  if (!subscribed) return
  subscribed = false
  unsubscribeHealth?.()
  unsubscribeHealth = null
  unsubscribeConnection?.()
  unsubscribeConnection = null
}

export type UseSystemHealth = {
  systemHealth: ShallowRef<SystemHealth | null>
  autopilotState: ComputedRef<AutopilotHealth>
  autopilotOnline: ComputedRef<boolean>
  discoveryEmpty: ComputedRef<boolean>
  expectedMissing: ComputedRef<ExpectedCamera[]>
  diagnostics: ComputedRef<Diagnostics | null>
}

export function useSystemHealth(): UseSystemHealth {
  ensureSubscribed()
  refCount += 1

  onUnmounted(() => {
    refCount -= 1
    if (refCount === 0) {
      releaseSubscription()
    }
  })

  const autopilotState = computed(
    (): AutopilotHealth => systemHealth.value?.autopilot ?? 'unknown',
  )
  const autopilotOnline = computed(() => systemHealth.value?.autopilot === 'online')
  const discoveryEmpty = computed(() => {
    const health = systemHealth.value
    if (!health) return false
    return (
      health.mcm === 'online'
      && health.cameras_discovered === 0
      && health.expected_missing.length === 0
    )
  })
  const expectedMissing = computed(() => systemHealth.value?.expected_missing ?? [])
  const diagnostics = computed(() => systemHealth.value?.diagnostics ?? null)

  return {
    systemHealth,
    autopilotState,
    autopilotOnline,
    discoveryEmpty,
    expectedMissing,
    diagnostics,
  }
}
