// Types mirroring the Tauri backend (src-tauri/src/backend). Keep these in sync with the Rust
// structs they name; serde serializes Option<T> as `T | null`.

import type { GameKey } from '@/helpers/games'

// Mirrors backend/models.rs::ModState (serde tag = "kind", rename_all = "snake_case").
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

// Mirrors backend/models.rs::ModItem.
export type ModItem = {
	name: string
	description: string
	version: string
	/** Required dependencies, by mod name. */
	dependencies: string[]
	link: string
	sha256: string
	repository: string
	issues: string
	tags: string[]
	/** Mods this one has optional integration with (Hollow Knight ModLinks only). */
	integrations: string[]
	authors: string[]
	state: ModState
	/** Minimum version per dependency, when the catalog publishes it (Thunderstore). */
	dependency_versions?: Record<string, string>
	icon?: string | null
	downloads?: number | null
	updated_at?: string | null
	/** The project's own website, when listed separately from `repository`. */
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

// Mirrors backend/models.rs::PersistedInstalled: one modpack's installed-mods database.
export type InstalledDb = {
	mods: Record<string, { enabled: boolean; version: string; pinned: boolean }>
	not_in_modlinks_mods: Record<
		string,
		{ enabled: boolean; pinned: boolean; installed: boolean; modlinks_mod: boolean }
	>
}

// Mirrors backend/settings.rs::AppSettings (the fields the UI reads or writes).
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
	/** The game was found where Needlelight expects it. */
	found: boolean
}

export type ModReadme = {
	markdown: string
	/** Base for resolving relative image paths. */
	image_base: string | null
	/** Base for resolving relative links. */
	link_base: string | null
}
