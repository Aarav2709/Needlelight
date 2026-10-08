// listeners for events the rust backend emits, each resolves to an unlisten function

import { listen } from '@tauri-apps/api/event'

async function safeListen(eventName, callback) {
  try {
    return await listen(eventName, callback)
  } catch {
    return () => { }
  }
}

// payload for the loading event: event type, loader uuid, fraction, and message
export async function loading_listener(callback) {
  return await safeListen('loading', (event) => callback(event.payload))
}

// payload for the process event: uuid, pid, event, and message
export async function process_listener(callback) {
  return await safeListen('process', (event) => callback(event.payload))
}

// payload for the profile event: uuid, name, profile path, path, and event
export async function profile_listener(callback) {
  return await safeListen('profile', (event) => callback(event.payload))
}

// payload for the command event: event and id
export async function command_listener(callback) {
  return await safeListen('command', (event) => {
    callback(event.payload)
  })
}

// payload for the warning event: message
export async function warning_listener(callback) {
  return await safeListen('warning', (event) => callback(event.payload))
}

