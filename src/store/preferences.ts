/**
 * UI preferences. These only affect the frontend, so they persist in the webview's
 * localStorage (the backend's settings file holds where each game is installed).
 */
import { defineStore } from 'pinia'
import { reactive, watch } from 'vue'

import type { GameKey } from '@/helpers/games'
import { useTheming } from '@/store/theme'
import type { ColorTheme } from '@/store/theme.ts'

const STORAGE_KEY = 'needlelight.preferences.v1'
export const ONBOARDED_KEY = 'needlelight.onboarded'

export type Density = 'comfortable' | 'compact'

export type Preferences = {
	theme: ColorTheme
	density: Density
	reduceMotion: boolean
	/** Blur and depth effects. */
	advancedRendering: boolean
	confirmModRemoval: boolean
	minimizeOnLaunch: boolean
	/** Install available mod updates before a modpack launches. */
	updateBeforePlay: boolean
	/** Last opened modpack per game, so the Modpacks page reopens where you left off. */
	lastModpack: Partial<Record<GameKey, string>>
	/** The player's order of modpacks in the sidebar (folder paths). Empty until they rearrange. */
	modpackOrder: string[]
}

export const DEFAULT_PREFERENCES: Preferences = {
	theme: 'dark',
	density: 'comfortable',
	reduceMotion: false,
	advancedRendering: true,
	confirmModRemoval: true,
	minimizeOnLaunch: false,
	updateBeforePlay: false,
	lastModpack: {},
	modpackOrder: [],
}

function read(): Partial<Preferences> {
	try {
		const raw = localStorage.getItem(STORAGE_KEY)
		if (!raw) return {}
		const stored = JSON.parse(raw) as Record<string, unknown>
		// Keep only known keys, so preferences removed in later versions don't linger.
		return Object.fromEntries(
			Object.entries(stored).filter(([key]) => key in DEFAULT_PREFERENCES),
		) as Partial<Preferences>
	} catch {
		return {}
	}
}

function write(value: Preferences) {
	try {
		localStorage.setItem(STORAGE_KEY, JSON.stringify(value))
	} catch {
		/* storage unavailable: preferences just won't survive a restart */
	}
}

export const usePreferences = defineStore('preferences', () => {
	const prefs = reactive<Preferences>({ ...DEFAULT_PREFERENCES, ...read() })
	const theming = useTheming()
	const systemDark = window.matchMedia('(prefers-color-scheme: dark)')

	/** Push preferences into the DOM and the theme store. */
	function apply() {
		const html = document.documentElement
		theming.setThemeState(prefs.theme)
		theming.advancedRendering = prefs.advancedRendering
		html.classList.toggle('density-compact', prefs.density === 'compact')
		html.classList.toggle('reduce-motion', prefs.reduceMotion)
	}

	systemDark.addEventListener('change', () => {
		if (prefs.theme === 'system') apply()
	})

	watch(
		prefs,
		(value) => {
			write(value)
			apply()
		},
		{ deep: true },
	)

	/** Back to defaults, keeping which modpack each game had open. */
	function reset() {
		const lastModpack = prefs.lastModpack
		Object.assign(prefs, { ...DEFAULT_PREFERENCES, lastModpack })
	}

	return { prefs, apply, reset }
})

export function isOnboarded(): boolean {
	try {
		return localStorage.getItem(ONBOARDED_KEY) === '1'
	} catch {
		return true
	}
}

export function setOnboarded(value: boolean) {
	try {
		if (value) localStorage.setItem(ONBOARDED_KEY, '1')
		else localStorage.removeItem(ONBOARDED_KEY)
	} catch {
		/* storage unavailable */
	}
}
