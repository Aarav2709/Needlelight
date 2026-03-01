/**
 * All theseus API calls return serialized values (both return values and errors);
 * So, for example, addDefaultInstance creates a blank Profile object, where the Rust struct is serialized,
 *  and deserialized into a usable JS object.
 */
import { invoke } from '@tauri-apps/api/core'

// Initialize the theseus API state
// This should be called during the initializion/opening of the launcher
export async function initialize_state() {
	try {
		return await invoke('initialize_state')
	} catch {
		return null
	}
}

// Gets active progress bars
export async function progress_bars_list() {
	try {
		return await invoke('plugin:utils|progress_bars_list')
	} catch {
		return []
	}
}

// Get opening command
// For example, if a user clicks on an .mrpack to open the app.
// This should be called once and only when the app is done booting up and ready to receive a command
// Returns a Command struct- see events.js
export async function get_opening_command() {
	try {
		return await invoke('plugin:utils|get_opening_command')
	} catch {
		return null
	}
}
