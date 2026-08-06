import { backendClient } from './backendClient'

export function rebootCamera(cameraUuid: string): Promise<unknown> {
  return backendClient.request('POST', '/camera/control', {
    camera_uuid: cameraUuid,
    action: 'restart',
  })
}
