import { runActuatorFlightSelfCheck } from './actuatorFlight.selfcheck'
import { runBackendVersionSelfCheck } from './backendVersion.selfcheck'
import { runHealthDialogSelfCheck } from './healthDialog.selfcheck'
import { runPendingFieldsSelfCheck } from './pendingFields.selfcheck'

runPendingFieldsSelfCheck()
runActuatorFlightSelfCheck()
runBackendVersionSelfCheck()
runHealthDialogSelfCheck()
