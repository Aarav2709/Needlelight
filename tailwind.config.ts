import preset from '@modrinth/tooling-config/tailwind/tailwind-preset.ts'
import type { Config } from 'tailwindcss'

const config: Config = {
	content: [
		'./src/components/**/*.{js,vue,ts}',
		'./src/layouts/**/*.vue',
		'./src/pages/**/*.vue',
		'./src/plugins/**/*.{js,ts}',
		'./src/App.vue',
		'./src/error.vue',
		// scan the local packages folder too, the old modrinth glob pointed outside this repo so classes unique to the packages were never generated
		'./packages/**/*.{js,vue,ts}',
		'!./packages/**/node_modules/**',
	],
	presets: [preset],
}

export default config
