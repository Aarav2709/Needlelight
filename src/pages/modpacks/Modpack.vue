<script setup lang="ts">
// a modpack's page, its header and the mods installed in it
import {
	CheckCircleIcon,
	CircleAlertIcon,
	ClockIcon,
	CopyIcon,
	EditIcon,
	EmptyIllustration,
	FilterIcon,
	MoreVerticalIcon,
	NoConnectionIllustration,
	NoSearchResultIllustration,
	PackageIcon,
	PlayIcon,
	PlusIcon,
	RefreshCwIcon,
	SearchIcon,
	SortAscIcon,
	SpinnerIcon,
	TrashIcon,
	TriangleAlertIcon,
	UndoIcon,
	UpdatedIcon,
	WrenchIcon,
	XCircleIcon,
	XIcon,
} from '@modrinth/assets'
import {
	Button,
	type ButtonMenuOption,
	Checkbox,
	type ComboboxOption,
	ConfirmModal,
	FloatingActionBar,
	injectNotificationManager,
	TeleportOverflowMenu,
} from '@modrinth/ui'
import { useStorage } from '@vueuse/core'
import { computed, ref, watch } from 'vue'
import { useRouter } from 'vue-router'

import ModPanel from '@/components/modpacks/ModPanel.vue'
import ModRow from '@/components/modpacks/ModRow.vue'
import ModpackEditorModal from '@/components/ui/modal/ModpackEditorModal.vue'
import ModpackArt from '@/components/ui/ModpackArt.vue'
import SelectMenu from '@/components/ui/SelectMenu.vue'
import { useModpackContext } from '@/composables/modpack'
import { useShortcut } from '@/composables/shortcuts'
import { modpackArt } from '@/helpers/art'
import { modpackRoute, playedLabel } from '@/helpers/modpacks'
import { isEnabled, listNames, type ModEntry, needsUpdate } from '@/helpers/mods'
import { useGames } from '@/store/games'
import { useModpacks } from '@/store/modpacks'
import { useUi } from '@/store/ui'

type Show = 'all' | 'enabled' | 'disabled' | 'updates' | 'issues'
type Sort = 'name' | 'name-desc' | 'enabled' | 'disabled'

const router = useRouter()
const games = useGames()
const modpacks = useModpacks()
const ui = useUi()
const { handleError, addNotification } = injectNotificationManager()

const ctx = useModpackContext()
const { path, modpack, game, gameName, health, installed, activity, catalogState, db } = ctx

const art = computed(() => (modpack.value ? modpackArt(modpack.value.path, modpack.value.game) : null))
const isActiveInGame = computed(() => game.value === 'hollow_knight' && modpacks.activeHk === path.value)
const gameMissing = computed(() => games.found[game.value] === false)
const browseRoute = computed(() => modpackRoute({ path: path.value }, 'browse'))

// details panel

const selectedName = ref<string | null>(null)
// clicking the open mod's row again closes its panel
function toggleMod(name: string) {
	selectedName.value = selectedName.value === name ? null : name
}
// a manually added mod disappears once uninstalled, so close its panel
watch(
	() => selectedName.value && !ctx.find(selectedName.value),
	(gone) => {
		if (gone) selectedName.value = null
	},
)

// search, filter, sort

const search = ref('')
const show = ref<Show>('all')
const sort = useStorage<Sort>('nl-mods-sort', 'name')

const showOptions = computed<ComboboxOption<Show>[]>(() => {
	const h = health.value
	const disabled = h.installed.length - h.enabledCount
	const options: ComboboxOption<Show>[] = [
		{ value: 'all', label: 'All mods', subLabel: String(h.installed.length) },
		{ value: 'enabled', label: 'Enabled', subLabel: String(h.enabledCount) },
		{ value: 'disabled', label: 'Disabled', subLabel: String(disabled) },
	]
	if (h.updates.length) options.push({ value: 'updates', label: 'Updates available', subLabel: String(h.updates.length) })
	if (h.issues.size) options.push({ value: 'issues', label: "Won't load", subLabel: String(h.issues.size) })
	return options
})
watch(showOptions, (options) => {
	if (!options.some((o) => o.value === show.value)) show.value = 'all'
})

