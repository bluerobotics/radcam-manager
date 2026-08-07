import {
  closeHealthDialog,
  degradedBannerCopy,
  evaluateHealthFlags,
  healthDialogStateOnDisconnect,
  healthDialogView,
  initialHealthDialogState,
  minimizeHealthDialog,
  noteActiveProblems,
  noteForgetSuccess,
  recoveryWhileMinimizedToast,
  reduceHealthDialogOnProblems,
  reopenHealthDialog,
} from './healthDialogState'
import {
  allNotableProblemsSelfRecover,
  collectHealthProblems,
  formatProblemSince,
  recoveryMessage,
  recoveryTitle,
  SELF_RECOVERING_KINDS,
  type HealthProblemsInput,
} from './systemHealthProblems'

export function runHealthDialogSelfCheck(): void {
  let state = initialHealthDialogState()
  const baseInput = (overrides: Partial<HealthProblemsInput> = {}): HealthProblemsInput => ({
    systemHealth: {
      mcm: 'down',
      mcm_detail: 'connection refused',
      cameras_discovered: 0,
      expected_missing: [],
      autopilot: 'online',
      lua_scripting_disabled: false,
      lua_script: 'ok',
      diagnostics: {
        mavlink_reconnects: 0,
        mavlink_frames_lagged: 0,
        state_events_lagged: 0,
        mcm_consecutive_failures: 3,
        script_reloads: 0,
        backend_version: 'test',
      },
    },
    cameraUuid: null,
    cameraLabel: '',
    cameraConnectivity: 'unknown',
    ...overrides,
  })

  const down = evaluate(baseInput())
  const mcmDownProblems = evaluateHealthFlags(baseInput()).problems
  if (!down.active || !down.degraded) {
    throw new Error('healthDialog: MCM down must be active+degraded')
  }
  if (collectHealthProblems(baseInput()).length !== 1) {
    throw new Error('healthDialog: expected one MCM problem')
  }

  state = reduceHealthDialogOnProblems(initialHealthDialogState(), down.degraded)
  if (state.mode !== 'open') throw new Error('healthDialog: must auto-open on degraded problems')

  state = noteActiveProblems(state, mcmDownProblems, 5000)
  state = noteActiveProblems(state, mcmDownProblems, 10_000)

  // Idempotent while already open — must keep the same reference.
  const openAgain = reduceHealthDialogOnProblems(state, down.degraded)
  if (openAgain !== state) {
    throw new Error('healthDialog: reduce while open must be idempotent')
  }

  state = minimizeHealthDialog(state)
  let view = healthDialogView(state, true)
  if (view.showDialog || !view.showDegradedBanner) {
    throw new Error('healthDialog: minimize must show banner only')
  }

  state = reopenHealthDialog(state)
  view = healthDialogView(state, true)
  if (!view.showDialog || view.showDegradedBanner) {
    throw new Error('healthDialog: reopen must show dialog')
  }

  // Recovery while open → sticky until Close.
  state = reduceHealthDialogOnProblems(state, false)
  view = healthDialogView(state, false)
  if (!view.showDialog || !view.awaitingClose) {
    throw new Error('healthDialog: recovery must keep dialog open awaiting close')
  }
  if (recoveryTitle(['mcm'], false) !== 'Video service restored') {
    throw new Error(`healthDialog: bad recovery title: ${recoveryTitle(['mcm'], false)}`)
  }
  if (
    recoveryMessage(['mcm'], 5000, false)
    !== 'BlueOS video service is back (after about 5 sec). Click Close to continue.'
  ) {
    throw new Error(`healthDialog: bad recovery line: ${recoveryMessage(['mcm'], 5000, false)}`)
  }
  if (recoveryMessage([], 0, false) !== 'All clear. Click Close to continue.') {
    throw new Error(`healthDialog: bad empty recovery line: ${recoveryMessage([], 0, false)}`)
  }
  if (
    recoveryWhileMinimizedToast({
      ...initialHealthDialogState(),
      episodeKinds: ['mcm'],
      mcmOutageMsPeak: 5000,
    })
    !== 'BlueOS video service is back (after about 5 sec).'
  ) {
    throw new Error('healthDialog: minimized recovery toast must omit Close hint')
  }

  state = closeHealthDialog()
  if (state.mode !== 'hidden' || state.mcmOutageMsPeak !== 0) {
    throw new Error('healthDialog: close must reset')
  }

  // Recovery while minimized → clear banner.
  state = reduceHealthDialogOnProblems(initialHealthDialogState(), true)
  state = minimizeHealthDialog(state)
  state = reduceHealthDialogOnProblems(state, false)
  view = healthDialogView(state, false)
  if (view.showDialog || view.showDegradedBanner || state.mode !== 'hidden') {
    throw new Error('healthDialog: recovery while minimized must clear')
  }

  state = reduceHealthDialogOnProblems(initialHealthDialogState(), true)
  state = minimizeHealthDialog(state)
  view = healthDialogView(state, true)
  if (!view.showDegradedBanner) {
    throw new Error('healthDialog: minimized+degraded must show banner')
  }

  // Camera + MCM together — camera cable blame suppressed while MCM is down.
  const multi = collectHealthProblems(
    baseInput({
      cameraUuid: 'cam',
      cameraLabel: 'RadCam',
      cameraConnectivity: 'unreachable',
    }),
  )
  if (multi.length !== 2) {
    throw new Error(`healthDialog: expected MCM+camera-unknown problems, got ${multi.length}`)
  }
  if (multi[1]?.title !== 'Camera status unknown') {
    throw new Error(`healthDialog: expected camera status unknown, got ${multi[1]?.title}`)
  }

  // Syncing is not a modal problem.
  const syncingInput = baseInput({
    systemHealth: {
      mcm: 'online',
      cameras_discovered: 1,
      expected_missing: [],
      autopilot: 'syncing',
      lua_scripting_disabled: false,
      lua_script: 'ok',
      diagnostics: {
        mavlink_reconnects: 0,
        mavlink_frames_lagged: 0,
        state_events_lagged: 0,
        mcm_consecutive_failures: 0,
        script_reloads: 0,
        backend_version: 'test',
      },
    },
  })
  const syncing = evaluate(syncingInput)
  if (syncing.active || syncing.degraded) {
    throw new Error('healthDialog: syncing must not be active or degraded')
  }
  state = reduceHealthDialogOnProblems(initialHealthDialogState(), syncing.degraded)
  if (state.mode !== 'hidden') {
    throw new Error('healthDialog: syncing must not open sticky modal')
  }

  const mavlinkNever = baseInput({
    systemHealth: {
      mcm: 'online',
      cameras_discovered: 0,
      expected_missing: [],
      autopilot: 'mavlink_down',
      autopilot_detail: 'MAVLink component unavailable',
      lua_scripting_disabled: false,
      lua_script: 'ok',
      diagnostics: {
        mavlink_reconnects: 0,
        mavlink_frames_lagged: 0,
        state_events_lagged: 0,
        mcm_consecutive_failures: 0,
        script_reloads: 0,
        backend_version: 'test',
      },
    },
  })
  const mavlinkFlags = evaluate(mavlinkNever)
  if (!mavlinkFlags.degraded) {
    throw new Error('healthDialog: mavlink_down without frame age must be degraded')
  }
  const mavlinkProblems = collectHealthProblems(mavlinkNever)
  if (mavlinkProblems.length === 0 || mavlinkProblems[0]?.kind !== 'autopilot') {
    throw new Error('healthDialog: mavlink_down without frame age must yield autopilot problem')
  }

  const unknownInput = baseInput({
    systemHealth: {
      mcm: 'online',
      cameras_discovered: 0,
      expected_missing: [],
      autopilot: 'unknown',
      lua_scripting_disabled: false,
      lua_script: 'ok',
      diagnostics: {
        mavlink_reconnects: 0,
        mavlink_frames_lagged: 0,
        state_events_lagged: 0,
        mcm_consecutive_failures: 0,
        script_reloads: 0,
        backend_version: 'test',
      },
    },
  })
  const unknownProblems = collectHealthProblems(unknownInput)
  if (unknownProblems.length === 0 || unknownProblems[0]?.kind !== 'autopilot') {
    throw new Error('healthDialog: unknown autopilot must yield operator copy')
  }
  if (evaluate(unknownInput).degraded) {
    throw new Error('healthDialog: unknown during boot must not be degraded')
  }

  // Healthy boot: unknown → syncing → online must stay hidden.
  state = initialHealthDialogState()
  for (const autopilot of ['unknown', 'syncing', 'online'] as const) {
    const input = baseInput({
      systemHealth: {
        mcm: 'online',
        cameras_discovered: autopilot === 'online' ? 1 : 0,
        expected_missing: [],
        autopilot,
        lua_scripting_disabled: false,
        lua_script: 'ok',
        diagnostics: {
          mavlink_reconnects: 0,
          mavlink_frames_lagged: 0,
          state_events_lagged: 0,
          mcm_consecutive_failures: 0,
        script_reloads: 0,
          backend_version: 'test',
        },
      },
    })
    const flags = evaluate(input)
    if (flags.degraded) {
      throw new Error(`healthDialog: boot step autopilot=${autopilot} must not be degraded`)
    }
    state = reduceHealthDialogOnProblems(state, flags.degraded)
    if (state.mode !== 'hidden' || state.awaitingClose) {
      throw new Error(`healthDialog: boot step autopilot=${autopilot} must not open modal`)
    }
  }

  // A stale or absent autopilot script must offer the one-click fix when a camera is
  // selected, and only while the autopilot can actually take it.
  for (const lua_script of ['missing', 'outdated', 'failing'] as const) {
    const scriptInput = baseInput({
      cameraUuid: 'cam',
      cameraLabel: 'RadCam',
      systemHealth: {
        mcm: 'online',
        cameras_discovered: 1,
        expected_missing: [],
        autopilot: 'online',
        lua_scripting_disabled: false,
        lua_script,
        diagnostics: {
          mavlink_reconnects: 0,
          mavlink_frames_lagged: 0,
          state_events_lagged: 0,
          mcm_consecutive_failures: 0,
          script_reloads: 0,
          backend_version: 'test',
        },
      },
    })
    const scriptProblem = collectHealthProblems(scriptInput).find(
      (problem) => problem.kind === 'lua_script',
    )
    if (!scriptProblem?.showUpdateLuaScript) {
      throw new Error(`healthDialog: lua_script=${lua_script} must offer the update button`)
    }

    const noCameraProblem = collectHealthProblems({
      ...scriptInput,
      cameraUuid: null,
      cameraLabel: '',
    }).find((problem) => problem.kind === 'lua_script')
    if (noCameraProblem?.showUpdateLuaScript) {
      throw new Error(`healthDialog: lua_script=${lua_script} must hide update without camera`)
    }
    if (!noCameraProblem?.body.includes('discovered')) {
      throw new Error(`healthDialog: lua_script=${lua_script} without camera must mention discovery`)
    }

    const offlineProblems = collectHealthProblems(
      baseInput({
        systemHealth: { ...scriptInput.systemHealth!, autopilot: 'mavlink_down' },
      }),
    )
    if (offlineProblems.some((problem) => problem.kind === 'lua_script')) {
      throw new Error('healthDialog: lua_script problem must stay hidden while autopilot is down')
    }
  }

  const luaScriptUnknownProblems = collectHealthProblems(
    baseInput({
      systemHealth: {
        mcm: 'online',
        cameras_discovered: 1,
        expected_missing: [],
        autopilot: 'online',
        lua_scripting_disabled: false,
        lua_script: 'unknown',
        diagnostics: {
          mavlink_reconnects: 0,
          mavlink_frames_lagged: 0,
          state_events_lagged: 0,
          mcm_consecutive_failures: 0,
        script_reloads: 0,
          backend_version: 'test',
        },
      },
    }),
  )
  if (luaScriptUnknownProblems.some((problem) => problem.kind === 'lua_script')) {
    throw new Error('healthDialog: lua_script unknown must not yield a problem')
  }

  if (recoveryTitle(['lua_script'], false) !== 'Autopilot script updated') {
    throw new Error(`healthDialog: bad lua_script recovery title: ${recoveryTitle(['lua_script'], false)}`)
  }
  if (
    recoveryMessage(['lua_script'], 0, false)
    !== 'Autopilot script is up to date. Click Close to continue.'
  ) {
    throw new Error(`healthDialog: bad lua_script recovery line: ${recoveryMessage(['lua_script'], 0, false)}`)
  }

  if (recoveryTitle(['lua_scripting_disabled'], false) !== 'Lua scripting enabled') {
    throw new Error(
      `healthDialog: bad lua_scripting_disabled recovery title: ${recoveryTitle(['lua_scripting_disabled'], false)}`,
    )
  }

  if (SELF_RECOVERING_KINDS.includes('lua_script' as never)) {
    throw new Error('healthDialog: lua_script must not be self-recovering')
  }
  const mcmOnly = collectHealthProblems(baseInput())
  if (!allNotableProblemsSelfRecover(mcmOnly)) {
    throw new Error('healthDialog: lone MCM outage must be self-recovering')
  }
  const luaScriptOnly = collectHealthProblems(
    baseInput({
      cameraUuid: 'cam',
      systemHealth: {
        mcm: 'online',
        cameras_discovered: 1,
        expected_missing: [],
        autopilot: 'online',
        lua_scripting_disabled: false,
        lua_script: 'missing',
        diagnostics: {
          mavlink_reconnects: 0,
          mavlink_frames_lagged: 0,
          state_events_lagged: 0,
          mcm_consecutive_failures: 0,
          script_reloads: 0,
          backend_version: 'test',
        },
      },
    }),
  )
  if (allNotableProblemsSelfRecover(luaScriptOnly)) {
    throw new Error('healthDialog: lua_script must not count as self-recovering')
  }
  const mcmAndLuaScript = collectHealthProblems(
    baseInput({
      cameraUuid: 'cam',
      systemHealth: {
        mcm: 'down',
        mcm_detail: 'down',
        cameras_discovered: 0,
        expected_missing: [],
        autopilot: 'online',
        lua_scripting_disabled: false,
        lua_script: 'missing',
        diagnostics: {
          mavlink_reconnects: 0,
          mavlink_frames_lagged: 0,
          state_events_lagged: 0,
          mcm_consecutive_failures: 3,
          script_reloads: 0,
          backend_version: 'test',
        },
      },
    }),
  )
  if (allNotableProblemsSelfRecover(mcmAndLuaScript)) {
    throw new Error('healthDialog: mixed self-recovering and lua_script must not all self-recover')
  }
  const twoSelfRecovering = collectHealthProblems(
    baseInput({
      cameraUuid: 'cam',
      cameraLabel: 'RadCam',
      cameraConnectivity: 'unreachable',
    }),
  )
  if (!allNotableProblemsSelfRecover(twoSelfRecovering)) {
    throw new Error('healthDialog: MCM+camera unreachable must all be self-recovering')
  }

  const banner = degradedBannerCopy([
    { kind: 'mcm', severity: 'error', title: 'MCM down', body: 'video service down' },
    { kind: 'autopilot', severity: 'warning', title: 'AP warn', body: 'autopilot warn' },
  ])
  if (!banner.title.includes('2 issues')) {
    throw new Error(`healthDialog: degraded banner must summarize multiple issues: ${banner.title}`)
  }
  if (!banner.body.includes('MCM down') || !banner.body.includes('AP warn')) {
    throw new Error(`healthDialog: degraded banner must list titles: ${banner.body}`)
  }

  state = reduceHealthDialogOnProblems(initialHealthDialogState(), true)
  state = minimizeHealthDialog(state)
  state = healthDialogStateOnDisconnect(state)
  if (state.mode !== 'minimized') {
    throw new Error('healthDialog: disconnect must preserve minimized mode')
  }
  state = reduceHealthDialogOnProblems(initialHealthDialogState(), true)
  state = reopenHealthDialog(state)
  state = healthDialogStateOnDisconnect(state)
  if (state.mode !== 'hidden') {
    throw new Error('healthDialog: disconnect while open must reset to hidden')
  }

  state = reduceHealthDialogOnProblems(initialHealthDialogState(), true)
  state = noteActiveProblems(state, mcmDownProblems, 5000)
  state = noteForgetSuccess(state)
  if (state.awaitingClose) {
    throw new Error('healthDialog: forget during MCM outage must not switch to recovery view')
  }
  if (!state.forgetSuccess) {
    throw new Error('healthDialog: forget must record forgetSuccess')
  }
  state = reduceHealthDialogOnProblems(state, false)
  if (!state.awaitingClose) {
    throw new Error('healthDialog: forget recovery must await close after problems clear')
  }

  const onvifAuth = collectHealthProblems(
    baseInput({
      systemHealth: {
        mcm: 'online',
        cameras_discovered: 0,
        expected_missing: [],
        autopilot: 'online',
        lua_scripting_disabled: false,
        lua_script: 'ok',
        diagnostics: {
          mavlink_reconnects: 0,
          mavlink_frames_lagged: 0,
          state_events_lagged: 0,
          mcm_consecutive_failures: 0,
        script_reloads: 0,
          backend_version: 'test',
        },
      },
      cameraUuid: 'cam',
      cameraLabel: 'RadCam',
      cameraConnectivity: 'online',
      cameraOnvifAuthError: 'ONVIF authentication failed: wrong password',
    }),
  )
  if (onvifAuth.length !== 1 || onvifAuth[0]?.title !== 'Camera ONVIF password does not match') {
    throw new Error(`healthDialog: expected ONVIF auth problem, got ${onvifAuth[0]?.title}`)
  }
  if (onvifAuth[0]?.kind !== 'camera_onvif_auth') {
    throw new Error(`healthDialog: expected camera_onvif_auth kind, got ${onvifAuth[0]?.kind}`)
  }

  if (recoveryTitle(['camera_stream'], false) !== 'Camera video stream restored') {
    throw new Error(`healthDialog: bad camera_stream recovery title: ${recoveryTitle(['camera_stream'], false)}`)
  }
  if (
    recoveryMessage(['camera_stream'], 0, false)
    !== 'Camera video stream is running again. Click Close to continue.'
  ) {
    throw new Error(`healthDialog: bad camera_stream recovery line: ${recoveryMessage(['camera_stream'], 0, false)}`)
  }

  if (recoveryTitle(['camera_onvif_auth'], false) !== 'Camera ONVIF login restored') {
    throw new Error(`healthDialog: bad camera_onvif_auth recovery title: ${recoveryTitle(['camera_onvif_auth'], false)}`)
  }
  if (
    recoveryMessage(['camera_onvif_auth'], 0, false)
    !== 'ONVIF login succeeded and video is available again. Click Close to continue.'
  ) {
    throw new Error(`healthDialog: bad camera_onvif_auth recovery line: ${recoveryMessage(['camera_onvif_auth'], 0, false)}`)
  }

  const streamError = collectHealthProblems(
    baseInput({
      systemHealth: {
        mcm: 'online',
        cameras_discovered: 1,
        expected_missing: [],
        autopilot: 'online',
        lua_scripting_disabled: false,
        lua_script: 'ok',
        diagnostics: {
          mavlink_reconnects: 0,
          mavlink_frames_lagged: 0,
          state_events_lagged: 0,
          mcm_consecutive_failures: 0,
        script_reloads: 0,
          backend_version: 'test',
        },
      },
      cameraUuid: 'cam',
      cameraLabel: 'RadCam',
      cameraConnectivity: 'online',
      cameraStreamError: 'pipeline error',
    }),
  )
  if (streamError.length !== 1 || streamError[0]?.title !== 'Camera video stream not running') {
    throw new Error(`healthDialog: expected stream problem, got ${streamError[0]?.title}`)
  }
  if (streamError[0]?.kind !== 'camera_stream') {
    throw new Error(`healthDialog: expected camera_stream kind, got ${streamError[0]?.kind}`)
  }

  const streamErrorWhileUnreachable = collectHealthProblems(
    baseInput({
      systemHealth: {
        mcm: 'online',
        cameras_discovered: 1,
        expected_missing: [],
        autopilot: 'online',
        lua_scripting_disabled: false,
        lua_script: 'ok',
        diagnostics: {
          mavlink_reconnects: 0,
          mavlink_frames_lagged: 0,
          state_events_lagged: 0,
          mcm_consecutive_failures: 0,
        script_reloads: 0,
          backend_version: 'test',
        },
      },
      cameraUuid: 'cam',
      cameraLabel: 'RadCam',
      cameraConnectivity: 'unreachable',
      cameraStreamError: 'MCM stream state: stopped',
    }),
  )
  if (streamErrorWhileUnreachable.some((problem) => problem.title === 'Camera video stream not running')) {
    throw new Error('healthDialog: stream error must stay hidden while camera is unreachable')
  }
  if (streamErrorWhileUnreachable.length !== 1 || streamErrorWhileUnreachable[0]?.title !== 'Camera unavailable') {
    throw new Error(`healthDialog: expected unreachable only, got ${streamErrorWhileUnreachable.map((p) => p.title).join(', ')}`)
  }

  const driftInput = baseInput({
    systemHealth: {
      mcm: 'online',
      cameras_discovered: 1,
      expected_missing: [],
      autopilot: 'online',
      lua_scripting_disabled: false,
      lua_script: 'ok',
      parameter_drifts: [
        { name: 'SERVO1_FUNCTION', expected: 33, actual: 0 },
        { name: 'SERVO2_FUNCTION', expected: 34, actual: 0 },
      ],
      diagnostics: {
        mavlink_reconnects: 0,
        mavlink_frames_lagged: 0,
        state_events_lagged: 0,
        mcm_consecutive_failures: 0,
        script_reloads: 0,
        backend_version: 'test',
      },
    },
  })
  const driftProblem = collectHealthProblems(driftInput).find(
    (problem) => problem.kind === 'parameter_drift',
  )
  if (!driftProblem?.showGoToSetup) {
    throw new Error('healthDialog: parameter drift must offer hardware setup')
  }
  if (driftProblem.severity !== 'warning') {
    throw new Error(`healthDialog: parameter drift must be warning, got ${driftProblem.severity}`)
  }
  if (!driftProblem.detail?.includes('SERVO1_FUNCTION: saved 33, autopilot has 0')) {
    throw new Error(`healthDialog: bad drift detail: ${driftProblem.detail}`)
  }
  if (!evaluate(driftInput).degraded) {
    throw new Error('healthDialog: parameter drift must be degraded')
  }

  const manyDrifts = Array.from({ length: 5 }, (_, index) => ({
    name: `SERVO${index + 1}_FUNCTION`,
    expected: 33 + index,
    actual: 0,
  }))
  const cappedDetail = collectHealthProblems(
    baseInput({
      systemHealth: { ...driftInput.systemHealth!, parameter_drifts: manyDrifts },
    }),
  ).find((problem) => problem.kind === 'parameter_drift')?.detail
  if (!cappedDetail?.includes('…and 2 more')) {
    throw new Error(`healthDialog: expected capped drift detail, got ${cappedDetail}`)
  }

  const driftWhileOffline = collectHealthProblems(
    baseInput({
      systemHealth: { ...driftInput.systemHealth!, autopilot: 'mavlink_down' },
    }),
  )
  if (driftWhileOffline.some((problem) => problem.kind === 'parameter_drift')) {
    throw new Error('healthDialog: parameter drift must stay hidden while autopilot is down')
  }

  if (recoveryTitle(['parameter_drift'], false) !== 'Autopilot parameters restored') {
    throw new Error(`healthDialog: bad parameter_drift recovery title: ${recoveryTitle(['parameter_drift'], false)}`)
  }
  if (
    recoveryMessage(['parameter_drift'], 0, false)
    !== 'Autopilot parameters match the saved configuration again. Click Close to continue.'
  ) {
    throw new Error(`healthDialog: bad parameter_drift recovery line: ${recoveryMessage(['parameter_drift'], 0, false)}`)
  }

  const suppressed = collectHealthProblems(
    baseInput({
      cameraUuid: 'cam',
      cameraLabel: 'RadCam',
      cameraConnectivity: 'online',
      cameraOnvifAuthError: 'ONVIF authentication failed: wrong password',
      cameraStreamError: 'pipeline error',
    }),
  )
  if (suppressed.length !== 2) {
    throw new Error(`healthDialog: expected MCM+camera-unknown problems, got ${suppressed.length}`)
  }
  if (suppressed.some((problem) => problem.title === 'Camera ONVIF password does not match')) {
    throw new Error('healthDialog: ONVIF auth must be suppressed while MCM is down')
  }
  if (suppressed.some((problem) => problem.title === 'Camera video stream not running')) {
    throw new Error('healthDialog: stream error must be suppressed while MCM is down')
  }

  const onvifAuthWhileUnreachable = collectHealthProblems(
    baseInput({
      systemHealth: {
        mcm: 'online',
        cameras_discovered: 1,
        expected_missing: [],
        autopilot: 'online',
        lua_scripting_disabled: false,
        lua_script: 'ok',
        diagnostics: {
          mavlink_reconnects: 0,
          mavlink_frames_lagged: 0,
          state_events_lagged: 0,
          mcm_consecutive_failures: 0,
        script_reloads: 0,
          backend_version: 'test',
        },
      },
      cameraUuid: 'cam',
      cameraLabel: 'RadCam',
      cameraConnectivity: 'unreachable',
      cameraOnvifAuthError: 'ONVIF authentication failed: wrong password',
    }),
  )
  if (onvifAuthWhileUnreachable.some((problem) => problem.title === 'Camera ONVIF password does not match')) {
    throw new Error('healthDialog: ONVIF auth must stay hidden while camera is unreachable')
  }

  const mcmSince = 1_000_000
  const mcmNow = mcmSince + 45_000
  const mcmProgress = collectHealthProblems(
    baseInput({ problemFirstSeen: { mcm: mcmSince }, nowMs: mcmNow }),
  )[0]?.progress
  if (mcmProgress !== 'Retrying every 1 sec (about 45 sec so far)…') {
    throw new Error(`healthDialog: MCM progress must use first-seen clock: ${mcmProgress}`)
  }

  const autopilotError = {
    kind: 'autopilot' as const,
    severity: 'error' as const,
    title: 'MAVLink connection unavailable',
    body: 'test',
  }
  let firstSeenState = noteActiveProblems(initialHealthDialogState(), [autopilotError], 1_000_000)
  firstSeenState = noteActiveProblems(firstSeenState, [], 1_060_000)
  firstSeenState = noteActiveProblems(firstSeenState, [autopilotError], 1_120_000)
  if (firstSeenState.problemFirstSeen.autopilot !== 1_120_000) {
    throw new Error(
      `healthDialog: reappeared problem must get fresh first-seen: ${firstSeenState.problemFirstSeen.autopilot}`,
    )
  }

  const sinceTenSec = formatProblemSince(1_000_000, 1_010_000)
  if (!sinceTenSec.includes('(10 sec)')) {
    throw new Error(`healthDialog: formatProblemSince must show seconds below 1 min: ${sinceTenSec}`)
  }

  state = reduceHealthDialogOnProblems(initialHealthDialogState(), true)
  state = noteActiveProblems(state, mcmDownProblems, 5000)
  state = noteActiveProblems(state, mcmDownProblems, 60_000)
  state = reduceHealthDialogOnProblems(state, false)
  state = reduceHealthDialogOnProblems(state, true)
  if (state.episodeKinds.length !== 0 || state.mcmOutageMsPeak !== 0) {
    throw new Error('healthDialog: new problem during recovery must reset episode tracking')
  }

  if (!driftProblem?.body.includes('Autopilot-driven camera controls')) {
    throw new Error(`healthDialog: parameter drift copy must be generic: ${driftProblem?.body}`)
  }

  console.log('healthDialog self-check ok')
}

function evaluate(input: HealthProblemsInput): { active: boolean; degraded: boolean } {
  const flags = evaluateHealthFlags(input)
  return { active: flags.active, degraded: flags.degraded }
}
