import type { CameraConnectivity, SystemHealth } from '@/bindings/radcam_api'

/** Best-effort copy for BlueOS HTTP pages (often non-secure-context). */
export type CopyTextResult = 'copied' | 'manual'

export type DiagnosticsContext = {
  system_health: SystemHealth | null
  camera_connectivity?: CameraConnectivity | null
  problem_titles?: string[]
  page_url?: string
  page_title?: string
  user_agent?: string
}

export function buildDiagnosticsPayload(ctx: DiagnosticsContext): Record<string, unknown> {
  return {
    exported_at: new Date().toISOString(),
    page: {
      url: ctx.page_url ?? (typeof window !== 'undefined' ? window.location.href : null),
      title: ctx.page_title ?? (typeof document !== 'undefined' ? document.title : null),
    },
    user_agent: ctx.user_agent ?? (typeof navigator !== 'undefined' ? navigator.userAgent : null),
    problems: ctx.problem_titles ?? [],
    system_health: ctx.system_health,
    camera_connectivity: ctx.camera_connectivity ?? null,
  }
}

export async function copyText(text: string): Promise<CopyTextResult> {
  if (!text) return 'manual'

  try {
    if (window.isSecureContext && navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text)
      return 'copied'
    }
  } catch {
    // Fall through to execCommand / manual select.
  }

  const field = document.createElement('textarea')
  field.value = text
  field.setAttribute('readonly', '')
  // Nearly invisible but still focusable — off-screen nodes are ignored by some browsers.
  field.style.cssText =
    'position:fixed;top:0;left:0;width:2em;height:2em;padding:0;border:none;outline:none;box-shadow:none;background:transparent;opacity:0.01;z-index:2147483647'
  document.body.appendChild(field)
  field.focus()
  field.select()
  field.setSelectionRange(0, text.length)

  let ok = false
  try {
    ok = document.execCommand('copy')
  } catch {
    ok = false
  }
  document.body.removeChild(field)
  return ok ? 'copied' : 'manual'
}

export function diagnosticsJson(blob: unknown): string {
  return JSON.stringify(
    blob,
    (_key, value) => (typeof value === 'bigint' ? value.toString() : value),
    2,
  )
}
