import { createPendingFields } from './pendingFields'

/** Assert-based self-check for pending-field ownership (no test framework). */
export function runPendingFieldsSelfCheck(): void {
  const pending = createPendingFields<'a' | 'b', number>()

  const a1 = pending.begin('a', 0, 1)
  const b1 = pending.begin('b', 10, 11)

  // Fail of A must not drop B's pending.
  pending.settleFail(
    'a',
    a1.token,
    a1.epoch,
    () => true,
    () => {},
  )
  if (!pending.isPendingKey('b')) {
    throw new Error('pendingFields: fail of A cleared B')
  }

  // Success of B must not clear a re-begun A.
  const a2 = pending.begin('a', 0, 2)
  pending.settleSuccess('b', b1.token, b1.epoch, () => {})
  if (!pending.isPendingKey('a')) {
    throw new Error('pendingFields: success of B cleared A')
  }

  // Pending A keeps entry.value (2), not a poisoned local or server 99.
  const merged = pending.mergeRemote({ a: 99, b: 99 })
  if (merged.a !== 2) {
    throw new Error('pendingFields: mergeRemote did not keep entry.value for A')
  }
  if (merged.b !== 99) {
    throw new Error('pendingFields: mergeRemote should take server B after settle')
  }

  // Settle A then merge: only remaining pending keys are protected.
  const pending2 = createPendingFields<'a' | 'b', number>()
  const pa = pending2.begin('a', 0, 1)
  pending2.begin('b', 10, 11)
  pending2.settleSuccess('a', pa.token, pa.epoch, () => {})
  const afterSettle = pending2.mergeRemote({ a: 50, b: 50 })
  if (afterSettle.a !== 50) {
    throw new Error('pendingFields: settled A should take server value')
  }
  if (afterSettle.b !== 11) {
    throw new Error('pendingFields: pending B must keep entry.value after sibling settle')
  }

  // Restore then edit then late restore response must not wipe the new pending.
  const pending3 = createPendingFields<'a', number>()
  pending3.beginRestore()
  pending3.begin('a', 0, 7)
  const late = pending3.mergeRemote({ a: 1 })
  if (late.a !== 7) {
    throw new Error('pendingFields: late restore merge overwrote post-restore edit')
  }

  pending.beginRestore()
  if (
    pending.settleSuccess('a', a2.token, a2.epoch, () => {
      throw new Error('should not apply')
    })
  ) {
    throw new Error('pendingFields: restore epoch did not reject late settle')
  }

  console.log('pendingFields self-check ok')
}
