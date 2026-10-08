// merges a game's catalog with a modpack's installed mods and works out dependencies from real catalog data only
import type { GameKey } from '@/helpers/games'
import type { InstalledDb, ModItem, ModState } from '@/helpers/types'

export type ModEntry = ModItem & {
	// false for installed mods the catalog doesn't know, like manual installs or an offline catalog
	inCatalog: boolean
}

export type ModIndex = {
	byName: Map<string, ModEntry>
	byLowerName: Map<string, ModEntry>
}

function stateFor(name: string, latest: string, db: InstalledDb | null): ModState {
	const installed = db?.mods[name]
	if (installed) {
		return {
			kind: 'installed',
			enabled: installed.enabled,
			pinned: installed.pinned,
			version: installed.version,
			updated: installed.version === latest,
		}
	}
	const local = db?.not_in_modlinks_mods[name]
	if (local) return { kind: 'not_in_modlinks', ...local }
	return { kind: 'not_installed', installing: false }
}

function localEntry(name: string, state: ModState, version: string): ModEntry {
	return {
		name,
		description: '',
		version,
		dependencies: [],
		link: '',
		sha256: '',
		repository: '',
		issues: '',
		tags: [],
		integrations: [],
		authors: [],
		state,
		inCatalog: false,
	}
}

// catalog items with this modpack's install state, plus installed mods the catalog lacks
export function mergeInstallState(catalog: ModItem[] | null, db: InstalledDb | null): ModEntry[] {
	const entries: ModEntry[] = []
	const seen = new Set<string>()
	for (const item of catalog ?? []) {
		entries.push({ ...item, state: stateFor(item.name, item.version, db), inCatalog: true })
		seen.add(item.name)
	}
	if (db) {
		for (const [name, st] of Object.entries(db.mods)) {
			if (seen.has(name)) continue
			seen.add(name)
			entries.push(
				localEntry(
					name,
					{ kind: 'installed', enabled: st.enabled, pinned: st.pinned, version: st.version, updated: true },
					st.version,
				),
			)
		}
		for (const [name, st] of Object.entries(db.not_in_modlinks_mods)) {
			if (seen.has(name) || !st.installed) continue
			seen.add(name)
			entries.push(localEntry(name, { kind: 'not_in_modlinks', ...st }, ''))
		}
	}
	return entries
}

export function buildIndex(entries: ModEntry[]): ModIndex {
	const byName = new Map<string, ModEntry>()
	const byLowerName = new Map<string, ModEntry>()
	for (const entry of entries) {
		byName.set(entry.name, entry)
		byLowerName.set(entry.name.toLowerCase(), entry)
	}
	return { byName, byLowerName }
}

export function lookup(index: ModIndex, name: string): ModEntry | null {
	return index.byName.get(name) ?? index.byLowerName.get(name.toLowerCase()) ?? null
}

// state helpers

export function isInstalled(mod: ModItem): boolean {
	return (
		mod.state.kind === 'installed' || (mod.state.kind === 'not_in_modlinks' && mod.state.installed)
	)
}

export function isEnabled(mod: ModItem): boolean {
	return isInstalled(mod) && 'enabled' in mod.state && mod.state.enabled !== false
}

export function needsUpdate(mod: ModItem): boolean {
	return mod.state.kind === 'installed' && mod.state.updated === false
}

// on disk and in the catalog but installed outside needlelight, so its version is unknown
export function isUntracked(mod: ModEntry): boolean {
	return mod.inCatalog && mod.state.kind === 'not_in_modlinks'
}

// installed version, when known
export function installedVersion(mod: ModItem): string | null {
	return mod.state.kind === 'installed' ? mod.state.version || null : null
}

export function formatVersion(version?: string | null): string {
	if (!version) return ''
	return /^v/i.test(version) ? version : `v${version}`
}

// compares dotted versions numerically so 1.10.0 is newer than 1.9.2
export function compareVersions(a: string, b: string): number {
	const left = a.replace(/^v/i, '').split(/[.+-]/)
	const right = b.replace(/^v/i, '').split(/[.+-]/)
	for (let i = 0; i < Math.max(left.length, right.length); i++) {
		const x = left[i] ?? '0'
		const y = right[i] ?? '0'
		const nx = Number(x)
		const ny = Number(y)
		if (!Number.isNaN(nx) && !Number.isNaN(ny)) {
			if (nx !== ny) return nx - ny
			continue
		}
		const text = x.localeCompare(y)
		if (text) return text
	}
	return 0
}

// the mod loader itself (the bepinex pack), which every silksong modpack already ships
export function isLoaderDependency(name: string): boolean {
	return /bepinexpack/i.test(name)
}

// thunderstore names start with the author, so silksong shows just the mod name with spaces
export function displayModName(name: string, game: GameKey): string {
	if (game !== 'silksong') return name
	const parts = name.split('-')
	const withoutAuthor = parts.length <= 1 ? name : parts.slice(1).join('-')
	return withoutAuthor.replace(/_/g, ' ')
}

