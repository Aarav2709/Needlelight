/**
 * Everything a modpack's pages show and do, in one place. ModpackShell provides it; the mod
 * list, Browse and the details panel read from it, so a mod's state and the actions on it are
 * defined once.
 *
 * Dependencies are handled here rather than by the player:
 * - installing or updating a mod downloads what it needs and re-enables anything it needs that
 *   was disabled,
 * - enabling a mod enables what it needs,
 * - disabling or uninstalling a mod other enabled mods need asks first, then disables those too,
 * - "Fix" repairs whatever is still broken (missing, outdated or disabled dependencies).
 */
import { injectNotificationManager } from '@modrinth/ui'
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed, inject, type InjectionKey, provide, type Ref, ref } from 'vue'

import { usePlayModpack } from '@/composables/play'
import { gameInfo } from '@/helpers/games'
import {
	addOutcome,
	analyzePack,
	buildIndex,
	disabledDependenciesDeep,
	displayModName,
	enabledDependentsDeep,
	isEnabled,
	listNames,
	lookup,
	mergeInstallState,
} from '@/helpers/mods'
import { useCatalog } from '@/store/catalog'
import { useGames } from '@/store/games'
import { useModpacks } from '@/store/modpacks'
import { usePreferences } from '@/store/preferences'

export type ConfirmRequest = {
	title: string
	description: string
	proceedLabel: string
	danger: boolean
	onProceed: () => Promise<unknown> | void
}

function plural(count: number, word: string) {
	return `${count} ${word}${count === 1 ? '' : 's'}`
}

