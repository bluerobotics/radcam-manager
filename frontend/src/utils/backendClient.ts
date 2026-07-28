import type {
  ConnectionStats,
  UiDismissField,
  WsClientMessage,
  WsEvent,
  WsRequest,
  WsResponse,
} from '@/bindings/radcam_api'

type PendingRequest = {
  method: 'GET' | 'POST'
  path: string
  body?: unknown
  resolve: (body: unknown) => void
  reject: (error: Error) => void
  timeoutId: ReturnType<typeof setTimeout>
  sent: boolean
}

export type BackendRequestError = Error & {
  response?: {
    status?: number
    statusText?: string
    data?: unknown
  }
}

export type ConnectionState = 'disconnected' | 'connecting' | 'connected'

export type { ConnectionStats }

// Above reboot wait budget (150s) and matching camera_ui::REBOOT_GRACE (180s).
const REQUEST_TIMEOUT_MS = 180_000
const REQUEST_MAX_ATTEMPTS = 3
const RECONNECT_BASE_MS = 1_000
const RECONNECT_MAX_MS = 30_000
const STALE_CONNECTION_MS = 90_000

function makeRequestError(status: number, body: unknown): BackendRequestError {
  const error = new Error(`Request failed with status ${status}`) as BackendRequestError
  error.response = { status, data: body }
  return error
}

function isRetriableConnectionError(error: unknown, method: 'GET' | 'POST', sent: boolean): boolean {
  if (!(error instanceof Error)) return false
  const message = error.message

  // Never retry after a POST may have reached the backend.
  if (method === 'POST' && sent) return false

  // Timeouts after send are ambiguous for every method.
  if (message === 'Request timed out' && sent) return false

  return (
    message === 'WebSocket closed'
    || message === 'WebSocket connection failed'
    || message === 'WebSocket send failed'
    || message === 'Request timed out'
  )
}

function isTransportError(error: unknown): boolean {
  if (!(error instanceof Error)) return false
  const message = error.message
  return (
    message === 'WebSocket closed'
    || message === 'WebSocket connection failed'
    || message === 'WebSocket send failed'
    || message === 'Request timed out'
  )
}

function transportErrorMessage(error: unknown): string {
  if (error instanceof Error && error.message === 'Request timed out') {
    return 'Request timed out. Check the connection and try again.'
  }
  return 'Backend disconnected. Changes were not applied.'
}

class BackendClient {
  private ws: WebSocket | null = null
  private nextId = 1
  private pending = new Map<number, PendingRequest>()
  private eventHandlers = new Map<string, Set<(body: unknown) => void>>()
  private connectPromise: Promise<void> | null = null
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null
  private staleCheckTimer: ReturnType<typeof setInterval> | null = null
  // Reserved for an explicit disconnect() path; reconnect runs while false.
  private intentionalClose = false
  private reconnectAttempt = 0
  private lastMessageAt = 0
  private connectionState: ConnectionState = 'disconnected'
  private connectionStateHandlers = new Set<(state: ConnectionState, previousState: ConnectionState) => void>()
  private transportErrorHandlers = new Set<(message: string) => void>()
  private listenersRegistered = false
  private subscribedCameraUuid: string | null = null
  private cameraSubscribeCounts = new Map<string, number>()
  private pendingSubscribe: string | null = null
  private subscribeQueued = false

  private wsUrl(): string {
    const url = new URL('v1/ws', window.location.href)
    url.protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    return url.href
  }

  private setConnectionState(state: ConnectionState): void {
    if (this.connectionState === state) return
    const previousState = this.connectionState
    this.connectionState = state
    for (const handler of this.connectionStateHandlers) {
      handler(state, previousState)
    }
  }

  private notifyTransportError(error: unknown): void {
    if (!isTransportError(error)) return
    const message = transportErrorMessage(error)
    for (const handler of this.transportErrorHandlers) {
      handler(message)
    }
  }

