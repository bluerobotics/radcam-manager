type RequestError = {
  response?: {
    status?: number
    statusText?: string
    data?: unknown
  }
  message?: string
}

export function formatRequestError(error: unknown): string {
  const err = error as RequestError
  const { response } = err

  if (response) {
    const { status, statusText, data } = response

    if (typeof data === 'object' && data !== null && 'message' in data) {
      const message = (data as { message: unknown }).message
      if (typeof message === 'string' && message.length > 0) {
        return message
      }
    }

    if (typeof data === 'string') {
      const trimmed = data.trim()
      if (trimmed.startsWith('<')) {
        return status ? `HTTP ${status}${statusText ? ` ${statusText}` : ''}` : 'Request failed'
      }
      return trimmed.length > 200 ? `${trimmed.slice(0, 200)}…` : trimmed
    }

    if (status) {
      return `HTTP ${status}${statusText ? ` ${statusText}` : ''}`
    }
  }

  if (typeof err.message === 'string' && err.message.length > 0) {
    return err.message
  }

  return 'Unknown error'
}