function createModpackContext(path: Ref<string>) {
	const games = useGames()
	const modpacks = useModpacks()
	const catalog = useCatalog()
	const { prefs } = usePreferences()
	const { play: playModpack } = usePlayModpack()
	const { handleError, addNotification } = injectNotificationManager()

	const modpack = computed(() => modpacks.byPath.get(path.value) ?? null)
	const game = computed(() => modpack.value?.game ?? games.activeGame)
	const gameName = computed(() => gameInfo(game.value).name)

	const catalogState = computed(() => catalog.entries[game.value] ?? null)
	const catalogItems = computed(() => catalogState.value?.items ?? null)
	const db = computed(() => modpacks.installed[path.value] ?? null)
	const installedError = computed(() => modpacks.installedErrors[path.value] ?? null)
	const entries = computed(() => mergeInstallState(catalogItems.value, db.value))
	const index = computed(() => buildIndex(entries.value))
	const health = computed(() => analyzePack(entries.value, index.value))
	const installed = computed(() => health.value.installed)
	const activity = computed(() => modpacks.activity[path.value] ?? null)

	const isBusy = (name: string) => modpacks.isBusy(path.value, name)
	const progressFor = (name: string) =>
		isBusy(name) ? (modpacks.progress.get(name) ?? undefined) : undefined
	const label = (name: string) => displayModName(name, game.value)

	/** A confirmation the page should show (it owns the dialog). */
	const confirmRequest = ref<ConfirmRequest | null>(null)
	function confirm(request: ConfirmRequest) {
		confirmRequest.value = request
	}

	function success(title: string, text?: string) {
		addNotification({ title, text, type: 'success' })
	}

	async function run(task: () => Promise<unknown>): Promise<boolean> {
		try {
			await task()
			return true
		} catch (err) {
			handleError(err as Error)
			return false
		}
	}

	/** Enable whatever `names` need that is installed but disabled. */
	async function enableNeeded(names: string[]) {
		const needed = disabledDependenciesDeep(names, index.value).map((m) => m.name)
		if (needed.length) await modpacks.setEnabled(path.value, needed, true)
		return needed
	}

	async function install(name: string) {
		const outcome = addOutcome(name, index.value)
		if (!(await run(() => modpacks.install(path.value, [name])))) return
		await run(() => enableNeeded([name]))
		const extra = outcome.added.length
		success(
			`Installed ${label(name)}`,
			extra ? `Also installed ${plural(extra, 'mod')} it needs.` : undefined,
		)
	}

	async function update(names: string[]) {
		if (!names.length) return
		if (!(await run(() => modpacks.install(path.value, names)))) return
		await run(() => enableNeeded(names))
		success(names.length === 1 ? `Updated ${label(names[0])}` : `Updated ${plural(names.length, 'mod')}`)
	}

	const updateAll = () => update(health.value.updates.map((m) => m.name))

	/** Repair everything the data says is broken and Needlelight can fix. */
	async function fix() {
		const h = health.value
		const toInstall = [...new Set([...h.installableMissing, ...h.outdatedRequired])]
		if (toInstall.length && !(await run(() => modpacks.install(path.value, toInstall)))) return
		const enabledMods = installed.value.filter(isEnabled).map((m) => m.name)
		if (!(await run(() => enableNeeded(enabledMods)))) return
		const left = health.value.unavailable
		success(
			'Dependencies fixed',
			left.length
				? `${listNames(left.map(label))} can't be downloaded, so the mods that need ${left.length === 1 ? 'it' : 'them'} still won't load.`
				: undefined,
		)
	}

	async function setEnabled(name: string, enable: boolean) {
		if (enable) {
			const needed = disabledDependenciesDeep([name], index.value).map((m) => m.name)
			if (!(await run(() => modpacks.setEnabled(path.value, [...needed, name], true)))) return
			if (needed.length) {
				success(`Enabled ${label(name)}`, `Also enabled ${listNames(needed.map(label))}, which it needs.`)
			}
			return
		}
		const dependents = enabledDependentsDeep(name, installed.value)
		if (!dependents.length) {
			await run(() => modpacks.setEnabled(path.value, [name], false))
			return
		}
		const names = dependents.map((m) => label(m.name))
		confirm({
			title: `Disable ${label(name)}?`,
			description: `${listNames(names)} ${names.length === 1 ? 'needs' : 'need'} it, so ${names.length === 1 ? 'it' : 'they'} will be disabled too.`,
			proceedLabel: 'Disable',
			danger: false,
			onProceed: () =>
				run(() =>
					modpacks.setEnabled(path.value, [...dependents.map((m) => m.name), name], false),
				),
		})
	}

	async function uninstallNow(name: string, disable: string[] = []) {
		if (disable.length && !(await run(() => modpacks.setEnabled(path.value, disable, false)))) return
		if (!(await run(() => modpacks.uninstall(path.value, name)))) return
		success(`Uninstalled ${label(name)}`)
	}

	function uninstall(name: string) {
		const dependents = enabledDependentsDeep(name, installed.value)
		if (dependents.length) {
			const names = dependents.map((m) => label(m.name))
			confirm({
				title: `Uninstall ${label(name)}?`,
				description: `${listNames(names)} ${names.length === 1 ? 'needs' : 'need'} it, so ${names.length === 1 ? 'it' : 'they'} will be disabled. You can install ${label(name)} again from Browse.`,
				proceedLabel: 'Uninstall',
				danger: true,
				onProceed: () => uninstallNow(name, dependents.map((m) => m.name)),
			})
		} else if (prefs.confirmModRemoval) {
			confirm({
				title: `Uninstall ${label(name)}?`,
				description: `It will be removed from this modpack. You can install it again from Browse.`,
				proceedLabel: 'Uninstall',
				danger: true,
				onProceed: () => uninstallNow(name),
			})
		} else {
			void uninstallNow(name)
		}
	}

	async function play() {
		if (modpack.value) {
			await playModpack(
				modpack.value,
				health.value.updates.map((m) => m.name),
			)
		}
	}

	// ─── Several mods at once (the selection bar) ────────────────────────────

	async function setEnabledMany(names: string[], enable: boolean) {
		if (!names.length) return
		if (names.length === 1) return setEnabled(names[0], enable)
		if (enable) {
			const needed = disabledDependenciesDeep(names, index.value)
				.map((m) => m.name)
				.filter((n) => !names.includes(n))
			if (!(await run(() => modpacks.setEnabled(path.value, [...needed, ...names], true)))) return
			success(
				`Enabled ${plural(names.length, 'mod')}`,
				needed.length ? `Also enabled ${listNames(needed.map(label))}, which they need.` : undefined,
			)
			return
		}
		const chosen = new Set(names.map((n) => n.toLowerCase()))
		const dependents = [
			...new Map(
				names
					.flatMap((n) => enabledDependentsDeep(n, installed.value))
					.filter((m) => !chosen.has(m.name.toLowerCase()))
					.map((m) => [m.name, m]),
			).values(),
		]
		const disableNow = () =>
			run(async () => {
				await modpacks.setEnabled(path.value, [...dependents.map((m) => m.name), ...names], false)
				success(`Disabled ${plural(names.length + dependents.length, 'mod')}`)
			})
		if (!dependents.length) {
			await disableNow()
			return
		}
		const others = dependents.map((m) => label(m.name))
		confirm({
			title: `Disable ${plural(names.length, 'mod')}?`,
			description: `${listNames(others)} ${others.length === 1 ? 'needs' : 'need'} them, so ${others.length === 1 ? 'it' : 'they'} will be disabled too.`,
			proceedLabel: 'Disable',
			danger: false,
			onProceed: disableNow,
		})
	}

	function uninstallMany(names: string[]) {
		if (!names.length) return
		if (names.length === 1) return uninstall(names[0])
		const chosen = new Set(names.map((n) => n.toLowerCase()))
		const dependents = [
			...new Map(
				names
					.flatMap((n) => enabledDependentsDeep(n, installed.value))
					.filter((m) => !chosen.has(m.name.toLowerCase()))
					.map((m) => [m.name, m]),
			).values(),
		]
		const others = dependents.map((m) => label(m.name))
		confirm({
			title: `Uninstall ${plural(names.length, 'mod')}?`,
			description:
				`${listNames(names.map(label))} will be removed from this modpack.` +
				(others.length
					? ` ${listNames(others)} ${others.length === 1 ? 'needs' : 'need'} them, so ${others.length === 1 ? 'it' : 'they'} will be disabled.`
					: ''),
			proceedLabel: 'Uninstall',
			danger: true,
			onProceed: async () => {
				if (
					others.length &&
					!(await run(() => modpacks.setEnabled(path.value, dependents.map((m) => m.name), false)))
				)
					return
				for (const name of names) {
					if (!(await run(() => modpacks.uninstall(path.value, name)))) return
				}
				success(`Uninstalled ${plural(names.length, 'mod')}`)
			},
		})
	}

	async function openLink(url: string) {
		await run(() => openUrl(url))
	}

	/** The catalog entry (or installed-only entry) for a mod name. */
	const find = (name: string) => lookup(index.value, name)

	return {
		path,
		modpack,
		game,
		gameName,
		catalogState,
		catalogItems,
		db,
		installedError,
		entries,
		index,
		health,
		installed,
		activity,
		isBusy,
		progressFor,
		label,
		find,
		confirmRequest,
		install,
		update,
		updateAll,
		fix,
		setEnabled,
		setEnabledMany,
		uninstall,
		uninstallMany,
		play,
		openLink,
		reloadCatalog: () => catalog.load(game.value, true),
	}
}

export type ModpackContext = ReturnType<typeof createModpackContext>

const KEY: InjectionKey<ModpackContext> = Symbol('modpack')

export function provideModpackContext(path: Ref<string>) {
	const context = createModpackContext(path)
	provide(KEY, context)
	return context
}

export function useModpackContext(): ModpackContext {
	const context = inject(KEY)
	if (!context) throw new Error('useModpackContext() needs a modpack page above it')
	return context
}