const sortOptions: ComboboxOption<Sort>[] = [
	{ value: 'name', label: 'Name (A–Z)' },
	{ value: 'name-desc', label: 'Name (Z–A)' },
	{ value: 'enabled', label: 'Enabled first' },
	{ value: 'disabled', label: 'Disabled first' },
]

function matches(mod: ModEntry, query: string) {
	return (
		mod.name.toLowerCase().includes(query) ||
		ctx.label(mod.name).toLowerCase().includes(query) ||
		mod.description.toLowerCase().includes(query) ||
		mod.authors.some((a) => a.toLowerCase().includes(query)) ||
		mod.tags.some((t) => t.toLowerCase().includes(query))
	)
}

const byName = (a: ModEntry, b: ModEntry) =>
	ctx.label(a.name).localeCompare(ctx.label(b.name), undefined, { sensitivity: 'base' })

const visible = computed(() => {
	const query = search.value.trim().toLowerCase()
	let list = installed.value
	if (show.value === 'enabled') list = list.filter(isEnabled)
	else if (show.value === 'disabled') list = list.filter((m) => !isEnabled(m))
	else if (show.value === 'updates') list = list.filter(needsUpdate)
	else if (show.value === 'issues') list = list.filter((m) => health.value.issues.has(m.name))
	if (query) list = list.filter((m) => matches(m, query))
	const sorted = [...list].sort(byName)
	if (sort.value === 'name-desc') sorted.reverse()
	else if (sort.value === 'enabled') sorted.sort((a, b) => Number(isEnabled(b)) - Number(isEnabled(a)))
	else if (sort.value === 'disabled') sorted.sort((a, b) => Number(isEnabled(a)) - Number(isEnabled(b)))
	return sorted
})

function clearFilters() {
	search.value = ''
	show.value = 'all'
}

// selection

const checked = ref(new Set<string>())
let anchor: string | null = null

const checkedMods = computed(() => installed.value.filter((m) => checked.value.has(m.name)))
const allVisibleChecked = computed(() => visible.value.length > 0 && visible.value.every((m) => checked.value.has(m.name)))
const someVisibleChecked = computed(() => visible.value.some((m) => checked.value.has(m.name)))

function setChecked(names: string[], value: boolean) {
	const next = new Set(checked.value)
	for (const name of names) {
		if (value) next.add(name)
		else next.delete(name)
	}
	checked.value = next
}

// shift click checks everything between the last clicked row and this one
function onCheck(name: string, event?: MouseEvent) {
	const value = !checked.value.has(name)
	if (event?.shiftKey && anchor) {
		const names = visible.value.map((m) => m.name)
		const [from, to] = [names.indexOf(anchor), names.indexOf(name)].sort((a, b) => a - b)
		if (from >= 0) {
			setChecked(names.slice(from, to + 1), value)
			anchor = name
			return
		}
	}
	setChecked([name], value)
	anchor = name
}

function toggleAll() {
	setChecked(
		visible.value.map((m) => m.name),
		!allVisibleChecked.value,
	)
}

function clearChecked() {
	checked.value = new Set()
	anchor = null
}

// forget mods that were uninstalled
watch(installed, (list) => {
	const names = new Set(list.map((m) => m.name))
	if ([...checked.value].some((n) => !names.has(n))) {
		checked.value = new Set([...checked.value].filter((n) => names.has(n)))
	}
})

const checkedNames = computed(() => checkedMods.value.map((m) => m.name))
const checkedUpdates = computed(() => checkedMods.value.filter(needsUpdate).map((m) => m.name))
const anyCheckedBusy = computed(() => checkedNames.value.some((n) => ctx.isBusy(n)))

async function bulkEnable(value: boolean) {
	await ctx.setEnabledMany(
		checkedMods.value.filter((m) => isEnabled(m) !== value).map((m) => m.name),
		value,
	)
}

