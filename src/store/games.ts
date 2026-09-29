/**
 * The active game and where each game is installed. The backend's settings file is the
 * source of truth: `game` is the active game, `managed_folders` holds each game's location.
 * Locations are found automatically; the player is only asked when a game can't be found.
 */
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

	/** Whether each game was found on this computer (null = not checked yet). */
	const found = reactive<Record<GameKey, boolean | null>>(
		Object.fromEntries(GAMES.map((g) => [g.key, null])) as Record<GameKey, boolean | null>,
	)
	/** Games currently being searched for. */
	const searching = reactive(new Set<GameKey>())
	/** The last game whose install folder was set (found by a search, or picked by the player). */
	const configured = ref<{ game: GameKey; at: number } | null>(null)
	const searched = new Set<GameKey>()

	const activeGame = computed<GameKey>(() =>
		isGameKey(settings.value?.game) ? settings.value!.game : 'hollow_knight',
	)

	async function refreshAvailability() {
		try {
			for (const entry of await getGameAvailability()) found[entry.game] = entry.found
		} catch {
			/* leave the previous answer in place */
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

	/**
	 * Save a changed copy of the settings. The UI updates from it right away; the backend's
	 * normalized version (and, for a game with no known location, a one-time search that can
	 * take a while) is picked up in the background.
	 */
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

	/** Switch the active game. Persists, and retints the app with that game's accent. */
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

	/**
	 * Look for a game that isn't found yet (at most once per session per game, since the
	 * search can walk whole drives). Saves the location when it turns up.
	 */
	async function findGame(game: GameKey) {
		await ensureLoaded()
		if (found[game] !== false || searched.has(game) || searching.has(game)) return
		searched.add(game)
		searching.add(game)
		try {
			const location = await invoke<string | null>('auto_detect_managed_folder', { game })
			if (location) await saveLocation(game, location)
		} catch {
			/* not found: the player can locate it by hand */
		} finally {
			searching.delete(game)
		}
	}

	/** Use a folder the player picked. Returns false when the game isn't in it. */
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
