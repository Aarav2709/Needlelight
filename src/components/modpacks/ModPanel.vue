<script setup lang="ts">
import {
	BugIcon,
	CheckIcon,
	CircleAlertIcon,
	DownloadIcon,
	ExternalIcon,
	GlobeIcon,
	PlusIcon,
	RefreshCwIcon,
	SpinnerIcon,
	TrashIcon,
	WrenchIcon,
	XIcon,
} from '@modrinth/assets'
import { computed, nextTick, ref, watch } from 'vue'

import ModReadme from '@/components/modpacks/ModReadme.vue'
import { useModpackContext } from '@/composables/modpack'
import {
	addOutcome,
	authorLine,
	companionsOf,
	dependenciesOf,
	type DependencyRef,
	dependentsOf,
	formatVersion,
	installedVersion,
	isEnabled,
	isInstalled,
	isUntracked,
	type ModEntry,
	needsUpdate,
	projectLinks,
	shortAuthorLine,
} from '@/helpers/mods'
import { useReadmes } from '@/store/readmes'

const props = defineProps<{ name: string }>()
const emit = defineEmits<{ close: []; open: [name: string] }>()

const ctx = useModpackContext()
const readmes = useReadmes()

const mod = computed<ModEntry | null>(() => ctx.find(props.name))
const title = computed(() => ctx.label(props.name))
const author = computed(() => (mod.value ? shortAuthorLine(mod.value, ctx.game.value) : ''))
const allAuthors = computed(() => (mod.value ? authorLine(mod.value, ctx.game.value) : ''))
const inPack = computed(() => !!mod.value && isInstalled(mod.value))
const enabled = computed(() => !!mod.value && isEnabled(mod.value))
const current = computed(() => (mod.value ? installedVersion(mod.value) : null))
const hasUpdate = computed(() => !!mod.value && needsUpdate(mod.value))
const untracked = computed(() => !!mod.value && isUntracked(mod.value))
const busy = computed(() => ctx.isBusy(props.name))
const progress = computed(() => ctx.progressFor(props.name))
const links = computed(() => (mod.value ? projectLinks(mod.value) : null))

type Tab = 'about' | 'dependencies'
const tab = ref<Tab>('about')

// dependencies

const outcome = computed(() =>
	mod.value && !inPack.value ? addOutcome(props.name, ctx.index.value) : null,
)
const dependencies = computed(() => (mod.value ? dependenciesOf(mod.value, ctx.index.value) : []))
const companions = computed(() => (mod.value ? companionsOf(mod.value, ctx.index.value) : []))
const usedBy = computed(() => dependentsOf(props.name, ctx.installed.value))
const problems = computed(() =>
	inPack.value && enabled.value ? dependencies.value.filter((d) => d.status !== 'enabled') : [],
)
const fixable = computed(() => problems.value.some((d) => d.status !== 'unavailable'))
const dependencyCount = computed(() =>
	inPack.value ? dependencies.value.length : (outcome.value?.added.length ?? 0) + (outcome.value?.present.length ?? 0),
)

type Item = { key: string; name: string; mod: ModEntry | null; tag: string; tone: string; note?: string }

// what installing this mod does to the modpack, one line per affected mod
const installItems = computed<Item[]>(() => {
	const o = outcome.value
	if (!o) return []
	const item = (m: ModEntry, tag: string, tone: string): Item => ({ key: m.name, name: m.name, mod: m, tag, tone })
	return [
		...o.added.map((m) => item(m, 'Will be installed', 'neutral')),
		...o.updated.map((m) => item(m, 'Will be updated', 'orange')),
		...o.enabled.map((m) => item(m, 'Will be enabled', 'neutral')),
		...o.present.filter((m) => !o.enabled.includes(m)).map((m) => item(m, 'Installed', 'neutral')),
		...o.unavailable.map((name) => ({ key: name, name, mod: null, tag: "Can't be downloaded", tone: 'red' })),
	]
})

const installSummary = computed(() => {
	const o = outcome.value
	if (!o) return ''
	if (o.added.length) {
		return `Installing ${title.value} also installs ${o.added.length} mod${o.added.length === 1 ? '' : 's'} it needs.`
	}
	if (o.updated.length || o.enabled.length) return `Installing ${title.value} also updates or enables what it needs.`
	if (o.unavailable.length) return `${title.value} needs mods that can't be downloaded, so it may not work.`
	if (o.present.length) return 'Everything it needs is already installed.'
	return 'It works on its own.'
})