  onConnectionState(handler: (state: ConnectionState, previousState: ConnectionState) => void): () => void {
    this.connectionStateHandlers.add(handler)
    handler(this.connectionState, this.connectionState)
    return () => {
      this.connectionStateHandlers.delete(handler)
    }
  }

  onTransportError(handler: (message: string) => void): () => void {
    this.transportErrorHandlers.add(handler)
    return () => {
      this.transportErrorHandlers.delete(handler)
    }
  }

  private registerListeners(): void {
    if (this.listenersRegistered) return
    this.listenersRegistered = true

    // Process-lifetime listeners on the singleton; never removed.
    window.addEventListener('online', () => {
      this.connect().catch(() => {
        this.scheduleReconnect()
      })
    })

    document.addEventListener('visibilitychange', () => {
      if (document.visibilityState !== 'visible') return
      if (this.ws?.readyState === WebSocket.OPEN) return
      this.connect().catch(() => {
        this.scheduleReconnect()
      })
    })
  }

  connect(): Promise<void> {
    this.registerListeners()

    if (this.ws?.readyState === WebSocket.OPEN) {
      return Promise.resolve()
    }

    if (this.ws?.readyState === WebSocket.CONNECTING && this.connectPromise) {
      return this.connectPromise
    }

    // Wait out CLOSING so a stale onclose cannot orphan a newer socket.
    if (this.ws?.readyState === WebSocket.CLOSING) {
      return new Promise((resolve, reject) => {
        const started = Date.now()
        const check = setInterval(() => {
          if (this.ws?.readyState === WebSocket.OPEN) {
            clearInterval(check)
            resolve()
            return
          }
          if (!this.ws || this.ws.readyState === WebSocket.CLOSED) {
            clearInterval(check)
            this.connect().then(resolve).catch(reject)
            return
          }
          if (Date.now() - started > 5_000) {
            clearInterval(check)
            this.ws = null
            this.connect().then(resolve).catch(reject)
          }
        }, 50)
      })
    }

    if (this.connectPromise) {
      return this.connectPromise
    }

    this.intentionalClose = false
    this.setConnectionState('connecting')

    this.connectPromise = new Promise<void>((resolve, reject) => {
      let settled = false
      const ws = new WebSocket(this.wsUrl())
      this.ws = ws

      ws.onopen = () => {
        settled = true
        this.connectPromise = null
        this.lastMessageAt = Date.now()
        this.setConnectionState('connected')
        this.startStaleCheck()
        // Re-subscribe immediately; camera/list also re-queues as a second chance.
        if (this.subscribedCameraUuid) {
          this.queueSubscribe(this.subscribedCameraUuid)
        }
        resolve()
      }

      ws.onmessage = (event) => {
        this.lastMessageAt = Date.now()
        // Reset backoff only after the connection proves it can deliver traffic.
        this.reconnectAttempt = 0
        this.handleMessage(String(event.data))
      }

      ws.onclose = () => {
        if (this.ws !== ws) return
        this.ws = null
        this.connectPromise = null
        this.stopStaleCheck()
        this.setConnectionState('disconnected')
        this.rejectAllPending(new Error('WebSocket closed'))

        if (!settled) {
          settled = true
          reject(new Error('WebSocket connection failed'))
        }

        if (!this.intentionalClose) {
          this.scheduleReconnect()
        }
      }

      ws.onerror = () => {
        // onclose handles reconnect and initial failure
      }
    })

    return this.connectPromise
  }

  private startStaleCheck(): void {
    this.stopStaleCheck()
    this.staleCheckTimer = setInterval(() => {
      if (this.ws?.readyState !== WebSocket.OPEN) return
      if (Date.now() - this.lastMessageAt < STALE_CONNECTION_MS) return
      this.ws.close()
    }, STALE_CONNECTION_MS / 3)
  }

