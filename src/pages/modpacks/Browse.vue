<script setup lang="ts">
// browse finds mods for the modpack's game, its own page with categories on the left like modrinth discover
import {
	ArrowLeftIcon,
	NoConnectionIllustration,
	NoSearchResultIllustration,
	RefreshCwIcon,
	SearchIcon,
	SortAscIcon,
	XIcon,
} from '@modrinth/assets'
import { Button, Checkbox, type ComboboxOption } from '@modrinth/ui'
import { useIntersectionObserver, useStorage } from '@vueuse/core'
import { computed, ref, watch } from 'vue'
import { useRouter } from 'vue-router'

import CatalogRow from '@/components/modpacks/CatalogRow.vue'
import ModPanel from '@/components/modpacks/ModPanel.vue'
import ModpackArt from '@/components/ui/ModpackArt.vue'
import SelectMenu from '@/components/ui/SelectMenu.vue'
import { useModpackContext } from '@/composables/modpack'
import { useShortcut } from '@/composables/shortcuts'
import { modpackRoute } from '@/helpers/modpacks'
import { dependenciesOf, type ModEntry } from '@/helpers/mods'

type Sort = 'relevance' | 'name' | 'name-desc' | 'fewest-deps'

const router = useRouter()
const ctx = useModpackContext()
const { path, modpack, gameName, catalogState, catalogItems } = ctx

const backRoute = computed(() => modpackRoute({ path: path.value }))

// search, categories, sort

const PAGE = 40
const search = ref('')
const categories = ref<string[]>([])
const sort = useStorage<Sort>('nl-browse-sort', 'relevance')
const limit = ref(PAGE)

const catalogEntries = computed(() => ctx.entries.value.filter((m) => m.inCatalog))

const categoryCounts = computed(() => {
	const counts = new Map<string, number>()
	for (const mod of catalogEntries.value) {
		for (const tag of mod.tags) counts.set(tag, (counts.get(tag) ?? 0) + 1)
	}
	return [...counts.entries()].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
})

function toggleCategory(tag: string) {
	categories.value = categories.value.includes(tag)
		? categories.value.filter((t) => t !== tag)
		: [...categories.value, tag]
}

