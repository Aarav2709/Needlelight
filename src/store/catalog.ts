/**
 * Mod catalogs, fetched once per game per session and shared by every modpack of that game.
 * A modpack's install state is merged in separately (see helpers/mods.ts), so toggling or
 * removing mods never refetches the catalog.
 */
import { defineStore } from 'pinia'
import { reactive } from 'vue'

import type { GameKey } from '@/helpers/games'
import { getGameCatalog } from '@/helpers/modpacks'
import type { ModItem } from '@/helpers/types'

type CatalogEntry = {
	items: ModItem[] | null
	loading: boolean
	error: string | null
	loadedAt: number
}

export const useCatalog = defineStore('catalog', () => {
	const entries = reactive<Partial<Record<GameKey, CatalogEntry>>>({})
	const inflight = new Map<GameKey, Promise<void>>()

	function entry(game: GameKey): CatalogEntry {
		if (!entries[game]) entries[game] = { items: null, loading: false, error: null, loadedAt: 0 }
		return entries[game]!
	}

	async function load(game: GameKey, force = false): Promise<void> {
		const current = entry(game)
		if (!force && current.items) return
		const pending = inflight.get(game)
		if (pending) return pending

		current.loading = true
		current.error = null
		const task = getGameCatalog(game)
			.then((catalog) => {
				current.items = catalog.items
				current.loadedAt = Date.now()
			})
			.catch((err) => {
				current.error = typeof err === 'string' ? err : String(err?.message ?? err)
			})
			.finally(() => {
				current.loading = false
				inflight.delete(game)
			})
		inflight.set(game, task)
		return task
	}

	/** Drop cached catalogs, e.g. after changing a custom catalog URL. */
	function invalidate(game?: GameKey) {
		for (const key of Object.keys(entries) as GameKey[]) {
			if (!game || key === game) entries[key] = { items: null, loading: false, error: null, loadedAt: 0 }
		}
	}

	return { entries, entry, load, invalidate }
})
