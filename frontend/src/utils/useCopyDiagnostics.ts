import { onUnmounted, ref, type Ref } from 'vue'

import {
  buildDiagnosticsPayload,
  copyText,
  diagnosticsJson,
  type DiagnosticsContext,
} from '@/utils/copyText'

const COPY_FEEDBACK_MS = 5000

export type UseCopyDiagnostics = {
  copyFeedback: Ref<string | null>
  copyFallbackText: Ref<string | null>
  clearCopyFeedbackMessage: () => void
  clearCopyState: () => void
  copyDiagnostics: (ctx: DiagnosticsContext) => Promise<void>
}

export function useCopyDiagnostics(): UseCopyDiagnostics {
  const copyFeedback = ref<string | null>(null)
  const copyFallbackText = ref<string | null>(null)
  let copyFeedbackTimer: ReturnType<typeof setTimeout> | null = null

  const clearCopyFeedbackMessage = (): void => {
    if (copyFeedbackTimer != null) {
      clearTimeout(copyFeedbackTimer)
      copyFeedbackTimer = null
    }
    copyFeedback.value = null
  }

  const clearCopyState = (): void => {
    clearCopyFeedbackMessage()
    copyFallbackText.value = null
  }

  const showCopyFeedback = (message: string, fallback: string | null = null): void => {
    if (copyFeedbackTimer != null) {
      clearTimeout(copyFeedbackTimer)
      copyFeedbackTimer = null
    }
    copyFeedback.value = message
    if (fallback != null) {
      copyFallbackText.value = fallback
    }
    copyFeedbackTimer = setTimeout(() => {
      copyFeedback.value = null
      copyFeedbackTimer = null
    }, COPY_FEEDBACK_MS)
  }

  const copyDiagnostics = async (ctx: DiagnosticsContext): Promise<void> => {
    clearCopyFeedbackMessage()

    let text = ''
    try {
      text = diagnosticsJson(buildDiagnosticsPayload(ctx))
    } catch (error) {
      showCopyFeedback(`Failed to build diagnostics: ${error}`)
      return
    }

    const result = await copyText(text)
    if (result === 'copied') {
      showCopyFeedback('Diagnostics copied to clipboard.')
      return
    }
    showCopyFeedback(
      'Clipboard unavailable — select the text below and copy manually.',
      text,
    )
  }

  onUnmounted(() => {
    clearCopyState()
  })

  return {
    copyFeedback,
    copyFallbackText,
    clearCopyFeedbackMessage,
    clearCopyState,
    copyDiagnostics,
  }
}
