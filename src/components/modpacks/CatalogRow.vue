<script setup lang="ts">
import { DownloadIcon, SpinnerIcon, TrashIcon } from '@modrinth/assets'
import { computed } from 'vue'

import { useModpackContext } from '@/composables/modpack'
import {
	addOutcome,
	authorLine,
	formatVersion,
	isInstalled,
	type ModEntry,
	needsUpdate,
	shortAuthorLine,
} from '@/helpers/mods'

const props = defineProps<{ mod: ModEntry; selected?: boolean }>()
const emit = defineEmits<{ open: [] }>()

const ctx = useModpackContext()

const name = computed(() => ctx.label(props.mod.name))
const author = computed(() => shortAuthorLine(props.mod, ctx.game.value))
const allAuthors = computed(() => authorLine(props.mod, ctx.game.value))
const installed = computed(() => isInstalled(props.mod))
const busy = computed(() => ctx.isBusy(props.mod.name))
const progress = computed(() => ctx.progressFor(props.mod.name))
const extra = computed(() =>
	installed.value ? 0 : addOutcome(props.mod.name, ctx.index.value).added.length,
)
</script>

<template>
	<article
		class="result"
		:class="{ 'is-selected': selected }"
		:style="progress !== undefined ? { '--_progress': `${progress}%` } : undefined"
		tabindex="0"
		:aria-label="`${name}. View details`"
		:aria-current="selected || undefined"
		@click="emit('open')"
		@keydown.enter.self="emit('open')"
		@keydown.space.self.prevent="emit('open')"
	>
		<div v-if="progress !== undefined" class="progress-fill" aria-hidden="true" />

		<div class="relative flex min-w-0 flex-1 flex-col gap-1">
			<div class="flex min-w-0 items-baseline gap-2">
				<h3 class="name" :title="name">{{ name }}</h3>
				<span v-if="author" class="author" :title="allAuthors">by {{ author }}</span>
			</div>
			<p class="m-0 line-clamp-2 text-sm leading-snug text-base" :title="mod.description || undefined">
				{{ mod.description || 'No description provided.' }}
			</p>
			<div class="meta">
				<span v-for="tag in mod.tags.slice(0, 3)" :key="tag" class="tag">{{ tag }}</span>
				<span v-if="mod.version">{{ formatVersion(mod.version) }}</span>
			</div>
		</div>

		<div class="relative flex w-[7.5rem] shrink-0 flex-col items-stretch gap-1.5 self-center" @click.stop @keydown.stop>
			<button
				v-if="!installed"
				type="button"
				class="nl-btn nl-btn--primary"
				:disabled="busy"
				:aria-label="`Install ${name}`"
				@click="ctx.install(mod.name)"
			>
				<SpinnerIcon v-if="busy" class="animate-spin" />
				<DownloadIcon v-else />
				{{ busy ? 'Installing' : 'Install' }}
			</button>
			<button
				v-else
				type="button"
				class="nl-btn nl-btn--danger"
				:disabled="busy"
				:aria-label="`Uninstall ${name}`"
				@click="ctx.uninstall(mod.name)"
			>
				<SpinnerIcon v-if="busy" class="animate-spin" />
				<TrashIcon v-else />
				Uninstall
			</button>
			<span v-if="installed && needsUpdate(mod)" class="caption text-orange">Update available</span>
			<span v-else-if="!installed && extra > 0" class="caption text-secondary">
				+ {{ extra }} mod{{ extra === 1 ? '' : 's' }} it needs
			</span>
		</div>
	</article>
</template>

<style scoped>
.result {
	position: relative;
	display: flex;
	align-items: flex-start;
	gap: 1rem;
	min-width: 0;
	padding: 0.875rem 1rem;
	border-radius: var(--nl-panel-radius);
	border: 1px solid var(--color-divider);
	background: var(--surface-2);
	overflow: hidden;
	cursor: pointer;
	transition:
		background-color 0.12s ease,
		border-color 0.12s ease;
}
.result:hover {
	background: var(--surface-3);
	border-color: var(--surface-5);
}
.result:focus-visible {
	outline: 2px solid var(--color-brand);
	outline-offset: 2px;
}
.result.is-selected {
	border-color: color-mix(in srgb, var(--color-brand) 55%, var(--color-divider));
	background: color-mix(in srgb, var(--color-brand) 9%, var(--surface-2));
}
.name {
	flex: 0 1 auto;
	min-width: 0;
	margin: 0;
	overflow: hidden;
	text-overflow: ellipsis;
	white-space: nowrap;
	font-size: 1rem;
	font-weight: 750;
	color: var(--color-contrast);
}
.author {
	flex: 0 100 auto;
	min-width: 0;
	overflow: hidden;
	text-overflow: ellipsis;
	white-space: nowrap;
	font-size: 0.875rem;
	color: var(--color-secondary);
}
.meta {
	display: flex;
	flex-wrap: wrap;
	align-items: center;
	gap: 0.25rem 0.875rem;
	min-width: 0;
	padding-top: 0.125rem;
	font-size: 0.8125rem;
	color: var(--color-secondary);
	font-variant-numeric: tabular-nums;
}
.tag {
	padding: 0.0625rem 0.4375rem;
	border-radius: 0.3125rem;
	background: var(--surface-4);
	color: var(--color-base);
	font-size: 0.75rem;
	font-weight: 600;
}
.caption {
	font-size: 0.6875rem;
	font-weight: 600;
	text-align: center;
}
.progress-fill {
	position: absolute;
	inset: 0 auto 0 0;
	width: var(--_progress);
	background: color-mix(in srgb, var(--color-brand) 12%, transparent);
	pointer-events: none;
	transition: width 120ms ease-out;
}
</style>
