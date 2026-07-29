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

  const merged = pending.mergeRemote({ a: 99, b: 99 }, { a: 2, b: 11 })
  if (merged.a !== 2) {
    throw new Error('pendingFields: mergeRemote overwrote pending A')
  }
  if (merged.b !== 99) {
    throw new Error('pendingFields: mergeRemote kept settled B local')
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

if (typeof process !== 'undefined' && process.env?.NODE_ENV === 'test') {
  runPendingFieldsSelfCheck()
}