// keyboard

const searchInput = ref<HTMLInputElement | null>(null)
const table = ref<HTMLElement | null>(null)

useShortcut(['mod+f', '/'], () => searchInput.value?.focus())
useShortcut('mod+b', () => void router.push(browseRoute.value))
useShortcut('mod+a', () => setChecked(visible.value.map((m) => m.name), true), {
	when: () => visible.value.length > 0,
})
useShortcut('delete', () => ctx.uninstallMany(checkedNames.value), {
	when: () => checkedNames.value.length > 0 && !anyCheckedBusy.value,
})
useShortcut(
	'escape',
	(event) => {
		if (selectedName.value) selectedName.value = null
		else if (checked.value.size) clearChecked()
		else if (search.value && event.target === searchInput.value) search.value = ''
	},
	{ inInputs: true },
)

// up and down move between rows, and an open details panel follows
function onTableKeydown(event: KeyboardEvent) {
	if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return
	const rows = [...(table.value?.querySelectorAll<HTMLElement>('.mod-row') ?? [])]
	const current = rows.indexOf(document.activeElement as HTMLElement)
	const next = rows[current < 0 ? 0 : current + (event.key === 'ArrowDown' ? 1 : -1)]
	if (!next) return
	event.preventDefault()
	next.focus()
	const index = rows.indexOf(next)
	if (selectedName.value && visible.value[index]) selectedName.value = visible.value[index].name
}

// modpack actions

const editor = ref<InstanceType<typeof ModpackEditorModal> | null>(null)
const deleteConfirm = ref<InstanceType<typeof ConfirmModal> | null>(null)

async function duplicate() {
	try {
		const copy = await modpacks.duplicate(path.value)
		addNotification({ title: `Created ${copy.name}`, type: 'success' })
		await router.push(modpackRoute(copy))
	} catch (err) {
		handleError(err as Error)
	}
}

async function deletePack() {
	const name = modpack.value?.name
	try {
		await modpacks.remove(path.value)
		addNotification({ title: `Deleted ${name}`, type: 'success' })
		await router.replace('/modpacks')
	} catch (err) {
		handleError(err as Error)
	}
}

// playing without mods, and for hollow knight handing the game back to the player's own mods
const restoreConfirm = ref<InstanceType<typeof ConfirmModal> | null>(null)

async function playWithoutMods() {
	try {
		if (games.activeGame !== game.value) await games.switchGame(game.value)
		await modpacks.launchWithoutMods()
		addNotification({ title: `Launching ${gameName.value} without mods`, type: 'success' })
	} catch (err) {
		handleError(err as Error)
	}
}

async function restoreOriginal() {
	try {
		const restored = await modpacks.restoreOriginal()
		addNotification({
			title: restored ? 'Your own mods are back' : 'Nothing to restore',
			text: restored
				? 'Hollow Knight has the mods it had before you started using modpacks.'
				: 'Hollow Knight already has your own mods.',
			type: 'success',
		})
	} catch (err) {
		handleError(err as Error)
	}
}

const moreOptions = computed<ButtonMenuOption[]>(() => [
	{
		id: 'edit',
		label: 'Edit details',
		icon: EditIcon,
		action: () => {
			if (modpack.value) editor.value?.show(modpack.value)
		},
	},
	{ id: 'check', label: 'Check for updates', icon: RefreshCwIcon, action: () => void ctx.reloadCatalog() },
	{ id: 'duplicate', label: 'Duplicate', icon: CopyIcon, action: duplicate },
	{ type: 'divider' },
	{
		id: 'vanilla',
		label: `Play ${gameName.value} without mods`,
		icon: PlayIcon,
		disabled: gameMissing.value,
		action: playWithoutMods,
	},
	{
		id: 'restore',
		label: 'Switch back to my own mods',
		icon: UndoIcon,
		shown: isActiveInGame.value,
		action: () => restoreConfirm.value?.show(),
	},
	{ type: 'divider' },
	{ id: 'delete', label: 'Delete modpack', icon: TrashIcon, tone: 'red', action: () => deleteConfirm.value?.show() },
])

