// leftover modrinth app calls, the backend has none of these commands so each falls back to an empty result
import { invoke } from '@tauri-apps/api/core'

// initializes backend state
export async function initialize_state() {
	try {
		return await invoke('initialize_state')
	} catch {
		return null
	}
}

// active progress bars
export async function progress_bars_list() {
	try {
		return await invoke('plugin:utils|progress_bars_list')
	} catch {
		return []
	}
}

// the command the app was opened with, like a file or url
export async function get_opening_command() {
	try {
		return await invoke('plugin:utils|get_opening_command')
	} catch {
		return null
	}
}
