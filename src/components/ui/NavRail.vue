<script setup lang="ts">
// the sidebar with every modpack, hollow knight's above silksong's until the player reorders them, plus new modpack and settings
import { PlayIcon, PlusIcon, SettingsIcon, SpinnerIcon } from '@modrinth/assets'
import { type ButtonMenuOption, TeleportOverflowMenu } from '@modrinth/ui'
import { computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import ModpackArt from '@/components/ui/ModpackArt.vue'
import { usePlayModpack } from '@/composables/play'
import { gameInfo, GAMES } from '@/helpers/games'
import { type Modpack, modpackRoute } from '@/helpers/modpacks'
import { analyzePack, buildIndex, mergeInstallState, type PackHealth } from '@/helpers/mods'
import { useCatalog } from '@/store/catalog'
import { useModpacks } from '@/store/modpacks'
import { useUi } from '@/store/ui'

const route = useRoute()
const router = useRouter()
const modpacks = useModpacks()
const catalog = useCatalog()
const ui = useUi()
const { play } = usePlayModpack()

const currentPath = computed(() => (route.params.path ? String(route.params.path) : null))
const packs = computed(() => modpacks.ordered)

// status dots for mods that won't load and for updates

watch(
	() => modpacks.list.map((m) => m.path),
	(paths) => {
		void modpacks.loadInstalledFor(paths)
		for (const game of new Set(modpacks.list.map((m) => m.game))) void catalog.load(game)
	},
	{ immediate: true },
)

const health = computed(() => {
	const out = new Map<string, PackHealth>()
	for (const pack of packs.value) {
		const db = modpacks.installed[pack.path]
		const items = catalog.entries[pack.game]?.items
		// updates and problems are only known once that game's mod list has loaded
		if (!db || !items) continue
		const entries = mergeInstallState(items, db)
		out.set(pack.path, analyzePack(entries, buildIndex(entries)))
	}
	return out
})

function tooltip(pack: Modpack) {
	const parts = [pack.name, gameInfo(pack.game).name]
	const h = health.value.get(pack.path)
	if (h?.issues.size) parts.push(`${h.issues.size} mod${h.issues.size === 1 ? '' : 's'} won't load`)
	else if (h?.updates.length) parts.push(`${h.updates.length} update${h.updates.length === 1 ? '' : 's'}`)
	return parts.join(' · ')
}

function playPack(pack: Modpack) {
	void play(pack, health.value.get(pack.path)?.updates.map((m) => m.name) ?? [])
}

const newOptions = computed<ButtonMenuOption[]>(() =>
	GAMES.map((game) => ({
		id: game.key,
		label: `${game.name} Modpack`,
		action: () => ui.createModpack(game.key),
	})),
)

// rearranging by dragging a tile, or alt up and down on a focused tile

const list = ref<HTMLElement | null>(null)
const dragging = ref<string | null>(null)
// where the dragged modpack would land, as an index among the other modpacks
const dropIndex = ref<number | null>(null)
let start: { path: string; y: number } | null = null
let suppressClick = false

function tiles() {
	return [...(list.value?.querySelectorAll<HTMLElement>('[data-pack]') ?? [])]
}

function onPointerDown(event: PointerEvent, pack: Modpack) {
	if (event.button !== 0) return
	start = { path: pack.path, y: event.clientY }
	window.addEventListener('pointermove', onPointerMove)
	window.addEventListener('pointerup', onPointerUp, { once: true })
}

function onPointerMove(event: PointerEvent) {
	if (!start) return
	if (!dragging.value) {
		if (Math.abs(event.clientY - start.y) < 6) return
		dragging.value = start.path
	}
	const others = tiles().filter((el) => el.dataset.pack !== dragging.value)
	const index = others.findIndex((el) => {
		const r = el.getBoundingClientRect()
		return event.clientY < r.top + r.height / 2
	})
	dropIndex.value = index < 0 ? others.length : index
	// keep scrolling while dragging past the ends of a long list
	const box = list.value?.getBoundingClientRect()
	if (box && event.clientY < box.top + 24) list.value!.scrollTop -= 8
	else if (box && event.clientY > box.bottom - 24) list.value!.scrollTop += 8
}

function onPointerUp() {
	window.removeEventListener('pointermove', onPointerMove)
	if (dragging.value && dropIndex.value !== null) {
		modpacks.move(dragging.value, dropIndex.value)
		suppressClick = true
		setTimeout(() => (suppressClick = false), 0)
	}
	dragging.value = null
	dropIndex.value = null
	start = null
}

function onClick(event: MouseEvent, pack: Modpack) {
	event.preventDefault()
	if (suppressClick) return
	void router.push(modpackRoute(pack))
}

function onKeydown(event: KeyboardEvent, pack: Modpack, index: number) {
	if (!event.altKey || (event.key !== 'ArrowUp' && event.key !== 'ArrowDown')) return
	event.preventDefault()
	const next = index + (event.key === 'ArrowUp' ? -1 : 1)
	if (next < 0 || next >= packs.value.length) return
	modpacks.move(pack.path, next)
	requestAnimationFrame(() =>
		list.value?.querySelector<HTMLElement>(`[data-pack="${CSS.escape(pack.path)}"] a`)?.focus(),
	)
}

// a thin line wherever the game changes between modpacks
const startsGroup = (index: number) =>
	index > 0 && packs.value[index - 1].game !== packs.value[index].game

// the drop line sits above the tile at the drop index, counted without the dragged one
function showsDropAbove(pack: Modpack) {
	if (dropIndex.value === null) return false
	const others = packs.value.filter((m) => m.path !== dragging.value)
	return others[dropIndex.value]?.path === pack.path
}
const dropAtEnd = computed(() => dropIndex.value !== null && dropIndex.value >= packs.value.length - 1)
</script>

<template>
	<nav class="rail" aria-label="Modpacks">
		<ul ref="list" class="packs" :class="{ 'is-dragging': dragging }">
			<li
				v-for="(pack, index) in packs"
				:key="pack.path"
				:data-pack="pack.path"
				class="pack"
				:class="{
					'starts-group': startsGroup(index),
					'is-dragged': dragging === pack.path,
					'drop-above': showsDropAbove(pack),
				}"
				@pointerdown="onPointerDown($event, pack)"
			>
				<a
					v-tooltip.right="dragging ? undefined : tooltip(pack)"
					:href="modpackRoute(pack)"
					class="tile"
					:class="{ 'is-active': currentPath === pack.path }"
					:aria-label="`${pack.name}, ${gameInfo(pack.game).name}. Alt and arrow keys move it.`"
					:aria-current="currentPath === pack.path ? 'page' : undefined"
					draggable="false"
					@click="onClick($event, pack)"
					@keydown="onKeydown($event, pack, index)"
				>
					<ModpackArt :seed="pack.path" :game="pack.game" size="2.5rem" />
					<span v-if="health.get(pack.path)?.issues.size" class="dot dot--red" aria-hidden="true" />
					<span v-else-if="health.get(pack.path)?.updates.length" class="dot dot--orange" aria-hidden="true" />
				</a>
				<button
					type="button"
					class="play"
					:class="{ 'is-launching': modpacks.launching === pack.path }"
					:disabled="!!modpacks.launching"
					:aria-label="`Play ${pack.name}`"
					@pointerdown.stop
					@click="playPack(pack)"
				>
					<SpinnerIcon v-if="modpacks.launching === pack.path" class="h-3 w-3 animate-spin" />
					<PlayIcon v-else class="play-icon" />
				</button>
			</li>
			<li v-if="dragging && dropAtEnd" class="drop-end" aria-hidden="true" />
		</ul>

		<div class="bottom">
			<div class="rail-menu">
				<TeleportOverflowMenu
					label="New modpack"
					tooltip="New modpack"
					type="quiet"
					size="lg"
					:circular="false"
					placement="right-end"
					:distance="22"
					:options="newOptions"
				>
					<PlusIcon />
				</TeleportOverflowMenu>
			</div>
			<button
				v-tooltip.right="'Settings'"
				type="button"
				class="rail-item"
				aria-label="Settings"
				@click="ui.openSettings()"
			>
				<SettingsIcon />
			</button>
		</div>
	</nav>