const problemText = computed(() => {
	const h = health.value
	const parts: string[] = []
	const missing = [...h.installableMissing, ...h.unavailable]
	if (missing.length) parts.push(`${listNames(missing.map(ctx.label))} ${missing.length === 1 ? 'is' : 'are'} missing`)
	if (h.disabledRequired.length) {
		parts.push(`${listNames(h.disabledRequired.map(ctx.label))} ${h.disabledRequired.length === 1 ? 'is' : 'are'} disabled`)
	}
	if (h.outdatedRequired.length) {
		parts.push(`${listNames(h.outdatedRequired.map(ctx.label))} ${h.outdatedRequired.length === 1 ? 'needs' : 'need'} an update`)
	}
	return parts.join('; ')
})

</script>

<template>
	<div v-if="modpack" class="relative flex h-full min-h-0">
		<div class="modpack-scroll nl-scroll min-w-0 flex-1">
			<!-- header -->
			<header class="hero" :style="art ? { '--_tint': art.tint } : undefined">
				<ModpackArt :seed="modpack.path" :game="modpack.game" size="6.5rem" />
				<div class="flex min-w-0 flex-1 flex-col gap-1.5 self-center">
					<div class="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1">
						<h1 class="title" :title="modpack.name">{{ modpack.name }}</h1>
						<span
							v-if="isActiveInGame"
							v-tooltip="'Hollow Knight loads this modpack, even when started from Steam'"
							class="nl-badge nl-badge--brand"
							>Active</span
						>
					</div>
					<p
						v-if="modpack.description"
						class="nl-selectable m-0 line-clamp-2 max-w-3xl break-words text-[0.9375rem] leading-relaxed text-base"
						:title="modpack.description"
					>
						{{ modpack.description }}
					</p>
					<div class="meta">
						<span class="inline-flex items-center gap-1.5">
							<PackageIcon class="h-4 w-4" aria-hidden="true" />
							<template v-if="db">
								{{ health.installed.length }} mod{{ health.installed.length === 1 ? '' : 's' }}
								<template v-if="health.installed.length - health.enabledCount">
									· {{ health.installed.length - health.enabledCount }} disabled
								</template>
							</template>
							<template v-else>Counting mods…</template>
						</span>
						<span class="inline-flex items-center gap-1.5" :title="modpack.last_played ?? undefined">
							<ClockIcon class="h-4 w-4" aria-hidden="true" /> {{ playedLabel(modpack.last_played) }}
						</span>
					</div>
				</div>
				<div class="actions">
					<Button type="colored" color="brand" size="xl" :disabled="!!modpacks.launching" @click="ctx.play()">
						<SpinnerIcon v-if="modpacks.launching === modpack.path" class="animate-spin" />
						<PlayIcon v-else />
						{{ modpacks.launching === modpack.path ? 'Launching…' : 'Play' }}
					</Button>
					<Button size="xl" @click="router.push(browseRoute)"><PlusIcon /> Browse mods</Button>
					<TeleportOverflowMenu size="lg" type="quiet" label="More modpack options" :options="moreOptions">
						<MoreVerticalIcon />
					</TeleportOverflowMenu>
				</div>
			</header>

			<!-- status -->
			<div
				v-if="activity || gameMissing || health.issues.size || health.updates.length"
				class="flex flex-col gap-2 px-[--nl-page-px] pb-5"
			>
				<div v-if="activity" class="notice notice--brand" role="status" aria-live="polite">
					<SpinnerIcon class="h-5 w-5 shrink-0 animate-spin text-brand" aria-hidden="true" />
					<p class="m-0 min-w-0 flex-1 text-sm">
						<span class="font-semibold text-contrast">
							Installing {{ listNames(activity.names.map(ctx.label)) }}
						</span>
						<span v-if="activity.current" class="text-secondary">
							· downloading {{ ctx.label(activity.current) }}
							<template v-if="modpacks.progress.has(activity.current)">
								({{ Math.round(modpacks.progress.get(activity.current) ?? 0) }}%)
							</template>
						</span>
					</p>
				</div>

				<div v-if="gameMissing" class="notice notice--orange">
					<TriangleAlertIcon class="h-5 w-5 shrink-0 text-orange" aria-hidden="true" />
					<p class="m-0 min-w-0 flex-1 text-sm">
						<span class="font-semibold text-contrast">{{ gameName }} wasn't found on this computer.</span>
						<span class="text-secondary"> You can still build this modpack; locate the game to play it.</span>
					</p>
					<Button size="sm" @click="ui.openSettings('games')">Locate game</Button>
				</div>

				<div v-if="health.issues.size" class="notice notice--red">
					<CircleAlertIcon class="h-5 w-5 shrink-0 text-red" aria-hidden="true" />
					<div class="min-w-0 flex-1 text-sm">
						<p class="m-0 font-semibold text-contrast">
							{{ health.issues.size }} mod{{ health.issues.size === 1 ? '' : 's' }} won't load
						</p>
						<p class="m-0 mt-0.5 break-words text-secondary">
							{{ problemText }}.
							<template v-if="health.unavailable.length && !health.fixable">
								{{ health.unavailable.length === 1 ? "It can't" : "They can't" }} be downloaded.
							</template>
						</p>
					</div>
					<div class="flex shrink-0 flex-wrap justify-end gap-2">
						<Button v-if="show !== 'issues'" size="sm" type="quiet" @click="show = 'issues'">Show</Button>
						<Button v-if="health.fixable" size="sm" type="colored" color="brand" @click="ctx.fix()">
							<WrenchIcon /> Fix
						</Button>
					</div>
				</div>

				<div v-if="health.updates.length" class="notice notice--orange">
					<UpdatedIcon class="h-5 w-5 shrink-0 text-orange" aria-hidden="true" />
					<p class="m-0 min-w-0 flex-1 text-sm">
						<span class="font-semibold text-contrast">
							{{ health.updates.length }} update{{ health.updates.length === 1 ? '' : 's' }} available
						</span>
						<span class="text-secondary">
							· {{ health.updates.map((m) => ctx.label(m.name)).slice(0, 4).join(', ')
							}}{{ health.updates.length > 4 ? `, +${health.updates.length - 4} more` : '' }}
						</span>
					</p>
					<Button
						size="sm"
						type="colored"
						color="orange"
						:disabled="health.updates.some((m) => ctx.isBusy(m.name))"
						@click="ctx.updateAll()"
					>
						<RefreshCwIcon /> Update all
					</Button>
				</div>
			</div>

			<!-- toolbar, sticky -->
			<div class="toolbar">
				<h2 class="section-title">
					Mods <span class="nl-count">{{ installed.length }}</span>
				</h2>
				<label class="field min-w-[12rem] flex-1">
					<SearchIcon class="h-4 w-4 shrink-0 text-secondary" aria-hidden="true" />
					<input
						ref="searchInput"
						v-model="search"
						type="search"
						placeholder="Search installed mods"
						aria-label="Search installed mods"
					/>
					<button v-if="search" type="button" class="clear" aria-label="Clear search" @click="search = ''">
						<XIcon class="h-3.5 w-3.5" />
					</button>
				</label>
				<SelectMenu v-model="show" label="Show" :icon="FilterIcon" :options="showOptions" />
				<SelectMenu v-model="sort" label="Sort" :icon="SortAscIcon" :options="sortOptions" />
				<span v-if="catalogState?.loading" class="flex items-center gap-2 text-xs text-secondary">
					<SpinnerIcon class="h-3.5 w-3.5 animate-spin" /> Checking for updates…
				</span>
			</div>

			<!-- body -->
			<div class="body">
				<div v-if="ctx.installedError.value" class="state">
					<NoConnectionIllustration class="illustration" aria-hidden="true" />
					<h2>Couldn't read this modpack</h2>
					<p class="nl-selectable">{{ ctx.installedError.value }}</p>
					<Button @click="modpacks.loadInstalled(path)">Try again</Button>
				</div>

				<div v-else-if="!db" class="mod-table">
					<div v-for="i in 5" :key="i" class="skeleton-row">
						<div class="flex flex-1 flex-col gap-1.5">
							<div class="nl-skeleton h-3.5 w-1/3" />
							<div class="nl-skeleton h-3 w-1/2" />
						</div>
					</div>
				</div>

				<div v-else-if="installed.length === 0" class="state">
					<EmptyIllustration class="illustration" aria-hidden="true" />
					<h2>No mods yet</h2>
					<p>Find mods for {{ gameName }} and install them here. Anything a mod needs is installed with it.</p>
					<Button type="colored" color="brand" @click="router.push(browseRoute)"><SearchIcon /> Browse mods</Button>
				</div>

				<template v-else>
					<p
						v-if="catalogState?.error"
						class="mb-3 mt-0 flex items-center gap-2 rounded-lg bg-bg-orange px-3 py-2 text-sm text-orange"
					>
						<TriangleAlertIcon class="h-4 w-4 shrink-0" />
						<span class="min-w-0 flex-1">Couldn't check for updates right now.</span>
						<button type="button" class="link-button" @click="ctx.reloadCatalog()">Try again</button>
					</p>

					<div v-if="visible.length === 0" class="state state--compact">
						<NoSearchResultIllustration class="illustration illustration--small" aria-hidden="true" />
						<p>
							No mods match
							<template v-if="search">“<span class="break-all text-contrast">{{ search }}</span>”</template>
							<template v-else>this filter</template>.
						</p>
						<Button size="sm" @click="clearFilters">Clear filters</Button>
					</div>

					<div
						v-else
						ref="table"
						class="mod-table"
						role="table"
						aria-label="Installed mods"
						@keydown="onTableKeydown"
					>
						<div class="mod-head" role="row">
							<span role="columnheader" class="flex items-center">
								<Checkbox
									:model-value="allVisibleChecked"
									:indeterminate="someVisibleChecked && !allVisibleChecked"
									description="Select all shown mods"
									@update:model-value="toggleAll"
								/>
							</span>
							<span role="columnheader">
								<template v-if="checked.size">{{ checked.size }} selected</template>
								<template v-else>Name</template>
							</span>
							<span role="columnheader" class="head-version">Version</span>
							<span role="columnheader" class="text-right">Actions</span>
						</div>
						<ModRow
							v-for="mod in visible"
							:key="mod.name"
							:mod="mod"
							:selected="selectedName === mod.name"
							:checked="checked.has(mod.name)"
							@open="toggleMod(mod.name)"
							@check="(event) => onCheck(mod.name, event)"
						/>
					</div>
				</template>
			</div>
		</div>

		<Transition name="panel">
			<ModPanel
				v-if="selectedName"
				class="side-panel"
				:name="selectedName"
				@close="selectedName = null"
				@open="(name) => (selectedName = name)"
			/>
		</Transition>

		<FloatingActionBar
			:shown="checked.size > 0"
			aria-label="Selected mods"
			class="selection-bar"
			:class="{ 'with-panel': selectedName }"
		>
			<span class="px-2 text-sm font-semibold text-contrast">{{ checked.size }} selected</span>
			<div class="mx-1 h-6 w-px bg-surface-5" aria-hidden="true" />
			<Button type="quiet" :disabled="anyCheckedBusy" @click="bulkEnable(true)">
				<CheckCircleIcon /> Enable
			</Button>
			<Button type="quiet" :disabled="anyCheckedBusy" @click="bulkEnable(false)">
				<XCircleIcon /> Disable
			</Button>
			<Button v-if="checkedUpdates.length" type="quiet" :disabled="anyCheckedBusy" @click="ctx.update(checkedUpdates)">
				<RefreshCwIcon /> Update {{ checkedUpdates.length }}
			</Button>
			<Button type="quiet" color="red" :disabled="anyCheckedBusy" @click="ctx.uninstallMany(checkedNames)">
				<TrashIcon /> Uninstall
			</Button>
			<div class="ml-auto" />
			<Button type="quiet" @click="clearChecked"><XIcon /> Clear</Button>
		</FloatingActionBar>

		<ModpackEditorModal ref="editor" />
		<ConfirmModal
			ref="deleteConfirm"
			title="Delete this modpack?"
			:description="`${modpack?.name ?? 'This modpack'} and every mod in it will be deleted. Your save files aren't affected.`"
			proceed-label="Delete modpack"
			:markdown="false"
			@proceed="deletePack"
		/>
		<ConfirmModal
			ref="restoreConfirm"
			title="Switch back to your own mods?"
			description="Hollow Knight goes back to the mods it had before you started using modpacks. Your modpacks aren't changed; playing one switches to it again."
			proceed-label="Switch back"
			:danger="false"
			:markdown="false"
			@proceed="restoreOriginal"
		/>
	</div>
