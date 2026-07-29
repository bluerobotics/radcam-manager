/**
 * In-flight ownership for coalesced actuator set POSTs.
 *
 * Only the request whose token still matches the flight slot may clear the
 * slot, drain the queue, or roll back optimistic UI on failure.
 */

export type ActuatorFlight = {
  token: number
  value: number
}

export function createActuatorTokenSource() {
  let next = 1
  return {
    next(): number {
      return next++
    },
  }
}

export function ownsActuatorFlight(
  flight: ActuatorFlight | null | undefined,
  token: number,
): boolean {
  return flight != null && flight.token === token
}

/**
 * Whether a failed POST should restore the pre-gesture UI value.
 *
 * Pass `desired` as it was *before* clearing the latch for this attempt.
 * If SERVO already matched `attempted` (desired already cleared by state),
 * skip rollback — hardware already reflects the command.
 */
export function shouldRollbackActuatorUi(args: {
  ownsFlight: boolean
  queued: number | null
  /** Desired latch before any fail-path clear for this attempt. */
  desiredBeforeClear: number | null
  ui: number | null
  attempted: number
  rollback: number | null
  valuesMatch: (a: number, b: number) => boolean
}): boolean {
  if (!args.ownsFlight) return false
  if (args.queued !== null) return false
  // Newer desire still pending — leave optimistic UI alone.
  if (
    args.desiredBeforeClear !== null &&
    !args.valuesMatch(args.desiredBeforeClear, args.attempted)
  ) {
    return false
  }
  // Desired already cleared because SERVO matched — do not snap back.
  if (args.desiredBeforeClear === null) {
    return false
  }
  if (args.rollback === null || args.ui === null) return false
  return args.valuesMatch(args.ui, args.attempted)
}