// author for display, from the catalog's authors or the thunderstore owner prefix
export function authorLine(mod: ModItem, game: GameKey): string {
	if (mod.authors.length) return mod.authors.join(', ')
	if (game === 'silksong' && mod.name.includes('-')) return mod.name.split('-')[0]
	return ''
}

// short author line for rows, like serena, serena and flib, or serena and 8 others
export function shortAuthorLine(mod: ModItem, game: GameKey): string {
	if (mod.authors.length > 2) return `${mod.authors[0]} and ${mod.authors.length - 1} others`
	if (mod.authors.length) return listNames(mod.authors)
	return authorLine(mod, game)
}

// joins names as a, a and b, or a, b and c
export function listNames(names: string[]): string {
	if (names.length <= 1) return names[0] ?? ''
	return `${names.slice(0, -1).join(', ')} and ${names[names.length - 1]}`
}

const compactNumber = new Intl.NumberFormat(undefined, {
	notation: 'compact',
	maximumFractionDigits: 1,
})
export function formatCount(value?: number | null): string {
	return value == null ? '' : compactNumber.format(value)
}

// links

// the repository url for any url inside a github repository
function githubRepo(url?: string | null): string | null {
	const match = url?.match(/^https?:\/\/github\.com\/([^/#?]+)\/([^/#?]+)/i)
	return match ? `https://github.com/${match[1]}/${match[2].replace(/\.git$/i, '')}` : null
}

const sameUrl = (a: string, b: string) => a.replace(/\/+$/, '') === b.replace(/\/+$/, '')

export type ProjectLinks = {
	// where the project is published, its thunderstore page or repository
	page: string | null
	// the project's own website when it differs from the page
	website: string | null
	// where to report problems, when the project has somewhere for that
	issues: string | null
}

export function projectLinks(mod: ModItem): ProjectLinks {
	const page = mod.repository || null
	const website = mod.homepage && (!page || !sameUrl(mod.homepage, page)) ? mod.homepage : null
	const repo = githubRepo(mod.repository) ?? githubRepo(mod.homepage)
	return { page, website, issues: mod.issues || (repo ? `${repo}/issues` : null) }
}

// relationships

export type DependencyStatus =
	// in the modpack and enabled
	| 'enabled'
	// in the modpack but disabled
	| 'disabled'
	// in the modpack but older than the version the mod asks for
	| 'outdated'
	// not in the modpack and can be downloaded
	| 'missing'
	// not in the modpack and can't be downloaded
	| 'unavailable'
	// the mod loader, which every modpack includes and is never shown
	| 'loader'

export type DependencyRef = {
	name: string
	status: DependencyStatus
	mod: ModEntry | null
	// minimum version the mod asks for, when the catalog says
	minVersion: string | null
	optional: boolean
}

function resolve(
	names: string[],
	index: ModIndex,
	optional: boolean,
	versions?: Record<string, string>,
): DependencyRef[] {
	const out: DependencyRef[] = []
	const seen = new Set<string>()
	for (const raw of names) {
		const name = raw.trim()
		if (!name || seen.has(name.toLowerCase())) continue
		seen.add(name.toLowerCase())
		const minVersion = versions?.[name] ?? null
		if (isLoaderDependency(name)) {
			out.push({ name, status: 'loader', mod: null, minVersion, optional })
			continue
		}
		const mod = lookup(index, name)
		let status: DependencyStatus
		if (mod && isInstalled(mod)) {
			const current = installedVersion(mod)
			if (minVersion && current && compareVersions(current, minVersion) < 0) status = 'outdated'
			else status = isEnabled(mod) ? 'enabled' : 'disabled'
		} else if (mod?.inCatalog) status = 'missing'
		else status = 'unavailable'
		out.push({ name, status, mod, minVersion, optional })
	}
	return out
}

// mods this one needs, excluding the mod loader
export function dependenciesOf(mod: ModItem, index: ModIndex): DependencyRef[] {
	return resolve(mod.dependencies, index, false, mod.dependency_versions).filter(
		(dep) => dep.status !== 'loader',
	)
}

// optional companion mods, which hollow knight's modlinks lists as integrations
export function companionsOf(mod: ModItem, index: ModIndex): DependencyRef[] {
	return resolve(mod.integrations, index, true).filter((dep) => dep.status !== 'loader')
}

// installed mods in the modpack that need this one directly
export function dependentsOf(name: string, installed: ModEntry[]): ModEntry[] {
	const lower = name.toLowerCase()
	return installed.filter((mod) =>
		mod.dependencies.some((dep) => dep.trim().toLowerCase() === lower),
	)
}

// enabled mods that need this one, directly or through other mods
export function enabledDependentsDeep(name: string, installed: ModEntry[]): ModEntry[] {
	const found = new Map<string, ModEntry>()
	const queue = [name]
	while (queue.length) {
		const current = queue.shift()!
		for (const mod of dependentsOf(current, installed)) {
			if (!isEnabled(mod) || found.has(mod.name) || mod.name === name) continue
			found.set(mod.name, mod)
			queue.push(mod.name)
		}
	}
	return [...found.values()]
}

// installed but disabled mods that these need, directly or through other mods
export function disabledDependenciesDeep(names: string[], index: ModIndex): ModEntry[] {
	const found = new Map<string, ModEntry>()
	const visited = new Set(names.map((n) => n.toLowerCase()))
	const stack = names.flatMap((n) => lookup(index, n)?.dependencies ?? [])
	while (stack.length) {
		const dep = stack.pop()!.trim()
		if (!dep || isLoaderDependency(dep) || visited.has(dep.toLowerCase())) continue
		visited.add(dep.toLowerCase())
		const mod = lookup(index, dep)
		if (!mod || !isInstalled(mod)) continue
		if (!isEnabled(mod)) found.set(mod.name, mod)
		stack.push(...mod.dependencies)
	}
	return [...found.values()]
}

// what adding a mod does, like the installer: missing dependencies download, outdated ones update, disabled ones are enabled again
export type AddOutcome = {
	added: ModEntry[]
	updated: ModEntry[]
	enabled: ModEntry[]
	// direct dependencies already in the modpack as they are
	present: ModEntry[]
	// dependencies needlelight can't download
	unavailable: string[]
}

export function addOutcome(name: string, index: ModIndex): AddOutcome {
	const outcome: AddOutcome = { added: [], updated: [], enabled: [], present: [], unavailable: [] }
	const direct = new Set(
		(lookup(index, name)?.dependencies ?? []).map((d) => d.trim().toLowerCase()),
	)
	const visited = new Set([name.toLowerCase()])
	const stack = [...(lookup(index, name)?.dependencies ?? [])]
	while (stack.length) {
		const dep = stack.pop()!.trim()
		if (!dep || isLoaderDependency(dep) || visited.has(dep.toLowerCase())) continue
		visited.add(dep.toLowerCase())
		const mod = lookup(index, dep)
		if (!mod || (!mod.inCatalog && !isInstalled(mod))) {
			outcome.unavailable.push(dep)
			continue
		}
		if (!isInstalled(mod)) {
			outcome.added.push(mod)
			stack.push(...mod.dependencies)
		} else if (needsUpdate(mod) || isUntracked(mod)) {
			outcome.updated.push(mod)
			stack.push(...mod.dependencies)
		} else if (direct.has(dep.toLowerCase())) {
			outcome.present.push(mod)
		}
	}
	outcome.enabled = disabledDependenciesDeep([name], index)
	return outcome
}

// modpack health

export type ModIssue = {
	kind: 'missing' | 'unavailable' | 'disabled' | 'outdated'
	dependency: string
}

export type PackHealth = {
	installed: ModEntry[]
	enabledCount: number
	updates: ModEntry[]
	// problems per installed mod name
	issues: Map<string, ModIssue[]>
	// missing dependencies that can be downloaded
	installableMissing: string[]
	// dependencies older than a mod needs, which updating fixes
	outdatedRequired: string[]
	// dependencies that aren't in the modpack and can't be downloaded
	unavailable: string[]
	// disabled mods that enabled mods need
	disabledRequired: string[]
	// problems the fix action can solve
	fixable: number
}

// issues only count for enabled mods because a disabled mod isn't loaded
export function modIssues(mod: ModItem, index: ModIndex): ModIssue[] {
	if (!isEnabled(mod)) return []
	const issues: ModIssue[] = []
	for (const dep of dependenciesOf(mod, index)) {
		if (dep.status === 'enabled') continue
		issues.push({ kind: dep.status as ModIssue['kind'], dependency: dep.mod?.name ?? dep.name })
	}
	return issues
}

export function analyzePack(entries: ModEntry[], index: ModIndex): PackHealth {
	const installed = entries.filter(isInstalled)
	const issues = new Map<string, ModIssue[]>()
	const missing = new Set<string>()
	const outdated = new Set<string>()
	const unavailable = new Set<string>()
	const disabled = new Set<string>()
	for (const mod of installed) {
		const found = modIssues(mod, index)
		if (!found.length) continue
		issues.set(mod.name, found)
		for (const issue of found) {
			if (issue.kind === 'missing') missing.add(issue.dependency)
			else if (issue.kind === 'outdated') outdated.add(issue.dependency)
			else if (issue.kind === 'unavailable') unavailable.add(issue.dependency)
			else disabled.add(issue.dependency)
		}
	}
	return {
		installed,
		enabledCount: installed.filter(isEnabled).length,
		updates: installed.filter(needsUpdate),
		issues,
		installableMissing: [...missing],
		outdatedRequired: [...outdated],
		unavailable: [...unavailable],
		disabledRequired: [...disabled],
		fixable: missing.size + outdated.size + disabled.size,
	}
}
