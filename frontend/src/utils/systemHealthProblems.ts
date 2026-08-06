import type {
  AutopilotHealth,
  CameraConnectivity,
  SystemHealth,
} from '@/bindings/radcam_api'

export type HealthProblemKind =
  | 'mcm'
  | 'autopilot'
  | 'camera'
  | 'camera_stream'
  | 'camera_onvif_auth'
  | 'lua'
  | 'lua_script'

export type HealthProblem = {
  kind: HealthProblemKind
  severity: 'error' | 'warning' | 'info'
  title: string
  body: string
  detail?: string | null
  /** Live retry / recovery line under the problem. */
  progress?: string | null
  /** Operator-facing "since HH:MM (N min)" — set by enrichHealthProblems. */
  since?: string | null
  showForget?: boolean
  showReboot?: boolean
  showGoToSetup?: boolean
  showUpdateLuaScript?: boolean
}

export type HealthProblemsInput = {
  systemHealth: SystemHealth | null
  cameraUuid: string | null
  cameraLabel: string
  cameraConnectivity: CameraConnectivity
  /** True when the selected camera is configured but absent from MCM discovery. */
  cameraExpectedMissing?: boolean
  /** MCM video stream failure detail when the stream stayed broken. */
  cameraStreamError?: string | null
  /** MCM ONVIF authentication failure detail when login with expected credentials fails. */
  cameraOnvifAuthError?: string | null
  /** Peak MCM consecutive failures observed this episode (for duration text). */
  mcmAttemptsPeak: number
}

const AUTOPILOT_PROBLEM_STATES: ReadonlySet<AutopilotHealth> = new Set([
  'endpoint_setup_failed',
  'mavlink_down',
  'autopilot_offline',
  'unresponsive',
])

const SEVERITY_RANK: Record<HealthProblem['severity'], number> = {
  error: 0,
  warning: 1,
  info: 2,
}

/** MCM re-probes every second while down. */
const MCM_PROBE_INTERVAL_MS = 1000

export function collectHealthProblems(input: HealthProblemsInput): HealthProblem[] {
  const problems: HealthProblem[] = []
  const health = input.systemHealth
  const mcmDown = health?.mcm === 'down'

  if (mcmDown) {
    problems.push({
      kind: 'mcm',
      severity: 'error',
      title: 'BlueOS video service unavailable',
      body: 'RadCam Manager cannot reach the BlueOS video service, so cameras cannot be discovered or controlled.',
      detail: health.mcm_detail ?? null,
      progress: mcmProgressLine(health, input.mcmAttemptsPeak),
    })
  }

  if (health) {
    const autopilot = autopilotProblem(health)
    if (autopilot) problems.push(autopilot)

    if (health.autopilot === 'online' && health.lua_scripting_disabled) {
      problems.push({
        kind: 'lua',
        severity: 'warning',
        title: 'Focus and zoom won\'t follow the autopilot',
        body: 'Lua scripting is disabled on the autopilot (SCR_ENABLE). Open hardware setup to re-apply it, then reboot the autopilot.',
        showGoToSetup: true,
      })
    }

    const script = luaScriptProblem(health)
    if (script) problems.push(script)
  }

  if (input.cameraUuid) {
    if (mcmDown) {
      problems.push({
        kind: 'camera',
        severity: 'info',
        title: 'Camera status unknown',
        body: `Cannot check ${input.cameraLabel} while the BlueOS video service is unavailable.`,
      })
    } else {
      const camera = cameraProblem(
        input.cameraLabel,
        input.cameraConnectivity,
        input.cameraExpectedMissing === true,
      )
      if (camera) problems.push(camera)

      if (input.cameraStreamError && input.cameraConnectivity === 'online') {
        problems.push({
          kind: 'camera_stream',
          severity: 'warning',
          title: 'Camera video stream not running',
          body: `${input.cameraLabel} is responding, but its video stream is not running. RadCam Manager is restarting it automatically.`,
          detail: input.cameraStreamError,
          progress: 'Restarting the video stream…',
        })
      }

      if (input.cameraOnvifAuthError && input.cameraConnectivity === 'online') {
        problems.push({
          kind: 'camera_onvif_auth',
          severity: 'error',
          title: 'Camera ONVIF password does not match',
          body: `${input.cameraLabel} is responding to RadCam Manager, but the BlueOS video service cannot log in over ONVIF with the expected factory credentials (admin / blue), so there is no video. Restore the camera's ONVIF password to admin / blue.`,
          detail: input.cameraOnvifAuthError,
          progress: 'Waiting for ONVIF login to succeed…',
        })
      }
    }
  }

  return problems
}

export function sortProblemsBySeverity(problems: HealthProblem[]): HealthProblem[] {
  return [...problems].sort(
    (left, right) => SEVERITY_RANK[left.severity] - SEVERITY_RANK[right.severity],
  )
}