const DEP_TAG: Record<DependencyRef['status'], { tag: string; tone: string }> = {
	enabled: { tag: 'Installed', tone: 'neutral' },
	disabled: { tag: 'Disabled', tone: 'orange' },
	outdated: { tag: 'Needs update', tone: 'orange' },
	missing: { tag: 'Missing', tone: 'red' },
	unavailable: { tag: "Can't be downloaded", tone: 'red' },
	loader: { tag: '', tone: 'neutral' },
}

const installedItems = computed<Item[]>(() =>
	dependencies.value.map((dep) => ({
		key: dep.name,
		name: dep.mod?.name ?? dep.name,
		mod: dep.mod,
		tag: DEP_TAG[dep.status].tag,
		tone: DEP_TAG[dep.status].tone,
		note: dep.status === 'outdated' && dep.minVersion ? `Needs ${formatVersion(dep.minVersion)} or newer` : undefined,
	})),
)

const installedSummary = computed(() => {
	if (!dependencies.value.length) return 'It works on its own.'
	if (!enabled.value) return "It's disabled, so these only matter once it's enabled again."
	if (!problems.value.length) return 'Everything it needs is installed.'
	return "Some of what it needs isn't ready, so it won't load until that's fixed."
})

function subline(m: ModEntry | null) {
	if (!m) return ''
	return [shortAuthorLine(m, ctx.game.value), formatVersion(installedVersion(m) ?? m.version)].filter(Boolean).join(' · ')
}

// readme

const readme = computed(() =>
	mod.value?.repository ? readmes.get(mod.value.repository, mod.value.version) : null,
)
watch(
	mod,
	(value) => {
		if (value?.inCatalog && value.repository) void readmes.load(value.repository, value.version)
	},
	{ immediate: true },
)

