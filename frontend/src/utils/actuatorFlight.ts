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
 * Caller should clear the desired latch for `attempted` before calling when
 * that latch still targets this attempt.
 */
export function shouldRollbackActuatorUi(args: {
  ownsFlight: boolean
  queued: number | null
  desired: number | null
  ui: number | null
  attempted: number
  rollback: number | null
  valuesMatch: (a: number, b: number) => boolean
}): boolean {
  if (!args.ownsFlight) return false
  if (args.queued !== null) return false
  if (args.desired !== null) return false
  if (args.rollback === null || args.ui === null) return false
  return args.valuesMatch(args.ui, args.attempted)
}
