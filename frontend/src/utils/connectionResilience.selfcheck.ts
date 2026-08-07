import { isWsConnectionStale, STALE_CONNECTION_MS } from './backendClient'
import { autopilotDependentControlsBlocked } from './useSystemHealth'

export function runConnectionResilienceSelfCheck(): void {
  const now = 100_000
  if (isWsConnectionStale(now - (STALE_CONNECTION_MS - 1), now)) {
    throw new Error('connectionResilience: just-under stale window must stay live')
  }
  if (!isWsConnectionStale(now - STALE_CONNECTION_MS, now)) {
    throw new Error('connectionResilience: exactly at stale window must be stale')
  }
  if (!isWsConnectionStale(now - STALE_CONNECTION_MS - 1, now)) {
    throw new Error('connectionResilience: past stale window must be stale')
  }
  // Stats land every 5s; the window must be longer than one miss and shorter than a
  // minute so a SIGSTOP'd backend is noticed before the operator assumes it is fine.
  if (STALE_CONNECTION_MS < 15_000 || STALE_CONNECTION_MS > 30_000) {
    throw new Error(`connectionResilience: unexpected STALE_CONNECTION_MS=${STALE_CONNECTION_MS}`)
  }

  if (autopilotDependentControlsBlocked('online')) {
    throw new Error('connectionResilience: online must not block autopilot controls')
  }
  if (autopilotDependentControlsBlocked('syncing')) {
    throw new Error('connectionResilience: syncing must not block autopilot controls')
  }
  for (const state of ['unknown', 'mavlink_down', 'autopilot_offline', 'unresponsive'] as const) {
    if (!autopilotDependentControlsBlocked(state)) {
      throw new Error(`connectionResilience: ${state} must block autopilot controls`)
    }
  }
  if (!autopilotDependentControlsBlocked(null) || !autopilotDependentControlsBlocked(undefined)) {
    throw new Error(
      'connectionResilience: absent health must block autopilot controls (reconnect gap)',
    )
  }

  console.log('connectionResilience self-check ok')
}
