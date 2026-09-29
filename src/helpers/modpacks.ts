import { invoke } from '@tauri-apps/api/core'

import type { GameKey } from '@/helpers/games'
import type { CatalogResponse, GameAvailability, InstalledDb, ModReadme } from '@/helpers/types'

/** Mirrors src-tauri/src/backend/profiles.rs::GameInstance (the fields the UI uses). */
export type Modpack = {
	path: string
	game: GameKey
	name: string
	description?: string | null
	created: string
	modified: string
	last_played?: string | null
}

export const listModpacks = (game?: GameKey) =>
	invoke<Modpack[]>('profile_list', { game: game ?? null })

export const getModpack = (path: string) => invoke<Modpack>('profile_get', { path })

export const createModpack = (opts: {
	name: string
	game: GameKey
	description?: string
	icon?: string | null
}) =>
	invoke<Modpack>('profile_create', {
		name: opts.name,
		game: opts.game,
		description: opts.description ?? null,
		icon: opts.icon ?? null,
	})

export const editModpack = (path: string, changes: { name?: string; description?: string }) =>
	invoke<void>('profile_edit', { path, editProfile: changes })

export const duplicateModpack = (path: string) => invoke<Modpack>('profile_duplicate', { path })

export const deleteModpack = (path: string) => invoke<void>('profile_remove', { path })

/** The full mod catalog for a game; every item's state is `not_installed`. */
export const getGameCatalog = (game: GameKey) =>
	invoke<CatalogResponse>('game_catalog', { game })

/** A modpack's installed mods, reconciled with the files on disk. */
export const getModpackInstalled = (path: string) =>
	invoke<InstalledDb>('modpack_installed', { path })

/** Installs mods plus their dependencies; also used to update installed mods. */
export const installModpackMods = (path: string, names: string[]) =>
	invoke<void>('modpack_install_mods', { path, names })

export const uninstallModpackMod = (path: string, name: string) =>
	invoke<void>('modpack_uninstall_mod', { path, name })

export const toggleModpackMod = (path: string, name: string, enable: boolean) =>
	invoke<void>('modpack_toggle_mod', { path, name, enable })

export const launchModpack = (path: string) => invoke<string>('modpack_launch', { path })

/** Hollow Knight only: puts back the Mods folder from before the first modpack was applied. */
export const restoreOriginalHkMods = () => invoke<boolean>('modpack_restore_original_mods')

export const activeHkModpack = () => invoke<string | null>('modpack_active_hk')

/** Whether each supported game is installed where Needlelight expects it. */
export const getGameAvailability = () => invoke<GameAvailability[]>('game_availability')

/** Whether `folder` contains `game` (used when the player locates a game by hand). */
export const isGameFolder = (game: GameKey, folder: string) =>
	invoke<boolean>('game_folder_valid', { game, folder })

/** A mod's README from its project page, or null when it doesn't publish one. */
export const getModReadme = (url: string, version: string) =>
	invoke<ModReadme | null>('mod_readme', { url, version })

/** Launch the game without any modpack (Hollow Knight temporarily disables the API). */
export const launchVanilla = () => invoke<string>('launch_game', { modded: false })

/** URL for a modpack's page (its folder path, encoded), or for browsing mods to install in it. */
export const modpackRoute = (modpack: Pick<Modpack, 'path'>, page?: 'browse') =>
	`/modpacks/${encodeURIComponent(modpack.path)}${page === 'browse' ? '/browse' : ''}`

const RELATIVE_UNITS: [number, Intl.RelativeTimeFormatUnit][] = [
	[31536000, 'year'],
	[2592000, 'month'],
	[604800, 'week'],
	[86400, 'day'],
	[3600, 'hour'],
	[60, 'minute'],
]
const relativeFormat = new Intl.RelativeTimeFormat(undefined, { numeric: 'auto' })

/** "3 days ago", "yesterday", "just now". */
export function relativeTime(iso?: string | null): string | null {
	if (!iso) return null
	const time = new Date(iso).getTime()
	if (Number.isNaN(time)) return null
	const seconds = (time - Date.now()) / 1000
	for (const [size, unit] of RELATIVE_UNITS) {
		if (Math.abs(seconds) >= size) return relativeFormat.format(Math.round(seconds / size), unit)
	}
	return 'just now'
}

export function playedLabel(iso?: string | null): string {
	const relative = relativeTime(iso)
	return relative ? `Played ${relative}` : 'Never played'
}
