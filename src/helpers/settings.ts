/**
 * All theseus API calls return serialized values (both return values and errors);
 * So, for example, addDefaultInstance creates a blank Profile object, where the Rust struct is serialized,
 *  and deserialized into a usable JS object.
 */
import { invoke } from '@tauri-apps/api/core'

import type { Hooks, MemorySettings, WindowSize } from '@/helpers/types'
import type { ColorTheme, FeatureFlag } from '@/store/theme.ts'

// Settings object
/*

Settings {
    "memory": MemorySettings,
    "game_resolution": [int int],
    "custom_java_args": [String ...],
    "custom_env_args" : [(string, string) ... ]>,
    "java_globals": Hash of (string, Path),
    "default_user": Uuid string (can be null),
    "hooks": Hooks,
    "max_concurrent_downloads": uint,
    "version": u32,
    "collapsed_navigation": bool,
}

Memorysettings {
    "min": u32, can be null,
    "max": u32,
}

*/

export type AppSettings = {
	max_concurrent_downloads: number
	max_concurrent_writes: number

	theme: ColorTheme
	locale: string
	default_page: 'home' | 'library'
	collapsed_navigation: boolean
	hide_nametag_skins_page: boolean
	advanced_rendering: boolean
	native_decorations: boolean
	toggle_sidebar: boolean

	telemetry: boolean
	discord_rpc: boolean
	personalized_ads: boolean

	onboarded: boolean

	extra_launch_args: string[]
	custom_env_vars: [string, string][]
	memory: MemorySettings
	force_fullscreen: boolean
	game_resolution: WindowSize
	hide_on_process_start: boolean
	hooks: Hooks

	custom_dir?: string | null
	prev_custom_dir?: string | null
	migrated: boolean

	developer_mode: boolean
	feature_flags: Record<FeatureFlag, boolean>

	skipped_update: string | null
	pending_update_toast_for_version: string | null
	auto_download_updates: boolean | null

	version: number
}

// Get full settings object
export async function get() {
	try {
		return (await invoke('plugin:settings|settings_get')) as AppSettings
	} catch {
		const loaded = (await invoke('load_settings')) as any
		return {
			max_concurrent_downloads: 4,
			max_concurrent_writes: 4,
			theme: 'dark',
			locale: 'en-US',
			default_page: 'home',
			collapsed_navigation: false,
			hide_nametag_skins_page: false,
			advanced_rendering: false,
			native_decorations: true,
			toggle_sidebar: false,
			telemetry: false,
			discord_rpc: false,
			personalized_ads: false,
			onboarded: true,
			extra_launch_args: [],
			custom_env_vars: [],
			memory: { min: null, max: 4096 },
			force_fullscreen: false,
			game_resolution: [1280, 720],
			hide_on_process_start: false,
			hooks: {
				pre_launch: null,
				wrapper: null,
				post_exit: null,
			},
			custom_dir: loaded.managed_folder || null,
			prev_custom_dir: loaded.managed_folder || null,
			migrated: true,
			developer_mode: false,
			feature_flags: {},
			skipped_update: null,
			pending_update_toast_for_version: null,
			auto_download_updates: null,
			version: 1,
		} as AppSettings
	}
}

// Set full settings object
export async function set(settings: AppSettings) {
	try {
		return await invoke('plugin:settings|settings_set', { settings })
	} catch {
		const current = (await invoke('load_settings')) as any
		await invoke('save_settings', {
			settings: {
				...current,
				managed_folder: settings.custom_dir ?? settings.prev_custom_dir ?? current.managed_folder,
			},
		})
		return null
	}
}

export async function cancel_directory_change(): Promise<void> {
	try {
		return await invoke('plugin:settings|cancel_directory_change')
	} catch {
		return
	}
}