</template>

<style scoped>
.modpack-scroll {
	height: 100%;
	background: var(--color-bg);
}
.hero {
	position: relative;
	isolation: isolate;
	display: flex;
	align-items: center;
	gap: 1.5rem;
	padding: calc(var(--nl-page-py) + 0.75rem) var(--nl-page-px) 1.75rem;
}
/* a wash of the modpack's own artwork color behind the header */
.hero::before {
	content: '';
	position: absolute;
	inset: 0;
	z-index: -1;
	background:
		radial-gradient(48rem 16rem at 4% 0%, color-mix(in srgb, var(--_tint, var(--color-brand)) 20%, transparent), transparent 70%),
		linear-gradient(to bottom, color-mix(in srgb, var(--_tint, var(--color-brand)) 6%, transparent), transparent);
	pointer-events: none;
}
.title {
	margin: 0;
	min-width: 0;
	font-size: 2rem;
	font-weight: 800;
	line-height: 1.15;
	letter-spacing: -0.02em;
	color: var(--color-contrast);
	overflow-wrap: anywhere;
	display: -webkit-box;
	-webkit-line-clamp: 2;
	-webkit-box-orient: vertical;
	overflow: hidden;
}
.meta {
	display: flex;
	flex-wrap: wrap;
	align-items: center;
	gap: 0.25rem 1.25rem;
	padding-top: 0.125rem;
	font-size: 0.875rem;
	color: var(--color-secondary);
}
.actions {
	display: flex;
	flex-shrink: 0;
	align-items: center;
	gap: 0.5rem;
	align-self: center;
}
@media (max-width: 1100px) {
	.hero {
		flex-wrap: wrap;
	}
	.actions {
		width: 100%;
	}
}
.notice {
	display: flex;
	align-items: center;
	gap: 0.75rem;
	padding: 0.75rem 0.875rem;
	border-radius: var(--nl-panel-radius);
	border: 1px solid color-mix(in srgb, var(--_c) 30%, var(--color-divider));
	background: color-mix(in srgb, var(--_c) 7%, var(--surface-2));
}
.notice--red {
	--_c: var(--color-red);
}
.notice--orange {
	--_c: var(--color-orange);
}
.notice--brand {
	--_c: var(--color-brand);
}
.toolbar {
	position: sticky;
	top: 0;
	z-index: 5;
	display: flex;
	flex-wrap: wrap;
	align-items: center;
	gap: 0.625rem;
	padding: 0.75rem var(--nl-page-px);
	background: var(--color-bg);
	border-bottom: 1px solid var(--color-divider);
}
.section-title {
	display: flex;
	align-items: baseline;
	gap: 0.5rem;
	margin: 0 0.5rem 0 0;
	font-size: 1.25rem;
	font-weight: 800;
	color: var(--color-contrast);
}
.section-title .nl-count {
	font-size: 0.875rem;
	font-weight: 700;
	color: var(--color-secondary);
}
.body {
	padding: 1rem var(--nl-page-px) 6rem;
	container: modlist / inline-size;
}
.mod-table {
	display: flex;
	flex-direction: column;
	gap: 0.125rem;
	padding: 0.375rem;
	border-radius: var(--nl-panel-radius);
	border: 1px solid var(--color-divider);
	background: var(--surface-2);
}
/* same columns as the mod row */
.mod-head {
	display: grid;
	grid-template-columns: 1.25rem minmax(0, 1fr) 10rem 6.25rem;
	align-items: center;
	gap: 1rem;
	padding: 0.375rem var(--nl-row-px) 0.625rem;
	border-bottom: 1px solid var(--color-divider);
	margin-bottom: 0.25rem;
	font-size: 0.75rem;
	font-weight: 700;
	letter-spacing: 0.04em;
	text-transform: uppercase;
	color: var(--color-secondary);
}
@container modlist (max-width: 42rem) {
	.mod-head {
		grid-template-columns: 1.25rem minmax(0, 1fr) 6.25rem;
	}
	.head-version {
		display: none;
	}
}
.skeleton-row {
	display: flex;
	align-items: center;
	gap: 1rem;
	padding: 0.75rem var(--nl-row-px);
}
.field {
	display: flex;
	align-items: center;
	gap: 0.5rem;
	height: 2.5rem;
	max-width: 26rem;
	padding: 0 0.75rem;
	border-radius: var(--nl-control-radius);
	background: var(--surface-2);
	border: 1px solid var(--color-divider);
	transition: border-color 0.15s ease;
}
.field:focus-within {
	border-color: var(--color-brand);
}
.field input {
	flex: 1;
	min-width: 0;
	padding: 0;
	border: none;
	outline: none;
	background: transparent;
	box-shadow: none;
	color: var(--color-contrast);
	font-size: 0.875rem;
}
.field input::placeholder {
	color: var(--color-secondary);
}
.field input::-webkit-search-cancel-button {
	display: none;
}
.clear {
	display: inline-flex;
	padding: 0.125rem;
	border: none;
	border-radius: 0.25rem;
	background: transparent;
	color: var(--color-secondary);
	cursor: pointer;
}
.clear:hover {
	color: var(--color-contrast);
}
.state {
	display: flex;
	flex-direction: column;
	align-items: center;
	gap: 0.625rem;
	padding: 2.5rem 1.5rem 3.5rem;
	text-align: center;
}
.state h2 {
	margin: 0.25rem 0 0;
	font-size: 1.125rem;
	font-weight: 800;
	color: var(--color-contrast);
}
.state p {
	margin: 0 0 0.5rem;
	max-width: 28rem;
	font-size: 0.875rem;
	line-height: 1.6;
	color: var(--color-secondary);
	overflow-wrap: anywhere;
}
.state--compact {
	padding: 1.5rem 1rem 2.5rem;
}
.illustration {
	width: 10rem;
	height: auto;
}
.illustration--small {
	width: 7rem;
}
.link-button {
	padding: 0;
	border: none;
	background: none;
	color: inherit;
	font-weight: 700;
	text-decoration: underline;
	cursor: pointer;
}
.side-panel {
	position: absolute;
	top: 0;
	right: 0;
	bottom: 0;
	z-index: 10;
}
/* wide windows show the list and the details side by side */
@media (min-width: 1280px) {
	.side-panel {
		position: relative;
		flex-shrink: 0;
	}
}
.panel-enter-active,
.panel-leave-active {
	transition:
		transform 0.22s var(--nl-ease),
		opacity 0.18s ease;
}
.panel-enter-from,
.panel-leave-to {
	transform: translateX(1.5rem);
	opacity: 0;
}
</style>

<style>
/* the selection bar is teleported to the body, keep it over the mod list and clear of the sidebar */
.selection-bar {
	--left-bar-width: var(--nl-rail-width);
}
@media (min-width: 1280px) {
	.selection-bar.with-panel {
		--right-bar-width: 27rem;
	}
}
</style>
