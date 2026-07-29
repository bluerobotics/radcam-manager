import {
  createActuatorTokenSource,
  ownsActuatorFlight,
  shouldRollbackActuatorUi,
  type ActuatorFlight,
} from './actuatorFlight'

/** Assert-based self-check for actuator flight tokens (no test framework). */
export function runActuatorFlightSelfCheck(): void {
  const tokens = createActuatorTokenSource()
  const t1 = tokens.next()
  const t2 = tokens.next()
  const flight: ActuatorFlight = { token: t1, value: 80 }

  if (!ownsActuatorFlight(flight, t1)) {
    throw new Error('actuatorFlight: owner should match token')
  }
  if (ownsActuatorFlight(flight, t2)) {
    throw new Error('actuatorFlight: superseded token must not own flight')
  }
  if (ownsActuatorFlight(null, t1)) {
    throw new Error('actuatorFlight: null flight must not own')
  }

  const match = (a: number, b: number) => Math.abs(a - b) <= 0.5

  if (
    !shouldRollbackActuatorUi({
      ownsFlight: true,
      queued: null,
      desired: null,
      ui: 80,
      attempted: 80,
      rollback: 50,
      valuesMatch: match,
    })
  ) {
    throw new Error('actuatorFlight: expected rollback when attempt still on UI')
  }

  if (
    shouldRollbackActuatorUi({
      ownsFlight: false,
      queued: null,
      desired: null,
      ui: 80,
      attempted: 80,
      rollback: 50,
      valuesMatch: match,
    })
  ) {
    throw new Error('actuatorFlight: superseded flight must not rollback')
  }

  if (
    shouldRollbackActuatorUi({
      ownsFlight: true,
      queued: 90,
      desired: null,
      ui: 80,
      attempted: 80,
      rollback: 50,
      valuesMatch: match,
    })
  ) {
    throw new Error('actuatorFlight: queued newer value must block rollback')
  }

  if (
    shouldRollbackActuatorUi({
      ownsFlight: true,
      queued: null,
      desired: 90,
      ui: 90,
      attempted: 80,
      rollback: 50,
      valuesMatch: match,
    })
  ) {
    throw new Error('actuatorFlight: newer desired must block rollback')
  }

  console.log('actuatorFlight self-check ok')
}

if (typeof process !== 'undefined' && process.env?.NODE_ENV === 'test') {
  runActuatorFlightSelfCheck()
}
