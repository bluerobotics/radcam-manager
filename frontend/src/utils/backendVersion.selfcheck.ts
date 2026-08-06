import { observeBackendVersion } from './backendClient'

export function runBackendVersionSelfCheck(): void {
  const fresh = (): { initial: string | null; changed: boolean } => ({
    initial: null,
    changed: false,
  })

  const firstSeen = fresh()
  observeBackendVersion(firstSeen, '1.0.0-abc')
  if (firstSeen.initial !== '1.0.0-abc' || firstSeen.changed) {
    throw new Error('backendVersion: first value should be remembered without triggering')
  }

  observeBackendVersion(firstSeen, '1.0.0-abc')
  if (firstSeen.changed) {
    throw new Error('backendVersion: same value must not trigger a change')
  }

  const changed = fresh()
  observeBackendVersion(changed, '1.0.0-abc')
  observeBackendVersion(changed, '1.0.1-def')
  if (!changed.changed) {
    throw new Error('backendVersion: different value must latch changed')
  }

  observeBackendVersion(changed, '9.9.9-zzz')
  if (!changed.changed) {
    throw new Error('backendVersion: changed state must stay latched')
  }

  const absent = fresh()
  observeBackendVersion(absent, undefined)
  observeBackendVersion(absent, '1.0.0-abc')
  if (absent.initial !== '1.0.0-abc' || absent.changed) {
    throw new Error('backendVersion: absent values should be ignored until a real version arrives')
  }

  console.log('backendVersion self-check ok')
}