const sortOptions: ComboboxOption<Sort>[] = [
	{ value: 'relevance', label: 'Best match' },
	{ value: 'name', label: 'Name (A–Z)' },
	{ value: 'name-desc', label: 'Name (Z–A)' },
	{ value: 'fewest-deps', label: 'Fewest requirements' },
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

function score(mod: ModEntry, query: string) {
	const shown = ctx.label(mod.name).toLowerCase()
	if (shown === query) return 0
	if (shown.startsWith(query)) return 1
	if (shown.includes(query)) return 2
	return 3
}

const byName = (a: ModEntry, b: ModEntry) =>
	ctx.label(a.name).localeCompare(ctx.label(b.name), undefined, { sensitivity: 'base' })

const requirementCount = (mod: ModEntry) =>
	dependenciesOf(mod, ctx.index.value).filter((d) => d.status !== 'loader').length

const results = computed(() => {
	const query = search.value.trim().toLowerCase()
	let list = catalogEntries.value
	if (categories.value.length) list = list.filter((m) => m.tags.some((t) => categories.value.includes(t)))
	if (query) list = list.filter((m) => matches(m, query))
	const sorted = [...list].sort(byName)
	if (sort.value === 'relevance' && query) sorted.sort((a, b) => score(a, query) - score(b, query))
	else if (sort.value === 'name-desc') sorted.reverse()
	else if (sort.value === 'fewest-deps') {
		const counts = new Map(sorted.map((m) => [m.name, requirementCount(m)]))
		sorted.sort((a, b) => counts.get(a.name)! - counts.get(b.name)!)
	}
	return sorted
})
const shown = computed(() => results.value.slice(0, limit.value))
watch([search, categories, sort], () => {
	limit.value = PAGE
	resultsEl.value?.scrollTo({ top: 0 })
})

const resultsEl = ref<HTMLElement | null>(null)
const sentinel = ref<HTMLElement | null>(null)
useIntersectionObserver(
	sentinel,
	([entry]) => {
		if (entry?.isIntersecting && limit.value < results.value.length) limit.value += PAGE
	},
	{ rootMargin: '400px' },
)

function clearFilters() {
	search.value = ''
	categories.value = []
}

// details panel

const selectedName = ref<string | null>(null)
function toggleMod(name: string) {
	selectedName.value = selectedName.value === name ? null : name
}

// keyboard

const searchInput = ref<HTMLInputElement | null>(null)
useShortcut(['mod+f', '/'], () => searchInput.value?.focus())
useShortcut(
	'escape',
	(event) => {
		if (selectedName.value) selectedName.value = null
		else if (search.value && event.target === searchInput.value) search.value = ''
		else void router.push(backRoute.value)
	},
	{ inInputs: true },
)

function onListKeydown(event: KeyboardEvent) {
	if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return
	const rows = [...(resultsEl.value?.querySelectorAll<HTMLElement>('.result') ?? [])]
	const current = rows.indexOf(document.activeElement as HTMLElement)
	const next = rows[current < 0 ? 0 : current + (event.key === 'ArrowDown' ? 1 : -1)]
	if (!next) return
	event.preventDefault()
	next.focus()
	next.scrollIntoView({ block: 'nearest' })
	const index = rows.indexOf(next)
	if (selectedName.value && shown.value[index]) selectedName.value = shown.value[index].name
}
</script>

<template>
	<div v-if="modpack" class="browse">
		<header class="browse-head">
			<RouterLink v-tooltip="`Back to ${modpack.name}`" :to="backRoute" class="nl-icon-btn back" :aria-label="`Back to ${modpack.name}`">
				<ArrowLeftIcon />
			</RouterLink>
			<ModpackArt :seed="modpack.path" :game="modpack.game" size="2.75rem" />
			<div class="flex min-w-0 flex-col">
				<h1 class="m-0 text-xl font-extrabold leading-tight text-contrast">Browse mods</h1>
				<p class="m-0 truncate text-sm text-secondary">
					Installing to <span class="font-semibold text-contrast">{{ modpack.name }}</span>
				</p>
			</div>
			<label class="field ml-auto">
				<SearchIcon class="h-4 w-4 shrink-0 text-secondary" aria-hidden="true" />
				<input
					ref="searchInput"
					v-model="search"
					type="search"
					:placeholder="`Search ${catalogEntries.length ? `${catalogEntries.length} ` : ''}mods for ${gameName}`"
					:aria-label="`Search mods for ${gameName}`"
				/>
				<button v-if="search" type="button" class="clear" aria-label="Clear search" @click="search = ''">
					<XIcon class="h-3.5 w-3.5" />
				</button>
			</label>
			<SelectMenu v-model="sort" label="Sort" :icon="SortAscIcon" :options="sortOptions" />
		</header>

		<div class="browse-body">
			<aside class="filters nl-scroll" aria-label="Filters">
				<div class="flex items-center justify-between">
					<h2 class="nl-section-label">Categories</h2>
					<button v-if="categories.length" type="button" class="reset" @click="categories = []">Clear</button>
				</div>
				<ul v-if="categoryCounts.length" class="category-list">
					<li v-for="[tag, count] in categoryCounts" :key="tag">
						<Checkbox
							:model-value="categories.includes(tag)"
							class="category"
							:description="`${tag} (${count})`"
							@update:model-value="toggleCategory(tag)"
						>
							<span class="min-w-0 flex-1 truncate text-sm font-medium">{{ tag }}</span>
							<span class="text-xs font-semibold text-secondary">{{ count }}</span>
						</Checkbox>
					</li>
				</ul>
				<template v-else-if="!catalogItems">
					<div v-for="i in 8" :key="i" class="nl-skeleton my-2 h-4 w-3/4" />
				</template>
			</aside>

			<main ref="resultsEl" class="results nl-scroll" @keydown="onListKeydown">
				<div v-if="catalogState?.error && !catalogItems" class="state">
					<NoConnectionIllustration class="illustration" aria-hidden="true" />
					<h2>Couldn't load mods</h2>
					<p>Check your internet connection and try again.</p>
					<Button type="colored" color="brand" @click="ctx.reloadCatalog()"><RefreshCwIcon /> Try again</Button>
				</div>

				<div v-else-if="!catalogItems" class="flex flex-col gap-2">
					<div v-for="i in 6" :key="i" class="skeleton-card">
						<div class="nl-skeleton h-4 w-1/4" />
						<div class="nl-skeleton h-3 w-2/3" />
						<div class="nl-skeleton h-3 w-1/3" />
					</div>
				</div>

				<div v-else-if="results.length === 0" class="state">
					<NoSearchResultIllustration class="illustration" aria-hidden="true" />
					<p>
						No mods match
						<template v-if="search">“<span class="break-all text-contrast">{{ search }}</span>”</template>
						<template v-else>these categories</template>.
					</p>
					<Button size="sm" @click="clearFilters">Clear filters</Button>
				</div>

				<template v-else>
					<p class="m-0 mb-2.5 text-xs text-secondary">
						{{ results.length }} mod{{ results.length === 1 ? '' : 's' }}
						<template v-if="categories.length"> in {{ categories.join(', ') }}</template>
					</p>
					<div class="flex flex-col gap-2">
						<CatalogRow
							v-for="mod in shown"
							:key="mod.name"
							:mod="mod"
							:selected="selectedName === mod.name"
							@open="toggleMod(mod.name)"
						/>
					</div>
					<div ref="sentinel" class="h-8" aria-hidden="true" />
				</template>
			</main>

			<Transition name="panel">
				<ModPanel
					v-if="selectedName"
					class="side-panel"
					:name="selectedName"
					@close="selectedName = null"
				/>
			</Transition>
		</div>
	</div>
</template>

<style scoped>
.browse {
	display: flex;
	flex-direction: column;
	height: 100%;
	min-height: 0;
	background: var(--color-bg);
}
.browse-head {
	display: flex;
	flex-wrap: wrap;
	align-items: center;
	gap: 0.875rem;
	padding: 1rem var(--nl-page-px);
	border-bottom: 1px solid var(--color-divider);
	background:
		radial-gradient(40rem 8rem at 0% 0%, color-mix(in srgb, var(--color-brand) 12%, transparent), transparent 70%),
		var(--color-bg);
}
.back {
	width: 2.5rem;
	height: 2.5rem;
	border: 1px solid var(--color-divider);
	background: var(--surface-2);
}
.browse-body {
	position: relative;
	display: flex;
	flex: 1;
	min-height: 0;
}
.filters {
	display: flex;
	flex-direction: column;
	gap: 0.5rem;
	width: 15rem;
	flex-shrink: 0;
	padding: 1.25rem 1rem 2rem var(--nl-page-px);
	border-right: 1px solid var(--color-divider);
}
.filters h2 {
	margin: 0;
}
.reset {
	padding: 0;
	border: none;
	background: none;
	color: var(--color-brand);
	font-size: 0.75rem;
	font-weight: 700;
	cursor: pointer;
}
.category-list {
	display: flex;
	flex-direction: column;
	gap: 0.125rem;
	margin: 0 -0.5rem;
	padding: 0;
	list-style: none;
}
.category {
	width: 100%;
	gap: 0.625rem !important;
	padding: 0.4375rem 0.5rem !important;
	border-radius: 0.5rem !important;
	color: var(--color-base) !important;
}
.category:hover {
	background: var(--surface-2-5) !important;
}
.category[aria-checked='true'] {
	color: var(--color-contrast) !important;
}
.results {
	flex: 1;
	min-width: 0;
	padding: 1.25rem var(--nl-page-px) 3rem 1.5rem;
}
.field {
	display: flex;
	align-items: center;
	gap: 0.5rem;
	flex: 1;
	min-width: 14rem;
	max-width: 34rem;
	height: 2.5rem;
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
.skeleton-card {
	display: flex;
	flex-direction: column;
	gap: 0.625rem;
	padding: 1rem;
	border-radius: var(--nl-panel-radius);
	border: 1px solid var(--color-divider);
	background: var(--surface-2);
}
.state {
	display: flex;
	flex-direction: column;
	align-items: center;
	gap: 0.625rem;
	padding: 3rem 1.5rem;
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
.illustration {
	width: 8rem;
	height: auto;
}
.side-panel {
	position: absolute;
	top: 0;
	right: 0;
	bottom: 0;
	z-index: 10;
}
@media (min-width: 1280px) {
	.side-panel {
		position: relative;
		flex-shrink: 0;
	}
}
@media (max-width: 900px) {
	.filters {
		display: none;
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