function pageLabel(url: string) {
	if (/^https?:\/\/(www\.)?thunderstore\.io\//i.test(url)) return 'Thunderstore'
	if (/^https?:\/\/(www\.)?github\.com\//i.test(url)) return 'GitHub'
	return 'Website'
}

// keyboard and focus

const closeButton = ref<HTMLButtonElement | null>(null)
const body = ref<HTMLElement | null>(null)
watch(
	() => props.name,
	() => {
		void nextTick(() => {
			closeButton.value?.focus({ preventScroll: true })
			body.value?.scrollTo({ top: 0 })
		})
	},
	{ immediate: true },
)
</script>

<template>
	<aside class="panel" :aria-label="`${title} details`">
		<header class="hero">
			<button
				ref="closeButton"
				v-tooltip="'Close'"
				type="button"
				class="nl-icon-btn close"
				aria-label="Close"
				@click="emit('close')"
			>
				<XIcon />
			</button>
			<h2 class="hero-title">{{ title }}</h2>
			<p v-if="author" class="m-0 text-sm text-secondary">
				by <span class="font-semibold text-contrast" :title="allAuthors">{{ author }}</span>
			</p>
			<div v-if="mod" class="stats">
				<span v-if="mod.version">{{ formatVersion(current ?? mod.version) }}</span>
				<span v-if="inPack && !enabled" class="text-orange">Disabled</span>
			</div>
		</header>

		<div v-if="mod" class="cta">
			<template v-if="!inPack">
				<button
					type="button"
					class="nl-btn nl-btn--primary nl-btn--lg nl-btn--block"
					:disabled="busy"
					@click="ctx.install(name)"
				>
					<SpinnerIcon v-if="busy" class="animate-spin" />
					<DownloadIcon v-else />
					{{ busy ? (progress !== undefined ? `Installing… ${Math.round(progress)}%` : 'Installing…') : 'Install' }}
				</button>
				<p v-if="outcome?.added.length" class="m-0 text-center text-xs text-secondary">
					Also installs {{ outcome.added.length }} mod{{ outcome.added.length === 1 ? '' : 's' }} it needs
				</p>
			</template>
			<template v-else>
				<div class="flex gap-2">
					<button
						v-if="hasUpdate || untracked"
						type="button"
						class="nl-btn nl-btn--update nl-btn--lg min-w-0 flex-1"
						:disabled="busy"
						@click="ctx.update([name])"
					>
						<SpinnerIcon v-if="busy" class="animate-spin" />
						<RefreshCwIcon v-else />
						{{ hasUpdate ? `Update to ${formatVersion(mod.version)}` : 'Reinstall' }}
					</button>
					<button
						type="button"
						class="nl-btn nl-btn--danger nl-btn--lg min-w-0 flex-1"
						:disabled="busy"
						@click="ctx.uninstall(name)"
					>
						<SpinnerIcon v-if="busy && !(hasUpdate || untracked)" class="animate-spin" />
						<TrashIcon v-else /> Uninstall
					</button>
				</div>
				<button
					v-if="fixable"
					type="button"
					class="nl-btn nl-btn--tonal nl-btn--block"
					:disabled="busy"
					@click="ctx.fix()"
				>
					<WrenchIcon /> Fix what it needs
				</button>
			</template>
		</div>

		<div v-if="mod" class="panel-tabs" role="tablist">
			<button type="button" role="tab" :aria-selected="tab === 'about'" @click="tab = 'about'">About</button>
			<button
				v-if="mod.inCatalog"
				type="button"
				role="tab"
				:aria-selected="tab === 'dependencies'"
				@click="tab = 'dependencies'"
			>
				Dependencies
				<span class="count">{{ dependencyCount }}</span>
				<CircleAlertIcon v-if="problems.length" class="h-3.5 w-3.5 text-red" aria-label="Has problems" />
			</button>
		</div>

		<div v-if="mod" ref="body" class="panel-body nl-scroll">
			<!-- about -->
			<template v-if="tab === 'about' || !mod.inCatalog">
				<div v-if="links && (links.page || links.website || links.issues)" class="links">
					<button v-if="links.page" type="button" class="link-chip" @click="ctx.openLink(links.page)">
						<ExternalIcon class="h-3.5 w-3.5" /> {{ pageLabel(links.page) }}
					</button>
					<button v-if="links.website" type="button" class="link-chip" @click="ctx.openLink(links.website)">
						<GlobeIcon class="h-3.5 w-3.5" /> {{ pageLabel(links.website) === 'GitHub' ? 'GitHub' : 'Website' }}
					</button>
					<button v-if="links.issues" type="button" class="link-chip link-chip--report" @click="ctx.openLink(links.issues)">
						<BugIcon class="h-3.5 w-3.5" /> Report an issue
					</button>
				</div>

				<p v-if="!mod.inCatalog" class="lede">
					This mod was added to the modpack outside Needlelight, so there's no information about it
					here. You can still enable, disable or uninstall it.
				</p>
				<div v-else-if="readme?.status === 'loading'" class="flex flex-col gap-2.5 py-1" aria-label="Loading">
					<div class="nl-skeleton h-4 w-2/5" />
					<div class="nl-skeleton h-3.5 w-full" />
					<div class="nl-skeleton h-3.5 w-11/12" />
					<div class="nl-skeleton h-3.5 w-3/4" />
				</div>
				<ModReadme
					v-else-if="readme?.status === 'ready' && readme.readme"
					:markdown="readme.readme.markdown"
					:image-base="readme.readme.image_base"
					:link-base="readme.readme.link_base"
					@open-link="ctx.openLink"
				/>
				<p v-else class="nl-selectable lede whitespace-pre-line">
					{{ mod.description || "The author hasn't written a description." }}
				</p>

				<section v-if="mod.inCatalog" class="facts" aria-label="Details">
					<h3 class="sub-head">Details</h3>
					<dl>
						<template v-if="inPack && current">
							<dt>Installed</dt>
							<dd>{{ formatVersion(current) }}</dd>
						</template>
						<template v-if="mod.version && (!inPack || hasUpdate)">
							<dt>{{ inPack ? 'Latest' : 'Version' }}</dt>
							<dd>
								{{ formatVersion(mod.version) }}
								<span v-if="hasUpdate" class="text-orange"> · update available</span>
							</dd>
						</template>
						<template v-if="allAuthors">
							<dt>{{ mod.authors.length > 1 ? 'Authors' : 'Author' }}</dt>
							<dd class="nl-selectable">{{ allAuthors }}</dd>
						</template>
						<template v-if="mod.tags.length">
							<dt>Tags</dt>
							<dd class="flex flex-wrap gap-1.5">
								<span v-for="tag in mod.tags" :key="tag" class="tag">{{ tag }}</span>
							</dd>
						</template>
					</dl>
					<p v-if="!links?.issues && links?.page" class="m-0 text-xs leading-relaxed text-secondary">
						This project doesn't list an issue tracker; contact the author through its
						{{ pageLabel(links.page) }} page.
					</p>
				</section>
			</template>

			<!-- dependencies -->
			<template v-else>
				<p class="lede">{{ inPack ? installedSummary : installSummary }}</p>
				<ul v-if="(inPack ? installedItems : installItems).length" class="dep-list">
					<li v-for="item in inPack ? installedItems : installItems" :key="item.key">
						<button
							type="button"
							class="dep"
							:disabled="!item.mod"
							:title="item.mod ? `View ${ctx.label(item.name)}` : undefined"
							@click="item.mod && emit('open', item.mod.name)"
						>
							<span class="flex min-w-0 flex-1 flex-col text-left">
								<span class="truncate font-semibold text-contrast">{{ ctx.label(item.name) }}</span>
								<span class="truncate text-xs text-secondary">{{ item.note ?? subline(item.mod) }}</span>
							</span>
							<span class="nl-badge nl-badge--sm" :class="`nl-badge--${item.tone}`">
								<CheckIcon v-if="item.tag === 'Installed'" />
								<PlusIcon v-else-if="item.tag === 'Will be installed'" />
								<CircleAlertIcon v-else-if="item.tone === 'red'" />
								{{ item.tag }}
							</span>
						</button>
					</li>
				</ul>

				<template v-if="inPack && usedBy.length">
					<h3 class="sub-head">Used by</h3>
					<ul class="dep-list">
						<li v-for="dependent in usedBy" :key="dependent.name">
							<button type="button" class="dep" @click="emit('open', dependent.name)">
								<span class="flex min-w-0 flex-1 flex-col text-left">
									<span class="truncate font-semibold text-contrast">{{ ctx.label(dependent.name) }}</span>
									<span class="truncate text-xs text-secondary">{{ subline(dependent) }}</span>
								</span>
								<span v-if="!isEnabled(dependent)" class="nl-badge nl-badge--sm nl-badge--orange">Disabled</span>
							</button>
						</li>
					</ul>
				</template>

				<template v-if="companions.length">
					<h3 class="sub-head">Works well with</h3>
					<p class="lede">Optional mods that add extra features.</p>
					<ul class="dep-list">
						<li v-for="companion in companions" :key="companion.name" class="flex items-center gap-2">
							<button
								type="button"
								class="dep min-w-0 flex-1"
								:disabled="!companion.mod"
								@click="companion.mod && emit('open', companion.mod.name)"
							>
								<span class="flex min-w-0 flex-1 flex-col text-left">
									<span class="truncate font-semibold text-contrast">{{ ctx.label(companion.name) }}</span>
									<span class="truncate text-xs text-secondary">{{ subline(companion.mod) || 'Optional' }}</span>
								</span>
								<span v-if="companion.status === 'enabled' || companion.status === 'disabled'" class="nl-badge nl-badge--sm nl-badge--neutral">
									<CheckIcon /> Installed
								</span>
							</button>
							<button
								v-if="companion.status === 'missing'"
								type="button"
								class="nl-btn nl-btn--tonal nl-btn--sm"
								:disabled="ctx.isBusy(companion.mod!.name)"
								:aria-label="`Install ${ctx.label(companion.name)}`"
								@click="ctx.install(companion.mod!.name)"
							>
								<SpinnerIcon v-if="ctx.isBusy(companion.mod!.name)" class="animate-spin" />
								<DownloadIcon v-else />
								Install
							</button>
						</li>
					</ul>
				</template>
			</template>
		</div>

		<p v-else class="lede p-6">This mod is no longer in the mod list.</p>
	</aside>
</template>

<style scoped>
.panel {
	display: flex;
	flex-direction: column;
	width: min(27rem, 100%);
	height: 100%;
	min-height: 0;
	border-left: 1px solid var(--color-divider);
	background: var(--surface-1-5);
	box-shadow: -20px 0 44px rgba(0, 0, 0, 0.25);
}
.hero {
	position: relative;
	isolation: isolate;
	display: flex;
	flex-direction: column;
	gap: 0.375rem;
	padding: 1.5rem 3.75rem 1rem 1.25rem;
	overflow: hidden;
}
/* a faint wash of the game's accent behind the header */
.hero::before {
	content: '';
	position: absolute;
	inset: 0;
	z-index: -1;
	background: radial-gradient(28rem 10rem at 0% 0%, color-mix(in srgb, var(--color-brand) 16%, transparent), transparent 75%);
}
.hero-title {
	margin: 0;
	font-size: 1.375rem;
	font-weight: 800;
	line-height: 1.2;
	letter-spacing: -0.01em;
	color: var(--color-contrast);
	overflow-wrap: anywhere;
}
.close {
	position: absolute;
	top: 0.75rem;
	right: 0.75rem;
	background: color-mix(in srgb, var(--surface-3) 70%, transparent);
}
.stats {
	display: flex;
	flex-wrap: wrap;
	gap: 0.25rem 0.875rem;
	padding-top: 0.25rem;
	font-size: 0.8125rem;
	color: var(--color-secondary);
	font-variant-numeric: tabular-nums;
}
.cta {
	display: flex;
	flex-direction: column;
	gap: 0.5rem;
	padding: 0.25rem 1.25rem 1rem;
}
.dep:focus-visible,
.link-chip:focus-visible {
	outline: 2px solid var(--color-brand);
	outline-offset: 2px;
}
.panel-tabs {
	display: flex;
	gap: 1.25rem;
	padding: 0 1.25rem;
	border-bottom: 1px solid var(--color-divider);
}
.panel-tabs > button {
	position: relative;
	display: inline-flex;
	align-items: center;
	gap: 0.375rem;
	padding: 0.625rem 0;
	border: none;
	background: none;
	color: var(--color-secondary);
	font-size: 0.875rem;
	font-weight: 700;
	cursor: pointer;
}
.panel-tabs > button:hover {
	color: var(--color-contrast);
}
.panel-tabs > button[aria-selected='true'] {
	color: var(--color-contrast);
}
.panel-tabs > button[aria-selected='true']::after {
	content: '';
	position: absolute;
	left: 0;
	right: 0;
	bottom: -1px;
	height: 2px;
	border-radius: 2px 2px 0 0;
	background: var(--color-brand);
}
.panel-tabs > button:focus-visible {
	outline: 2px solid var(--color-brand);
	outline-offset: 2px;
	border-radius: 0.25rem;
}
.count {
	min-width: 1.25rem;
	padding: 0 0.3125rem;
	border-radius: 0.3125rem;
	background: var(--surface-3);
	color: var(--color-base);
	font-size: 0.6875rem;
	line-height: 1.125rem;
	text-align: center;
}
.panel-body {
	display: flex;
	flex-direction: column;
	gap: 0.875rem;
	flex: 1;
	min-height: 0;
	padding: 1rem 1.25rem 2rem;
}
.lede {
	margin: 0;
	font-size: 0.875rem;
	line-height: 1.6;
	color: var(--color-secondary);
}
.links {
	display: flex;
	flex-wrap: wrap;
	gap: 0.375rem;
}
.link-chip {
	display: inline-flex;
	align-items: center;
	gap: 0.375rem;
	height: 1.875rem;
	padding: 0 0.625rem;
	border: 1px solid var(--color-divider);
	border-radius: 0.5rem;
	background: var(--surface-2);
	color: var(--color-base);
	font-size: 0.8125rem;
	font-weight: 600;
	cursor: pointer;
	transition:
		background-color 0.12s ease,
		color 0.12s ease;
}
.link-chip:hover {
	background: var(--surface-3);
	color: var(--color-contrast);
}
.link-chip--report {
	color: var(--color-brand);
}
.tag {
	padding: 0.1875rem 0.5rem;
	border-radius: 0.375rem;
	background: var(--surface-3);
	color: var(--color-base);
	font-size: 0.75rem;
	font-weight: 600;
}
.sub-head {
	margin: 0.5rem 0 0;
	font-size: 0.8125rem;
	font-weight: 800;
	color: var(--color-contrast);
}
.facts {
	display: flex;
	flex-direction: column;
	gap: 0.625rem;
	margin-top: 0.5rem;
	padding-top: 1rem;
	border-top: 1px solid var(--color-divider);
}
.facts .sub-head {
	margin: 0;
}
.facts dl {
	display: grid;
	grid-template-columns: max-content minmax(0, 1fr);
	gap: 0.5rem 1.25rem;
	margin: 0;
	font-size: 0.875rem;
}
.facts dt {
	color: var(--color-secondary);
}
.facts dd {
	margin: 0;
	min-width: 0;
	color: var(--color-contrast);
	overflow-wrap: anywhere;
	font-variant-numeric: tabular-nums;
}
.dep-list {
	display: flex;
	flex-direction: column;
	gap: 0.125rem;
	margin: 0 -0.5rem;
	padding: 0;
	list-style: none;
}
.dep {
	display: flex;
	align-items: center;
	gap: 0.75rem;
	width: 100%;
	min-width: 0;
	padding: 0.4375rem 0.5rem;
	border: none;
	border-radius: 0.5rem;
	background: transparent;
	color: inherit;
	font-size: 0.875rem;
	cursor: pointer;
	transition: background-color 0.12s ease;
}
.dep:hover:not(:disabled) {
	background: var(--surface-3);
}
.dep:disabled {
	cursor: default;
}
</style>
