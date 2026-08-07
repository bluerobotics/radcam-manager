import {
  collectHealthProblems,
  formatProblemSince,
  problemsSummary,
  recoveryMessage,
  recoveryTitle,
  sortProblemsBySeverity,
  type HealthProblem,
  type HealthProblemKind,
  type HealthProblemsInput,
} from './systemHealthProblems'

const HEALTH_DIALOG_MINIMIZED_KEY = 'radcam-health-dialog-minimized'

function isHealthDialogMinimizedPersisted(): boolean {
  try {
    return sessionStorage.getItem(HEALTH_DIALOG_MINIMIZED_KEY) === '1'
  } catch {
    return false
  }
}

function setHealthDialogMinimizedPersisted(minimized: boolean): void {
  try {
    if (minimized) {
      sessionStorage.setItem(HEALTH_DIALOG_MINIMIZED_KEY, '1')
    } else {
      sessionStorage.removeItem(HEALTH_DIALOG_MINIMIZED_KEY)
    }
  } catch {
    // sessionStorage unavailable (SSR, private mode quota, etc.)
  }
}

export type HealthDialogMode = 'hidden' | 'open' | 'minimized'

export type HealthDialogState = {
  mode: HealthDialogMode
  /** Dialog stays open after recovery until the user clicks Close. */
  awaitingClose: boolean
  mcmOutageMsPeak: number
  /** True once this episode hit a degraded (banner/modal-worthy) problem. */
  episodeDegraded: boolean
  /** Problem kinds seen during this degraded episode (for recovery copy). */
  episodeKinds: HealthProblemKind[]
  problemFirstSeen: Partial<Record<HealthProblemKind, number>>
  forgetSuccess: boolean
}

export type HealthDialogView = {
  showDialog: boolean
  showDegradedBanner: boolean
  awaitingClose: boolean
  recoveryTitle: string | null
  recoveryMessage: string | null
}

export function initialHealthDialogState(): HealthDialogState {
  return {
    mode: 'hidden',
    awaitingClose: false,
    mcmOutageMsPeak: 0,
    episodeDegraded: false,
    episodeKinds: [],
    problemFirstSeen: {},
    forgetSuccess: false,
  }
}

/** Advance sticky dialog state from a health snapshot transition. */
export function reduceHealthDialogOnProblems(
  state: HealthDialogState,
  degraded: boolean,
): HealthDialogState {
  if (degraded) {
    if (state.mode === 'hidden') {
      const mode = isHealthDialogMinimizedPersisted() ? 'minimized' : 'open'
      return { ...state, mode, awaitingClose: false, episodeDegraded: true, forgetSuccess: false }
    }
    if (state.awaitingClose) {
      return {
        ...state,
        awaitingClose: false,
        episodeDegraded: true,
        forgetSuccess: false,
        episodeKinds: [],
        mcmOutageMsPeak: 0,
      }
    }
    if (!state.episodeDegraded) {
      return { ...state, episodeDegraded: true }
    }
    return state
  }

  if (!state.episodeDegraded) {
    return state
  }

  // Recovered from a degraded episode.
  if (state.mode === 'open') {
    if (state.awaitingClose) return state
    return { ...state, awaitingClose: true }
  }
  if (state.mode === 'minimized') {
    setHealthDialogMinimizedPersisted(false)
    return initialHealthDialogState()
  }
  return state
}

export function minimizeHealthDialog(state: HealthDialogState): HealthDialogState {
  setHealthDialogMinimizedPersisted(true)
  return { ...state, mode: 'minimized', awaitingClose: false }
}

export function reopenHealthDialog(state: HealthDialogState): HealthDialogState {
  setHealthDialogMinimizedPersisted(false)
  return { ...state, mode: 'open', awaitingClose: false }
}

export function closeHealthDialog(): HealthDialogState {
  setHealthDialogMinimizedPersisted(false)
  return initialHealthDialogState()
}

/** Keep minimized intent across a transport disconnect; drop ephemeral episode data. */
export function healthDialogStateOnDisconnect(state: HealthDialogState): HealthDialogState {
  if (state.mode === 'minimized' || isHealthDialogMinimizedPersisted()) {
    return { ...initialHealthDialogState(), mode: 'minimized' }
  }
  return initialHealthDialogState()
}

