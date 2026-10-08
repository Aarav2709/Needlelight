// the active game and where each game is installed, the backend settings file is the source of truth
import { invoke } from '@tauri-apps/api/core'
import { defineStore } from 'pinia'
import { computed, reactive, ref, toRaw } from 'vue'

import { applyGameTheme } from '@/helpers/game-theme'
import { type GameKey, GAMES, isGameKey } from '@/helpers/games'
import { getGameAvailability, isGameFolder } from '@/helpers/modpacks'
import type { BackendSettings } from '@/helpers/types'

export const useGames = defineStore('games', () => {
	const settings = ref<BackendSettings | null>(null)
	const loaded = ref(false)
	const switching = ref<GameKey | null>(null)

	// whether each game was found on this computer, null until checked
	const found = reactive<Record<GameKey, boolean | null>>(
		Object.fromEntries(GAMES.map((g) => [g.key, null])) as Record<GameKey, boolean | null>,
	)
	// games currently being searched for
	const searching = reactive(new Set<GameKey>())
	// the last game whose install folder was set, found by a search or picked by the player
	const configured = ref<{ game: GameKey; at: number } | null>(null)
	const searched = new Set<GameKey>()

	const activeGame = computed<GameKey>(() =>
		isGameKey(settings.value?.game) ? settings.value!.game : 'hollow_knight',
	)

	async function refreshAvailability() {
		try {
			for (const entry of await getGameAvailability()) found[entry.game] = entry.found
		} catch {
			// leave the previous answer in place
		}
	}

	async function load() {
		settings.value = await invoke<BackendSettings>('load_settings')
		loaded.value = true
		applyGameTheme(activeGame.value)
		await refreshAvailability()
		return settings.value
	}

	async function ensureLoaded() {
		if (!loaded.value) await load()
	}

	// saves a changed copy of the settings, the ui updates right away and the backend's normalized version loads in the background
	async function update(change: (draft: BackendSettings) => void) {
		await ensureLoaded()
		const draft = structuredClone(toRaw(settings.value!)) as BackendSettings
		draft.managed_folders = { ...(draft.managed_folders ?? {}) }
		draft.custom_modlinks_by_game = { ...(draft.custom_modlinks_by_game ?? {}) }
		change(draft)
		await invoke('save_settings', { settings: draft })
		settings.value = draft
		applyGameTheme(activeGame.value)
		void load().catch(() => {})
	}

	// switches the active game, saves it, and retints the app with that game's accent
	async function switchGame(game: GameKey) {
		if (game === activeGame.value || switching.value) return
		switching.value = game
		try {
			await update((draft) => {
				draft.game = game
				draft.managed_folder = draft.managed_folders[game] ?? ''
			})
		} finally {
			switching.value = null
		}
	}

	async function saveLocation(game: GameKey, folder: string) {
		await update((draft) => {
			draft.managed_folders[game] = folder
			if (draft.game === game) draft.managed_folder = folder
		})
		await refreshAvailability()
		if (found[game]) configured.value = { game, at: Date.now() }
	}

	// looks for a game that isn't found yet, at most once per session since the search can walk whole drives
	async function findGame(game: GameKey) {
		await ensureLoaded()
		if (found[game] !== false || searched.has(game) || searching.has(game)) return
		searched.add(game)
		searching.add(game)
		try {
			const location = await invoke<string | null>('auto_detect_managed_folder', { game })
			if (location) await saveLocation(game, location)
		} catch {
			// not found, the player can locate it by hand
		} finally {
			searching.delete(game)
		}
	}

	// uses a folder the player picked, false when the game isn't in it
	async function locateGame(game: GameKey, folder: string): Promise<boolean> {
		if (!(await isGameFolder(game, folder))) return false
		await saveLocation(game, folder)
		return true
	}

	return {
		settings,
		loaded,
		switching,
		activeGame,
		found,
		searching,
		configured,
		load,
		ensureLoaded,
		refreshAvailability,
		switchGame,
		findGame,
		locateGame,
	}
})
