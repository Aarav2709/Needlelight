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
		// NOTE (Needlelight): this was '../../packages/**/*.{js,vue,ts}', copied verbatim from
		// Modrinth's own tailwind.config.ts, which lives at apps/app-frontend/ - two directories
		// below their monorepo root, so '../../packages/' correctly reaches their packages/ from
		// there. Needlelight's tailwind.config.ts is at the repo root, with packages/ as a direct
		// child, so the old glob resolved to a nonexistent path outside the repo entirely and
		// packages/ui/utils/assets were never scanned for Tailwind classes at all. Most classes
		// still worked by coincidence (Tailwind generates by class name globally, and many classes
		// used in packages/ui happen to also appear somewhere in src/), but classes unique to
		// packages/ui (e.g. TabbedModal's layout grid) were never generated.
		'./packages/**/*.{js,vue,ts}',
		'!./packages/**/node_modules/**',
	],
	presets: [preset],
}

export default config