export function degradedBannerCopy(problems: HealthProblem[]): { title: string; body: string } {
  const notable = problems.filter((problem) => problem.severity !== 'info')
  const summary = problemsSummary(problems)
  if (summary) {
    return {
      title: summary,
      body: notable.map((problem) => problem.title).join(' · '),
    }
  }
  const primary = notable[0] ?? problems[0]
  if (!primary) {
    return {
      title: 'Systems need attention',
      body: 'One or more systems need attention.',
    }
  }
  return { title: primary.title, body: primary.body }
}

export function recoveryWhileMinimizedToast(state: HealthDialogState): string {
  return recoveryMessage(
    state.episodeKinds,
    state.mcmOutageMsPeak,
    state.forgetSuccess,
    false,
  )
}

export function noteActiveProblems(
  state: HealthDialogState,
  problems: HealthProblem[],
  nowMs: number,
): HealthDialogState {
  const firstSeen: Partial<Record<HealthProblemKind, number>> = {}
  const kinds = [...state.episodeKinds]
  let changed = false
  let mcmOutageMsPeak = state.mcmOutageMsPeak

  for (const problem of problems) {
    if (problem.severity === 'error' || problem.severity === 'warning') {
      if (!kinds.includes(problem.kind)) {
        kinds.push(problem.kind)
        changed = true
      }
    }
    const prev = state.problemFirstSeen[problem.kind]
    if (prev != null) {
      firstSeen[problem.kind] = prev
    } else {
      firstSeen[problem.kind] = nowMs
      changed = true
    }
  }

  const mcmSince = firstSeen.mcm
  if (mcmSince != null) {
    const elapsed = nowMs - mcmSince
    if (elapsed > mcmOutageMsPeak) {
      mcmOutageMsPeak = elapsed
      changed = true
    }
  }

  if (Object.keys(firstSeen).length !== Object.keys(state.problemFirstSeen).length) {
    changed = true
  }

  if (!changed) return state
  return { ...state, episodeKinds: kinds, problemFirstSeen: firstSeen, mcmOutageMsPeak }
}

export function noteForgetSuccess(state: HealthDialogState): HealthDialogState {
  if (state.mode !== 'open') return state
  const kinds: HealthProblemKind[] = state.episodeKinds.includes('camera')
    ? state.episodeKinds
    : [...state.episodeKinds, 'camera']
  return {
    ...state,
    forgetSuccess: true,
    episodeKinds: kinds,
  }
}

export function enrichHealthProblems(
  problems: HealthProblem[],
  firstSeen: Partial<Record<HealthProblemKind, number>>,
  nowMs: number,
): HealthProblem[] {
  return sortProblemsBySeverity(
    problems.map((problem) => ({
      ...problem,
      since:
        firstSeen[problem.kind] != null
          ? formatProblemSince(firstSeen[problem.kind]!, nowMs)
          : null,
    })),
  )
}

export function healthDialogView(state: HealthDialogState, degraded: boolean): HealthDialogView {
  const showDialog = state.mode === 'open'
  const showDegradedBanner = state.mode === 'minimized' && degraded
  const awaitingRecovery = state.awaitingClose && state.mode === 'open'
  const recoveryTitleText = awaitingRecovery
    ? recoveryTitle(state.episodeKinds, state.forgetSuccess)
    : null
  const recoveryMessageText = awaitingRecovery
    ? recoveryMessage(state.episodeKinds, state.mcmOutageMsPeak, state.forgetSuccess)
    : null
  return {
    showDialog,
    showDegradedBanner,
    awaitingClose: state.awaitingClose,
    recoveryTitle: recoveryTitleText,
    recoveryMessage: recoveryMessageText,
  }
}

export function evaluateHealthFlags(input: HealthProblemsInput): {
  active: boolean
  degraded: boolean
  problems: ReturnType<typeof collectHealthProblems>
} {
  const problems = collectHealthProblems(input)
  return {
    active: problems.length > 0,
    degraded: problems.some(
      (problem) => problem.severity === 'error' || problem.severity === 'warning',
    ),
    problems,
  }
}
