import { runActuatorFlightSelfCheck } from './actuatorFlight.selfcheck'
import { runPendingFieldsSelfCheck } from './pendingFields.selfcheck'

runPendingFieldsSelfCheck()
runActuatorFlightSelfCheck()