  private stopStaleCheck(): void {
    if (this.staleCheckTimer !== null) {
      clearInterval(this.staleCheckTimer)
      this.staleCheckTimer = null
    }
  }

  private scheduleReconnect(): void {
    if (this.intentionalClose || this.reconnectTimer !== null) return

    const delay = Math.min(
      RECONNECT_BASE_MS * 2 ** this.reconnectAttempt,
      RECONNECT_MAX_MS,
    )
    const jitter = Math.floor(Math.random() * RECONNECT_BASE_MS)
    this.reconnectAttempt++

    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null
      this.connect().catch(() => {
        this.scheduleReconnect()
      })
    }, delay + jitter)
  }

  private rejectAllPending(error: Error): void {
    for (const pending of this.pending.values()) {
      clearTimeout(pending.timeoutId)
      pending.reject(error)
    }
    this.pending.clear()
  }

  private handleMessage(raw: string): void {
    let message: WsResponse | WsEvent
    try {
      message = JSON.parse(raw) as WsResponse | WsEvent
    } catch (error) {
      console.warn('Ignoring invalid WebSocket JSON', error)
      return
    }

    if (message.type === 'response') {
      const pending = this.pending.get(message.id)
      if (!pending) return
      this.pending.delete(message.id)
      clearTimeout(pending.timeoutId)

      if (message.status >= 200 && message.status < 300) {
        pending.resolve(message.body)
      } else {
        pending.reject(makeRequestError(message.status, message.body))
      }
      return
    }

    if (message.type === 'event') {
      // Re-push subscribe when the list arrives so a reconnect during MCM boot
      // does not leave the client permanently unsubscribed.
      if (message.event === 'camera/list' && this.subscribedCameraUuid) {
        this.queueSubscribe(this.subscribedCameraUuid)
      }

      const handlers = this.eventHandlers.get(message.event)
      if (!handlers) return
      for (const handler of handlers) {
        handler(message.body)
      }
    }
  }

  private sendRequest<T>(method: 'GET' | 'POST', path: string, body?: unknown): Promise<T> {
    const id = this.nextId++
    const request: WsRequest = { id, method, path }
    if (body !== undefined) {
      request.body = body
    }

    return new Promise<T>((resolve, reject) => {
      const pending: PendingRequest = {
        method,
        path,
        body,
        resolve: (data) => resolve(data as T),
        reject,
        timeoutId: setTimeout(() => {
          if (!this.pending.has(id)) return
          this.pending.delete(id)
          reject(new Error('Request timed out'))
        }, REQUEST_TIMEOUT_MS),
        sent: false,
      }

      this.pending.set(id, pending)

      try {
        if (this.ws?.readyState !== WebSocket.OPEN) {
          throw new Error('WebSocket send failed')
        }
        this.ws.send(JSON.stringify(request))
        pending.sent = true
      } catch {
        clearTimeout(pending.timeoutId)
        this.pending.delete(id)
        this.ws?.close()
        reject(new Error('WebSocket send failed'))
      }
    })
  }

  async request<T>(method: 'GET' | 'POST', path: string, body?: unknown): Promise<T> {
    let lastError: unknown
    // Deliberately sticky across attempts: once a POST may have reached the backend,
    // never retry it (non-idempotent camera/autopilot control).
    let sent = false

    for (let attempt = 0; attempt < REQUEST_MAX_ATTEMPTS; attempt++) {
      try {
        // Let the reconnect timer own the next connect when one is already scheduled.
        if (this.reconnectTimer !== null && this.ws?.readyState !== WebSocket.OPEN) {
          throw new Error('WebSocket connection failed')
        }
        await this.connect()
        const result = await this.sendRequest<T>(method, path, body)
        return result
      } catch (error) {
        lastError = error
        if (error instanceof Error && error.message !== 'WebSocket send failed' && error.message !== 'WebSocket connection failed') {
          sent = true
        }
        if (!isRetriableConnectionError(error, method, sent) || attempt === REQUEST_MAX_ATTEMPTS - 1) {
          this.notifyTransportError(error)
          throw error
        }
      }
    }

    throw lastError
  }

  onEvent(event: string, handler: (body: unknown) => void): () => void {
    let handlers = this.eventHandlers.get(event)
    if (!handlers) {
      handlers = new Set()
      this.eventHandlers.set(event, handlers)
    }
    handlers.add(handler)

    return () => {
      const current = this.eventHandlers.get(event)
      if (!current) return
      current.delete(handler)
      if (current.size === 0) {
        this.eventHandlers.delete(event)
      }
    }
  }

  subscribeCamera(cameraUuid: string): void {
    const count = this.cameraSubscribeCounts.get(cameraUuid) ?? 0
    this.cameraSubscribeCounts.set(cameraUuid, count + 1)

    if (this.subscribedCameraUuid && this.subscribedCameraUuid !== cameraUuid) {
      const previous = this.subscribedCameraUuid
      // Single active wire UUID: drop previous counts and unsubscribe once.
      this.cameraSubscribeCounts.delete(previous)
      this.sendCameraSubscription('unsubscribe', previous)
    }

    this.subscribedCameraUuid = cameraUuid
    // Always send subscribe so the backend re-pushes UI + snapshot for this
    // consumer (refcount > 0 alone used to skip the frame and leave remounts blank).
    this.queueSubscribe(cameraUuid)
  }

  /** Collapse same-uuid subscribes issued in the same tick into one wire frame. */
  private queueSubscribe(cameraUuid: string): void {
    this.pendingSubscribe = cameraUuid
    if (this.subscribeQueued) return
    this.subscribeQueued = true
    queueMicrotask(() => {
      this.subscribeQueued = false
      const pending = this.pendingSubscribe
      this.pendingSubscribe = null
      // Skip if unsubscribe cleared the active uuid in the same tick.
      if (pending && pending === this.subscribedCameraUuid) {
        this.sendCameraSubscription('subscribe', pending)
      }
    })
  }

  unsubscribeCamera(cameraUuid: string): void {
    const count = this.cameraSubscribeCounts.get(cameraUuid) ?? 0
    if (count <= 1) {
      this.cameraSubscribeCounts.delete(cameraUuid)
      if (this.subscribedCameraUuid === cameraUuid) {
        this.subscribedCameraUuid = null
        this.sendCameraSubscription('unsubscribe', cameraUuid)
      }
      return
    }

    this.cameraSubscribeCounts.set(cameraUuid, count - 1)
  }

  dismissUi(cameraUuid: string, field: UiDismissField): void {
    const message: WsClientMessage = {
      type: 'ui_dismiss',
      camera_uuid: cameraUuid,
      field,
    }
    const payload = JSON.stringify(message)
    const send = (): void => {
      if (this.ws?.readyState === WebSocket.OPEN) {
        this.ws.send(payload)
      }
    }

    if (this.ws?.readyState === WebSocket.OPEN) {
      send()
      return
    }

    this.connect().then(send).catch((error) => {
      // Local dismiss still cleared the overlay; wire retry is best-effort.
      console.warn('Failed to send ui_dismiss', error)
    })
  }

  private sendCameraSubscription(type: 'subscribe' | 'unsubscribe', cameraUuid: string): void {
    const message: WsClientMessage = type === 'subscribe'
      ? { type: 'subscribe', camera_uuid: cameraUuid }
      : { type: 'unsubscribe', camera_uuid: cameraUuid }
    const payload = JSON.stringify(message)
    const send = (): void => {
      if (this.ws?.readyState === WebSocket.OPEN) {
        this.ws.send(payload)
      }
    }

    if (this.ws?.readyState === WebSocket.OPEN) {
      send()
      return
    }

    this.connect().then(send).catch((error) => {
      console.warn(`Failed to send camera ${type}`, error)
    })
  }
}

export const backendClient = new BackendClient()
