// types mirroring the tauri backend structs, keep them in sync, serde sends empty options as null

import type { GameKey } from '@/helpers/games'

// mirrors the backend mod state enum, tagged by kind in snake case
export type ModState =
	| { kind: 'installed'; enabled: boolean; pinned: boolean; version: string; updated: boolean }
	| { kind: 'not_installed'; installing?: boolean }
	| {
			kind: 'not_in_modlinks'
			enabled: boolean
			pinned: boolean
			installed: boolean
			modlinks_mod: boolean
	  }

// mirrors the backend mod item
export type ModItem = {
	name: string
	description: string
	version: string
	// required dependencies by mod name
	dependencies: string[]
	link: string
	sha256: string
	repository: string
	issues: string
	tags: string[]
	// mods this one has optional integration with, hollow knight modlinks only
	integrations: string[]
	authors: string[]
	state: ModState
	// minimum version per dependency, when the catalog publishes it (thunderstore)
	dependency_versions?: Record<string, string>
	icon?: string | null
	downloads?: number | null
	updated_at?: string | null
	// the project's own website, when listed separately from the repository
	homepage?: string | null
}

export type ApiInfo = {
	url: string
	version: string
	sha256: string
}

export type CatalogResponse = {
	items: ModItem[]
	api: ApiInfo
	api_installed: boolean
	api_enabled: boolean
}

// mirrors the backend persisted installed struct, one modpack's installed mods database
export type InstalledDb = {
	mods: Record<string, { enabled: boolean; version: string; pinned: boolean }>
	not_in_modlinks_mods: Record<
		string,
		{ enabled: boolean; pinned: boolean; installed: boolean; modlinks_mod: boolean }
	>
}

// mirrors the backend app settings, only the fields the ui reads or writes
export type BackendSettings = {
	managed_folder: string
	game: GameKey
	managed_folders: Partial<Record<GameKey, string>>
	use_custom_modlinks: boolean
	custom_modlinks_uri: string
	custom_modlinks_by_game: Partial<Record<GameKey, { enabled: boolean; uri: string }>>
	use_github_mirror: boolean
	github_mirror_format: string
	low_storage_mode: boolean
}

export type GameAvailability = {
	game: GameKey
	// the game was found where needlelight expects it
	found: boolean
}

export type ModReadme = {
	markdown: string
	// base for resolving relative image paths
	image_base: string | null
	// base for resolving relative links
	link_base: string | null
}
