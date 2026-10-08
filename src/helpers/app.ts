import { invoke } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'

export type AppFolder = 'data' | 'modpacks' | 'install_log'

// opens one of needlelight's own folders in the file manager
export const openAppFolder = (folder: AppFolder) => invoke<void>('open_app_folder', { folder })

// the releases page, also used as the changelog
export const RELEASES_URL = 'https://github.com/Aarav2709/Needlelight/releases'

// opens web and mail links in the system browser instead of the app window
export function isExternalUrl(href: string): boolean {
	try {
		const url = new URL(href)
		if (url.protocol === 'mailto:') return true
		const local = ['localhost', 'tauri.localhost', '127.0.0.1'].includes(url.hostname)
		return (url.protocol === 'http:' || url.protocol === 'https:') && !local
	} catch {
		return false
	}
}

export const openExternal = (href: string) => openUrl(href)
