// modpacks and their installed mods, mod operations reload only that modpack's installed state instead of the catalog
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { defineStore } from 'pinia'
import { computed, reactive, ref } from 'vue'

import { type GameKey,GAMES } from '@/helpers/games'
import {
	activeHkModpack,
	createModpack,
	deleteModpack,
	duplicateModpack,
	editModpack,
	getModpackInstalled,
	installModpackMods,
	launchModpack,
	launchVanilla,
	listModpacks,
	type Modpack,
	restoreOriginalHkMods,
	toggleModpackMod,
	uninstallModpackMod,
} from '@/helpers/modpacks'
import type { InstalledDb } from '@/helpers/types'
import { usePreferences } from '@/store/preferences'

function byRecent(a: Modpack, b: Modpack) {
	// recently played first, then most recently created
	const played = (b.last_played ?? '').localeCompare(a.last_played ?? '')
	return played !== 0 ? played : b.created.localeCompare(a.created)
}

export const useModpacks = defineStore('modpacks', () => {
	const list = ref<Modpack[]>([])
	const loaded = ref(false)
	const loading = ref(false)
	const error = ref<string | null>(null)
	const activeHk = ref<string | null>(null)

	// installed mods per modpack path
	const installed = reactive<Record<string, InstalledDb>>({})
	const installedErrors = reactive<Record<string, string>>({})

	// mods with an operation in flight, keyed by path and mod name on separate lines
	const busy = reactive(new Set<string>())
	// download progress from 0 to 100 per mod name, from the backend's progress events
	const progress = reactive(new Map<string, number>())
	// the modpack path currently launching
	const launching = ref<string | null>(null)
	// in flight installs per modpack, what was asked for and the file downloading right now
	const activity = reactive<Record<string, { names: string[]; current: string | null }>>({})

	let listening = false
	async function listenForProgress() {
		if (listening) return
		listening = true
		await listen<{ item_name?: string; progress?: number }>('mod-install-progress', (event) => {
			const name = event.payload?.item_name
			if (!name) return
			const value = Math.max(0, Math.min(100, Number(event.payload?.progress ?? 0)))
			if (value >= 100) progress.delete(name)
			else progress.set(name, value)
			for (const [path, op] of Object.entries(activity)) {
				if (value < 100 && op.current !== name) {
					op.current = name
					// the previous file is extracted and recorded by now, so show it in the list
					void loadInstalled(path)
				}
			}
		})
	}

	function forGame(game: GameKey) {
		return list.value.filter((m) => m.game === game).sort(byRecent)
	}

	const byPath = computed(() => new Map(list.value.map((m) => [m.path, m])))

	// every modpack in sidebar order: the player's saved order, else hollow knight above silksong oldest first, new ones after the last of their game
	const ordered = computed(() => {
		const gameRank = (m: Modpack) => GAMES.findIndex((g) => g.key === m.game)
		const natural = [...list.value].sort(
			(a, b) => gameRank(a) - gameRank(b) || a.created.localeCompare(b.created),
		)
		const saved = usePreferences().prefs.modpackOrder
		const result = saved.map((path) => byPath.value.get(path)).filter((m): m is Modpack => !!m)
		for (const pack of natural) {
			if (result.includes(pack)) continue
			let at = 0
			for (let i = result.length - 1; i >= 0; i--) {
				if (result[i].game === pack.game) {
					at = i + 1
					break
				}
				if (gameRank(result[i]) < gameRank(pack)) {
					at = i + 1
					break
				}
			}
			result.splice(at, 0, pack)
		}
		return result
	})

	// moves a modpack to an index in the sidebar order and remembers the order
	function move(path: string, index: number) {
		const paths = ordered.value.map((m) => m.path)
		const from = paths.indexOf(path)
		if (from < 0) return
		paths.splice(from, 1)
		paths.splice(Math.max(0, Math.min(index, paths.length)), 0, path)
		usePreferences().prefs.modpackOrder = paths
	}

	async function load() {
		loading.value = true
		error.value = null
		try {
			const [modpacks, active] = await Promise.all([listModpacks(), activeHkModpack()])
			list.value = modpacks
			activeHk.value = active
			loaded.value = true
			void listenForProgress()
		} catch (err) {
			error.value = String(err)
			throw err
		} finally {
			loading.value = false
		}
	}

	async function ensureLoaded() {
		if (!loaded.value) await load()
	}

	async function loadInstalled(path: string) {
		try {
			installed[path] = await getModpackInstalled(path)
			Reflect.deleteProperty(installedErrors, path)
		} catch (err) {
			installedErrors[path] = String(err)
		}
	}

	// installed state for several modpacks at once, used for the sidebar's counts
	async function loadInstalledFor(paths: string[]) {
		await Promise.all(paths.filter((p) => !installed[p]).map(loadInstalled))
	}

	const key = (path: string, mod: string) => `${path}\n${mod}`
	const isBusy = (path: string, mod: string) => busy.has(key(path, mod))

	async function withBusy<T>(path: string, mods: string[], task: () => Promise<T>): Promise<T> {
		for (const mod of mods) busy.add(key(path, mod))
		try {
			return await task()
		} finally {
			for (const mod of mods) {
				busy.delete(key(path, mod))
				progress.delete(mod)
			}
			// dependencies report progress under their own names, so clear anything a failure left behind
			if (busy.size === 0) progress.clear()
			await loadInstalled(path)
		}
	}

	// installs or updates mods and their dependencies
	async function install(path: string, names: string[]) {
		for (const name of names) progress.set(name, 0)
		activity[path] = { names, current: null }
		try {
			return await withBusy(path, names, () => installModpackMods(path, names))
		} finally {
			Reflect.deleteProperty(activity, path)
		}
	}

	function uninstall(path: string, name: string) {
		return withBusy(path, [name], () => uninstallModpackMod(path, name))
	}

	// enables or disables several mods, switches flip right away and the reload afterwards confirms or reverts them
	function setEnabled(path: string, names: string[], enable: boolean) {
		const db = installed[path]
		for (const name of names) {
			const entry = db?.mods[name] ?? db?.not_in_modlinks_mods[name]
			if (entry) entry.enabled = enable
		}
		return withBusy(path, names, async () => {
			for (const name of names) await toggleModpackMod(path, name, enable)
		})
	}

	async function create(opts: Parameters<typeof createModpack>[0]) {
		const created = await createModpack(opts)
		await load()
		return created
	}

	async function edit(path: string, changes: { name?: string; description?: string }) {
		await editModpack(path, changes)
		await load()
	}

	async function duplicate(path: string) {
		const copy = await duplicateModpack(path)
		await load()
		return copy
	}

	async function remove(path: string) {
		await deleteModpack(path)
		Reflect.deleteProperty(installed, path)
		const prefs = usePreferences().prefs
		for (const game of Object.keys(prefs.lastModpack) as GameKey[]) {
			if (prefs.lastModpack[game] === path) Reflect.deleteProperty(prefs.lastModpack, game)
		}
		await load()
	}

	async function afterLaunch() {
		if (usePreferences().prefs.minimizeOnLaunch) {
			await getCurrentWindow()
				.minimize()
				.catch(() => {})
		}
	}

	async function launch(path: string) {
		if (launching.value) return
		launching.value = path
		try {
			const message = await launchModpack(path)
			await afterLaunch()
			await load()
			return message
		} finally {
			launching.value = null
		}
	}

	async function launchWithoutMods() {
		const message = await launchVanilla()
		await afterLaunch()
		return message
	}

	async function restoreOriginal() {
		const restored = await restoreOriginalHkMods()
		activeHk.value = await activeHkModpack()
		return restored
	}

	return {
		list,
		loaded,
		loading,
		error,
		activeHk,
		installed,
		installedErrors,
		progress,
		activity,
		launching,
		byPath,
		ordered,
		move,
		forGame,
		load,
		ensureLoaded,
		loadInstalled,
		loadInstalledFor,
		isBusy,
		install,
		uninstall,
		setEnabled,
		create,
		edit,
		duplicate,
		remove,
		launch,
		launchWithoutMods,
		restoreOriginal,
	}
})
