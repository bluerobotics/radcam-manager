import {
  closeHealthDialog,
  evaluateHealthFlags,
  healthDialogView,
  initialHealthDialogState,
  minimizeHealthDialog,
  noteMcmAttempts,
  recoveryWhileMinimizedToast,
  reduceHealthDialogOnProblems,
  reopenHealthDialog,
} from './healthDialogState'
import {
  collectHealthProblems,
  recoveryMessage,
  recoveryTitle,
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
        backend_version: 'test',
      },
    },
    cameraUuid: null,
    cameraLabel: '',
    cameraConnectivity: 'unknown',
    mcmAttemptsPeak: 0,
    ...overrides,
  })

  const down = evaluate(baseInput())
  if (!down.active || !down.degraded) {
    throw new Error('healthDialog: MCM down must be active+degraded')
  }
  if (collectHealthProblems(baseInput()).length !== 1) {
    throw new Error('healthDialog: expected one MCM problem')
  }

  state = reduceHealthDialogOnProblems(initialHealthDialogState(), down.degraded)
  if (state.mode !== 'open') throw new Error('healthDialog: must auto-open on degraded problems')

  // Idempotent while already open — must keep the same reference.
  const openAgain = reduceHealthDialogOnProblems(state, down.degraded)
  if (openAgain !== state) {
    throw new Error('healthDialog: reduce while open must be idempotent')
  }

  state = noteMcmAttempts(state, 5)
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
  state = noteMcmAttempts(state, 5)
  state = reduceHealthDialogOnProblems(state, false)
  view = healthDialogView(state, false)
  if (!view.showDialog || !view.awaitingClose) {
    throw new Error('healthDialog: recovery must keep dialog open awaiting close')
  }
  if (recoveryTitle(['mcm'], false) !== 'Video service restored') {
    throw new Error(`healthDialog: bad recovery title: ${recoveryTitle(['mcm'], false)}`)
  }
  if (
    recoveryMessage(['mcm'], 5, false)
    !== 'BlueOS video service is back (after about 5 sec). Click Close to continue.'
  ) {
    throw new Error(`healthDialog: bad recovery line: ${recoveryMessage(['mcm'], 5, false)}`)
  }
  if (recoveryMessage([], 0, false) !== 'All clear. Click Close to continue.') {
    throw new Error(`healthDialog: bad empty recovery line: ${recoveryMessage([], 0, false)}`)
  }
  if (
    recoveryWhileMinimizedToast({
      ...initialHealthDialogState(),
      episodeKinds: ['mcm'],
      mcmAttemptsPeak: 5,
    })
    !== 'BlueOS video service is back (after about 5 sec).'
  ) {
    throw new Error('healthDialog: minimized recovery toast must omit Close hint')
  }

  state = closeHealthDialog()
  if (state.mode !== 'hidden' || state.mcmAttemptsPeak !== 0) {
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

  // A stale or absent autopilot script must offer the one-click fix, and only while the
  // autopilot can actually take it.
  for (const lua_script of ['missing', 'outdated'] as const) {
    const scriptInput = baseInput({
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

  console.log('healthDialog self-check ok')
}

function evaluate(input: HealthProblemsInput): { active: boolean; degraded: boolean } {
  const flags = evaluateHealthFlags(input)
  return { active: flags.active, degraded: flags.degraded }
}
