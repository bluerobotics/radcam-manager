import { runActuatorFlightSelfCheck } from './actuatorFlight.selfcheck'
import { runHealthDialogSelfCheck } from './healthDialog.selfcheck'
import { runPendingFieldsSelfCheck } from './pendingFields.selfcheck'

runPendingFieldsSelfCheck()
runActuatorFlightSelfCheck()
runHealthDialogSelfCheck()
