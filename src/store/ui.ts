import { defineStore } from 'pinia'
import { ref } from 'vue'

import type { GameKey } from '@/helpers/games'

export type SettingsTab = 'general' | 'appearance' | 'games' | 'modpacks' | 'advanced'

// app wide ui requests that the app shell and the modpacks page fulfil
export const useUi = defineStore('ui', () => {
	const settingsRequest = ref<{ tab: SettingsTab; at: number } | null>(null)
	const welcomeRequest = ref(0)
	const createRequest = ref<{ game?: GameKey; at: number } | null>(null)
	const shortcutsRequest = ref(0)

	function openSettings(tab: SettingsTab = 'general') {
		settingsRequest.value = { tab, at: Date.now() }
	}

	function showWelcome() {
		welcomeRequest.value++
	}

	// opens new modpack for a game, or else the game selected in modpacks
	function createModpack(game?: GameKey) {
		createRequest.value = { game, at: Date.now() }
	}

	function showShortcuts() {
		shortcutsRequest.value++
	}

	return {
		settingsRequest,
		welcomeRequest,
		createRequest,
		shortcutsRequest,
		openSettings,
		showWelcome,
		createModpack,
		showShortcuts,
	}
})
