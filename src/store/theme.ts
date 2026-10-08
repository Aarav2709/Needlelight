import { defineStore } from 'pinia'

export const THEME_OPTIONS = ['dark', 'light', 'oled', 'system'] as const

export type ColorTheme = (typeof THEME_OPTIONS)[number]

// the color theme class on the html element and whether blur effects are on
export const useTheming = defineStore('themeStore', {
	state: () => ({
		selectedTheme: 'dark' as ColorTheme,
		advancedRendering: true,
	}),
	actions: {
		setThemeState(newTheme: ColorTheme) {
			if (THEME_OPTIONS.includes(newTheme)) {
				this.selectedTheme = newTheme
			} else {
				console.warn('Selected theme is not present. Check themeOptions.')
			}

			this.setThemeClass()
		},
		setThemeClass() {
			const html = document.documentElement
			for (const theme of THEME_OPTIONS) html.classList.remove(`${theme}-mode`)

			let theme = this.selectedTheme
			if (theme === 'system') {
				theme = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
			}

			html.classList.add(`${theme}-mode`)
		},
	},
})
