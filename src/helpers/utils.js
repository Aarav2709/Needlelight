import { invoke } from '@tauri-apps/api/core'

import { get_full_path, get_mod_full_path } from '@/helpers/profile'

export async function isDev() {
  return false
}

export async function areUpdatesEnabled() {
  return false
}

export async function getUpdateSize(updateRid) {
  try {
    return await invoke('get_update_size', { rid: updateRid })
  } catch {
    return null
  }
}

export async function enqueueUpdateForInstallation(updateRid) {
  try {
    return await invoke('enqueue_update_for_installation', { rid: updateRid })
  } catch {
    return null
  }
}

export async function removeEnqueuedUpdate() {
  try {
    return await invoke('remove_enqueued_update')
  } catch {
    return null
  }
}

// One of 'Windows', 'Linux', 'MacOS'
export async function getOS() {
  const userAgentDataPlatform = navigator.userAgentData?.platform?.toLowerCase?.() ?? ''
  const platform = (navigator.platform || '').toLowerCase()
  const userAgent = (navigator.userAgent || '').toLowerCase()
  const source = `${userAgentDataPlatform} ${platform} ${userAgent}`

  if (source.includes('mac')) {
    return 'MacOS'
  }
  if (source.includes('win')) {
    return 'Windows'
  }
  return 'Linux'
}

export async function isNetworkMetered() {
  try {
    return await invoke('plugin:utils|is_network_metered')
  } catch {
    return false
  }
}

export async function openPath(path) {
  try {
    return await invoke('plugin:utils|open_path', { path })
  } catch {
    return null
  }
}

export async function highlightInFolder(path) {
  try {
    return await invoke('plugin:utils|highlight_in_folder', { path })
  } catch {
    return null
  }
}

export async function showLauncherLogsFolder() {
  try {
    return await invoke('plugin:utils|show_launcher_logs_folder', {})
  } catch {
    return null
  }
}

// Opens a profile's folder in the OS file explorer
export async function showProfileInFolder(path) {
  const fullPath = await get_full_path(path)
  return await openPath(fullPath)
}

export async function highlightModInProfile(profilePath, projectPath) {
  const fullPath = await get_mod_full_path(profilePath, projectPath)
  return await highlightInFolder(fullPath)
}

export async function restartApp() {
  try {
    return await invoke('restart_app')
  } catch {
    window.location.reload()
  }
}

export const releaseColor = (releaseType) => {
  switch (releaseType) {
    case 'release':
      return 'green'
    case 'beta':
      return 'orange'
    case 'alpha':
      return 'red'
    default:
      return ''
  }
}

export async function copyToClipboard(text) {
  await navigator.clipboard.writeText(text)
}
