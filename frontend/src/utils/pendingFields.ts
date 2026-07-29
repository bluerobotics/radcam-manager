/**
 * Per-field optimistic write ownership.
 *
 * Each in-flight field write gets a token. Success/fail only settles that token.
 * Remote merges always keep `entry.value` for pending keys (never trust a caller
 * local). Restore bumps epoch so late per-field settles are ignored.
 */
export type PendingEntry<V> = {
  token: number
  previous: V
  value: V
}

export function createPendingFields<K extends string, V>() {
  const pending = new Map<K, PendingEntry<V>>()
  let nextToken = 1
  let epoch = 0

  const begin = (key: K, previous: V, value: V): { token: number; epoch: number } => {
    const token = nextToken++
    pending.set(key, { token, previous, value })
    return { token, epoch }
  }

  const settleSuccess = (
    key: K,
    token: number,
    settleEpoch: number,
    applySettled: (attempted: V) => void,
  ): boolean => {
    if (settleEpoch !== epoch) return false
    const entry = pending.get(key)
    if (!entry || entry.token !== token) return false
    pending.delete(key)
    applySettled(entry.value)
    return true
  }

  const settleFail = (
    key: K,
    token: number,
    settleEpoch: number,
    currentEqualsAttempted: (attempted: V) => boolean,
    revert: (previous: V) => void,
  ): boolean => {
    if (settleEpoch !== epoch) return false
    const entry = pending.get(key)
    if (!entry || entry.token !== token) return false
    pending.delete(key)
    if (currentEqualsAttempted(entry.value)) {
      revert(entry.previous)
    }
    return true
  }

  /** Merge remote snapshot; pending keys always win via `entry.value`. */
  const mergeRemote = <T extends Record<string, unknown>>(incoming: T): T => {
    if (pending.size === 0) return incoming
    const merged = { ...incoming }
    for (const [key, entry] of pending) {
      ;(merged as Record<string, unknown>)[key] = entry.value
    }
    return merged
  }

  /** Whole-object restore: invalidate all outstanding per-field settles. */
  const beginRestore = (): number => {
    epoch += 1
    pending.clear()
    return epoch
  }

  const clear = (): void => {
    epoch += 1
    pending.clear()
  }

  const hasPending = (): boolean => pending.size > 0

  const isPendingKey = (key: K): boolean => pending.has(key)

  return {
    begin,
    settleSuccess,
    settleFail,
    mergeRemote,
    beginRestore,
    clear,
    hasPending,
    isPendingKey,
    get epoch() {
      return epoch
    },
  }
}
