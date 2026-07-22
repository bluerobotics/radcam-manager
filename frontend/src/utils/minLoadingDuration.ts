import type { Ref } from 'vue'

const MIN_LOADING_MS = 3000

let endTimeout: ReturnType<typeof setTimeout> | undefined

export function startMinLoading(loading: Ref<boolean>): number {
  clearTimeout(endTimeout)
  loading.value = true
  return Date.now()
}

export function endMinLoading(loading: Ref<boolean>, startedAt: number, immediate = false): void {
  clearTimeout(endTimeout)
  if (immediate) {
    loading.value = false
    return
  }
  const remaining = MIN_LOADING_MS - (Date.now() - startedAt)
  endTimeout = setTimeout(() => {
    loading.value = false
  }, Math.max(0, remaining))
}