export function problemsSummary(problems: HealthProblem[]): string | null {
  const notable = problems.filter((problem) => problem.severity !== 'info')
  if (notable.length <= 1) return null
  const errors = notable.filter((problem) => problem.severity === 'error').length
  const warnings = notable.filter((problem) => problem.severity === 'warning').length
  const parts: string[] = []
  if (errors > 0) parts.push(`${errors} critical`)
  if (warnings > 0) parts.push(`${warnings} warning${warnings === 1 ? '' : 's'}`)
  return `${notable.length} issues need attention (${parts.join(', ')})`
}

function formatDurationShort(ms: number): string {
  const secs = Math.max(1, Math.round(ms / 1000))
  if (secs < 60) return `${secs} sec`
  const mins = Math.floor(secs / 60)
  const rem = secs % 60
  if (rem === 0) return `${mins} min`
  return `${mins} min ${rem} sec`
}

export function formatProblemSince(firstSeenMs: number, nowMs: number): string {
  const date = new Date(firstSeenMs)
  const time = date.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })
  const elapsedMin = Math.max(1, Math.round((nowMs - firstSeenMs) / 60_000))
  return `since ${time} (${elapsedMin} min)`
}

export function recoveryTitle(
  resolvedKinds: readonly HealthProblemKind[],
  forgetSuccess: boolean,
): string {
  if (forgetSuccess) return 'Camera removed from setup'
  const kinds = resolvedKinds.filter((kind) => kind !== 'lua')
  if (kinds.length === 0) {
    return resolvedKinds.includes('lua') ? 'Lua scripting enabled' : 'All clear'
  }
  if (kinds.length === 1) {
    switch (kinds[0]) {
      case 'mcm':
        return 'Video service restored'
      case 'autopilot':
        return 'Autopilot connection restored'
      case 'camera':
        return 'Camera connection restored'
      case 'camera_stream':
        return 'Camera video stream restored'
      case 'camera_onvif_auth':
        return 'Camera ONVIF login restored'
      case 'lua_script':
        return 'Autopilot script updated'
      default:
        break
    }
  }
  return 'Issues resolved'
}

export function recoveryMessage(
  resolvedKinds: readonly HealthProblemKind[],
  mcmAttemptsPeak: number,
  forgetSuccess: boolean,
  withCloseHint = true,
): string {
  if (forgetSuccess) {
    return withCloseHint
      ? 'This camera was removed from your saved setup. Click Close to continue.'
      : 'This camera was removed from your saved setup.'
  }

  const parts: string[] = []
  if (resolvedKinds.includes('mcm')) {
    if (mcmAttemptsPeak > 0) {
      const duration = formatDurationShort(mcmAttemptsPeak * MCM_PROBE_INTERVAL_MS)
      parts.push(`BlueOS video service is back (after about ${duration})`)
    } else {
      parts.push('BlueOS video service is back')
    }
  }
  if (resolvedKinds.includes('autopilot')) {
    parts.push('Autopilot connection is working again')
  }
  if (resolvedKinds.includes('camera')) {
    parts.push('Camera connection is working again')
  }
  if (resolvedKinds.includes('camera_stream')) {
    parts.push('Camera video stream is running again')
  }
  if (resolvedKinds.includes('camera_onvif_auth')) {
    parts.push('ONVIF login succeeded and video is available again')
  }
  if (resolvedKinds.includes('lua')) {
    parts.push('Focus and zoom correlation is ready')
  }
  if (resolvedKinds.includes('lua_script')) {
    parts.push('Autopilot script is up to date')
  }

  if (parts.length === 0) {
    return withCloseHint ? 'All clear. Click Close to continue.' : 'All clear.'
  }
  const body = parts.join('. ')
  return withCloseHint ? `${body}. Click Close to continue.` : `${body}.`
}

function mcmProgressLine(health: SystemHealth, mcmAttemptsPeak: number): string {
  const failures = Math.max(health.diagnostics.mcm_consecutive_failures, mcmAttemptsPeak, 1)
  const elapsed = formatDurationShort(failures * MCM_PROBE_INTERVAL_MS)
  const cadence = MCM_PROBE_INTERVAL_MS / 1000
  return `Retrying every ${cadence} sec (about ${elapsed} so far)…`
}

