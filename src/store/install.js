/**
 * Install store for Needlelight — Hollow Knight mod management.
 * Talks directly to the Tauri backend commands.
 */
import { invoke } from '@tauri-apps/api/core'
import { defineStore } from 'pinia'

export const useInstall = defineStore('installStore', {
	state: () => ({
		installing: {},  // Map of mod name -> boolean (currently installing)
	}),
	actions: {
		async installMod(modName) {
			this.installing[modName] = true
			try {
				await invoke('install_mod', { name: modName })
			} finally {
				delete this.installing[modName]
			}
		},
		async uninstallMod(modName) {
			await invoke('uninstall_mod', { name: modName })
		},
		async toggleMod(modName, enable) {
			await invoke('toggle_mod', { name: modName, enable })
		},
		async installApi() {
			await invoke('install_api')
		},
		async refreshCatalog(fetchOfficial = true) {
			return await invoke('refresh_catalog', { fetchOfficial })
		},
	},
})
