import { defineStore } from 'pinia'
import { ref } from 'vue'

import type { GameKey } from '@/helpers/games'

export type SettingsTab = 'general' | 'appearance' | 'games' | 'modpacks' | 'advanced'

// app wide ui requests that the app shell and the modpacks page fulfil
export const useUi = defineStore('ui', () => {
	const settingsRequest = ref<{ tab: SettingsTab; at: number } | null>(null)
	const createRequest = ref<{ game?: GameKey; at: number } | null>(null)

	function openSettings(tab: SettingsTab = 'general') {
		settingsRequest.value = { tab, at: Date.now() }
	}

	// opens new modpack for a game, or else the game selected in modpacks
	function createModpack(game?: GameKey) {
		createRequest.value = { game, at: Date.now() }
	}

	return {
		settingsRequest,
		createRequest,
		openSettings,
		createModpack,
	}
})