</template>

<style scoped>
.rail {
	display: flex;
	flex-direction: column;
	align-items: center;
	width: var(--nl-rail-width);
	height: 100%;
	min-height: 0;
	border-right: 1px solid var(--color-divider);
	background: var(--color-raised-bg);
}
.packs {
	display: flex;
	flex: 1;
	flex-direction: column;
	align-items: center;
	gap: 0.5rem;
	width: 100%;
	min-height: 0;
	margin: 0;
	padding: 0.75rem 0;
	list-style: none;
	overflow-y: auto;
	scrollbar-width: none;
}
.packs::-webkit-scrollbar {
	display: none;
}
.packs.is-dragging {
	cursor: grabbing;
	user-select: none;
}
.pack {
	position: relative;
	flex-shrink: 0;
	touch-action: none;
}
/* the line between one game's modpacks and the next */
.pack.starts-group {
	margin-top: 0.625rem;
}
.pack.starts-group::after {
	content: '';
	position: absolute;
	top: -0.625rem;
	left: 50%;
	width: 1.75rem;
	height: 1px;
	transform: translateX(-50%);
	background: var(--color-divider);
}
.pack.is-dragged {
	opacity: 0.35;
}
/* where a dragged modpack will land */
.pack.drop-above::before,
.drop-end {
	content: '';
	display: block;
	flex-shrink: 0;
	width: 2.5rem;
	height: 3px;
	border-radius: 3px;
	background: var(--color-brand);
}
.pack.drop-above::before {
	position: absolute;
	top: -0.375rem;
	left: 0;
}
.tile {
	position: relative;
	display: block;
	border-radius: 0.75rem;
	transition: transform 0.12s ease;
	-webkit-user-drag: none;
}
.pack:hover .tile {
	transform: scale(1.05);
}
.packs.is-dragging .tile {
	transform: none;
	cursor: grabbing;
}
.tile:focus-visible {
	outline: 2px solid var(--color-brand);
	outline-offset: 2px;
}
/* a small bar beside the open modpack */
.tile.is-active::before {
	content: '';
	position: absolute;
	left: calc((var(--nl-rail-width) - 2.5rem) / -2);
	top: 25%;
	bottom: 25%;
	width: 3px;
	border-radius: 0 3px 3px 0;
	background: var(--color-brand);
}
.dot {
	position: absolute;
	top: -0.1875rem;
	right: -0.1875rem;
	width: 0.6875rem;
	height: 0.6875rem;
	border-radius: 50%;
	border: 2px solid var(--color-raised-bg);
}
.dot--red {
	background: var(--color-red);
}
.dot--orange {
	background: var(--color-orange);
}
/* play button over the tile's corner while hovering or focusing it */
.play {
	position: absolute;
	right: -0.3125rem;
	bottom: -0.3125rem;
	display: inline-flex;
	align-items: center;
	justify-content: center;
	width: 1.375rem;
	height: 1.375rem;
	padding: 0;
	border: 2px solid var(--color-raised-bg);
	border-radius: 50%;
	background: var(--color-brand);
	color: var(--color-accent-contrast);
	cursor: pointer;
	opacity: 0;
	transform: scale(0.8);
	transition:
		opacity 0.12s ease,
		transform 0.12s ease;
}
.play-icon {
	width: 0.625rem;
	height: 0.625rem;
	margin-left: 0.0625rem;
	fill: currentColor;
}
.pack:hover .play,
.pack:focus-within .play,
.play.is-launching {
	opacity: 1;
	transform: scale(1);
}
.packs.is-dragging .play {
	opacity: 0;
}
.play:disabled:not(.is-launching) {
	opacity: 0 !important;
}
.play:focus-visible {
	outline: 2px solid var(--color-brand);
	outline-offset: 1px;
}
.bottom {
	display: flex;
	flex-direction: column;
	align-items: center;
	gap: 0.375rem;
	width: 100%;
	padding: 0.625rem 0 0.75rem;
	border-top: 1px solid var(--color-divider);
}
.rail-item,
.rail-menu :deep(button) {
	display: inline-flex;
	align-items: center;
	justify-content: center;
	width: 2.75rem;
	height: 2.75rem;
	padding: 0;
	border: none;
	border-radius: 0.875rem;
	background: transparent;
	color: var(--color-secondary);
	cursor: pointer;
	transition:
		background-color 0.12s ease,
		color 0.12s ease;
}
.rail-item svg,
.rail-menu :deep(button svg) {
	width: 1.375rem;
	height: 1.375rem;
}
.rail-item:hover,
.rail-menu :deep(button:hover) {
	background: var(--surface-4);
	color: var(--color-contrast);
}
.rail-item:focus-visible {
	outline: 2px solid var(--color-brand);
	outline-offset: 2px;
}
</style>