function autopilotProblem(health: SystemHealth): HealthProblem | null {
  if (health.autopilot === 'unknown') {
    return {
      kind: 'autopilot',
      severity: 'info',
      title: 'Checking autopilot connection',
      body: 'RadCam Manager is still determining whether the MAVLink link and flight controller are available. This is normal while BlueOS is starting.',
      detail: health.autopilot_detail ?? null,
      progress: 'Waiting for autopilot status from BlueOS…',
    }
  }

  if (!AUTOPILOT_PROBLEM_STATES.has(health.autopilot)) return null

  switch (health.autopilot) {
    case 'endpoint_setup_failed':
      return {
        kind: 'autopilot',
        severity: 'error',
        title: 'Autopilot service unavailable',
        body: 'RadCam Manager cannot configure its MAVLink connection through BlueOS. Focus, zoom and tilt are unavailable. Check that BlueOS is fully started; if this persists, restart BlueOS.',
        detail: health.autopilot_detail ?? null,
        progress: 'Waiting for BlueOS to finish starting…',
      }
    case 'mavlink_down': {
      const neverReceived = health.diagnostics.last_frame_age_ms == null
      return {
        kind: 'autopilot',
        severity: 'error',
        title: 'MAVLink connection unavailable',
        body: neverReceived
          ? 'RadCam Manager has not received any MAVLink data from BlueOS yet. Focus, zoom and tilt are unavailable. Check that BlueOS is fully started, then verify MAVLink endpoints are configured.'
          : 'No MAVLink data is reaching RadCam Manager. Focus, zoom and tilt are unavailable. Check BlueOS MAVLink endpoints, then restart BlueOS if needed.',
        detail: health.autopilot_detail ?? null,
        progress: neverReceived
          ? 'Waiting for the first MAVLink data from BlueOS…'
          : 'Waiting for MAVLink data from BlueOS…',
      }
    }
    case 'autopilot_offline':
      return {
        kind: 'autopilot',
        severity: 'error',
        title: 'Autopilot unavailable',
        body: 'MAVLink is working but the autopilot is not sending heartbeats. Check that the flight controller is powered and connected, then reboot it if needed.',
        detail: health.autopilot_detail ?? null,
        progress: 'Waiting for the flight controller to power on…',
      }
    case 'unresponsive':
      return {
        kind: 'autopilot',
        severity: 'error',
        title: 'Autopilot not responding',
        body: 'The autopilot is heartbeating but not answering our requests. Focus, zoom and tilt may not move. Reboot the autopilot; if it keeps happening, contact support.',
        detail: health.autopilot_detail ?? null,
        progress: 'Retrying communication with the autopilot…',
      }
    default:
      return null
  }
}

function luaScriptProblem(health: SystemHealth): HealthProblem | null {
  // Applying the script reloads scripting over MAVLink, so there is nothing the operator
  // can do about it until the autopilot is back.
  if (health.autopilot !== 'online') return null

  switch (health.lua_script) {
    case 'missing':
      return {
        kind: 'lua_script',
        severity: 'warning',
        title: 'Autopilot script is not installed',
        body: 'The script that drives focus and zoom from the autopilot is not on the flight controller, so focus and zoom will not follow the camera. Install it now — the autopilot may reboot.',
        showUpdateLuaScript: true,
      }
    case 'outdated':
      return {
        kind: 'lua_script',
        severity: 'warning',
        title: 'Autopilot script is out of date',
        body: 'The script installed on the flight controller is not the one this version of RadCam Manager expects, so focus and zoom may misbehave. Update it — the autopilot may reboot.',
        showUpdateLuaScript: true,
      }
    case 'failing':
      return {
        kind: 'lua_script',
        severity: 'warning',
        title: 'Autopilot script is failing',
        body: 'The right script is installed, but the autopilot reports it is erroring, so focus and zoom will not follow the camera. Re-installing it often clears this — the autopilot may reboot.',
        detail: health.lua_script_detail ?? null,
        showUpdateLuaScript: true,
      }
    default:
      return null
  }
}

function cameraProblem(
  label: string,
  connectivity: CameraConnectivity,
  expectedMissing: boolean,
): HealthProblem | null {
  switch (connectivity) {
    case 'missing':
      return {
        kind: 'camera',
        severity: 'info',
        title: 'Waiting for camera discovery',
        body: `${label} is configured and the BlueOS video service is still looking for it. This is normal after the video service starts — discovery can take a short while.`,
        showForget: expectedMissing,
        progress: 'Waiting for ONVIF discovery…',
      }
    case 'unreachable':
      return {
        kind: 'camera',
        severity: 'error',
        title: 'Camera unavailable',
        body: `RadCam Manager can no longer reach ${label}. Check the Ethernet cable between the camera and the vehicle, and confirm the camera has power.`,
        showForget: expectedMissing,
        progress: 'Waiting for the camera to come back online…',
      }
    case 'unresponsive':
      return {
        kind: 'camera',
        severity: 'error',
        title: 'Camera not responding',
        body: `${label} is on the network but is not answering. Power-cycle the camera. If it keeps happening, contact support.`,
        showReboot: true,
        progress: 'Waiting for the camera to respond…',
      }
    default:
      return null
  }
}
