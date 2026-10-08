<script setup lang="ts">
import { CircleAlertIcon, RefreshCwIcon, SpinnerIcon, TrashIcon } from '@modrinth/assets'
import { Checkbox, Toggle } from '@modrinth/ui'
import { computed } from 'vue'

import { useModpackContext } from '@/composables/modpack'
import {
	authorLine,
	dependentsOf,
	formatVersion,
	installedVersion,
	isEnabled,
	isUntracked,
	listNames,
	type ModEntry,
	modIssues,
	needsUpdate,
	shortAuthorLine,
} from '@/helpers/mods'

const props = defineProps<{ mod: ModEntry; selected?: boolean; checked?: boolean }>()
const emit = defineEmits<{ open: []; check: [event?: MouseEvent] }>()

const ctx = useModpackContext()

const name = computed(() => ctx.label(props.mod.name))
const author = computed(() => shortAuthorLine(props.mod, ctx.game.value))
const allAuthors = computed(() => authorLine(props.mod, ctx.game.value))
const enabled = computed(() => isEnabled(props.mod))
const current = computed(() => installedVersion(props.mod))
const hasUpdate = computed(() => needsUpdate(props.mod))
const busy = computed(() => ctx.isBusy(props.mod.name))
const progress = computed(() => ctx.progressFor(props.mod.name))

const issueText = computed(() => {
	const issues = modIssues(props.mod, ctx.index.value)
	if (!issues.length) return null
	const names = (kind: string) => issues.filter((i) => i.kind === kind).map((i) => ctx.label(i.dependency))
	const parts: string[] = []
	const missing = [...names('missing'), ...names('unavailable')]
	if (missing.length) parts.push(`${listNames(missing)} ${missing.length === 1 ? 'is' : 'are'} missing`)
	const disabled = names('disabled')
	if (disabled.length) parts.push(`${listNames(disabled)} ${disabled.length === 1 ? 'is' : 'are'} disabled`)
	const outdated = names('outdated')
	if (outdated.length) parts.push(`${listNames(outdated)} ${outdated.length === 1 ? 'needs' : 'need'} an update`)
	return `Won't load: ${parts.join(', ')}`
})

const detail = computed(() => {
	if (isUntracked(props.mod)) return 'Installed outside Needlelight'
	if (!props.mod.inCatalog) return 'Added manually'
	const usedBy = dependentsOf(props.mod.name, ctx.installed.value).map((m) => ctx.label(m.name))
	if (usedBy.length) {
		const shown = usedBy.slice(0, 2)
		const more = usedBy.length - shown.length
		return `Used by ${listNames(more ? [...shown, `${more} more`] : shown)}`
	}
	return props.mod.description
})
</script>

<template>
	<div
		class="mod-row"
		:class="{ 'is-disabled': !enabled, 'is-selected': selected, 'is-checked': checked }"
		:style="progress !== undefined ? { '--_progress': `${progress}%` } : undefined"
		role="row"
		tabindex="0"
		:aria-label="`${name}, ${enabled ? 'enabled' : 'disabled'}. View details`"
		:aria-current="selected || undefined"
		@click="emit('open')"
		@keydown.enter.self="emit('open')"
		@keydown.space.self.prevent="emit('open')"
	>
		<div v-if="progress !== undefined" class="progress-fill" aria-hidden="true" />

		<div class="relative flex items-center" role="cell" @click.stop @keydown.stop>
			<Checkbox
				:model-value="!!checked"
				:description="`Select ${name}`"
				@update:model-value="(_value: boolean, event?: MouseEvent) => emit('check', event)"
			/>
		</div>

		<div class="relative flex min-w-0 flex-col gap-0.5" role="cell">
			<div class="flex min-w-0 items-baseline gap-2">
				<span class="name" :title="name">{{ name }}</span>
				<span v-if="author" class="author" :title="allAuthors">by {{ author }}</span>
			</div>
			<div class="flex min-w-0 items-center gap-1.5 text-[0.8125rem]">
				<span v-if="busy" class="text-brand">
					{{ progress !== undefined ? `Downloading… ${Math.round(progress)}%` : 'Working…' }}
				</span>
				<template v-else-if="issueText">
					<CircleAlertIcon class="h-3.5 w-3.5 shrink-0 text-red" aria-hidden="true" />
					<span class="truncate text-red" :title="issueText">{{ issueText }}</span>
				</template>
				<span v-else-if="detail" class="truncate text-secondary" :title="detail">{{ detail }}</span>
			</div>
		</div>

		<div class="version relative" role="cell" @click.stop @keydown.stop>
			<template v-if="hasUpdate">
				<span class="truncate text-secondary">{{ formatVersion(current) }}</span>
				<button
					v-tooltip="`Update to ${formatVersion(mod.version)}`"
					type="button"
					class="update"
					:disabled="busy"
					:aria-label="`Update ${name} to ${formatVersion(mod.version)}`"
					@click="ctx.update([mod.name])"
				>
					<RefreshCwIcon class="h-3.5 w-3.5" /> {{ formatVersion(mod.version) }}
				</button>
			</template>
			<span v-else-if="current" class="truncate text-secondary">{{ formatVersion(current) }}</span>
		</div>

		<div class="relative flex items-center justify-end gap-1" role="cell" @click.stop @keydown.stop>
			<span class="flex w-12 justify-center">
				<SpinnerIcon v-if="busy" class="h-5 w-5 animate-spin text-secondary" aria-label="Working" />
				<Toggle
					v-else
					v-tooltip="enabled ? 'Disable' : 'Enable'"
					:model-value="enabled"
					:aria-label="`${enabled ? 'Disable' : 'Enable'} ${name}`"
					@update:model-value="(value) => ctx.setEnabled(mod.name, !!value)"
				/>
			</span>
			<button
				v-tooltip="'Uninstall'"
				type="button"
				class="nl-icon-btn nl-icon-btn--danger"
				:disabled="busy"
				:aria-label="`Uninstall ${name}`"
				@click="ctx.uninstall(mod.name)"
			>
				<TrashIcon />
			</button>
		</div>
	</div>
</template>

<style scoped>
.mod-row {
	position: relative;
	display: grid;
	grid-template-columns: 1.25rem minmax(0, 1fr) 10rem 6.25rem;
	align-items: center;
	gap: 1rem;
	min-height: 3.75rem;
	padding: calc(var(--nl-row-py) - 0.125rem) var(--nl-row-px);
	border-radius: 0.625rem;
	overflow: hidden;
	cursor: pointer;
	transition: background-color 0.12s ease;
}
.mod-row:hover {
	background: var(--surface-3);
}
.mod-row:focus-visible {
	outline: 2px solid var(--color-brand);
	outline-offset: -2px;
}
.mod-row.is-checked {
	background: color-mix(in srgb, var(--color-brand) 7%, transparent);
}
.mod-row.is-selected {
	background: color-mix(in srgb, var(--color-brand) 13%, var(--surface-2));
	box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--color-brand) 35%, transparent);
}
/* the name keeps its room and a long author list gives way first */
.name {
	flex: 0 1 auto;
	min-width: 0;
	overflow: hidden;
	text-overflow: ellipsis;
	white-space: nowrap;
	font-size: 0.9375rem;
	font-weight: 700;
	color: var(--color-contrast);
}
.author {
	flex: 0 100 auto;
	min-width: 0;
	overflow: hidden;
	text-overflow: ellipsis;
	white-space: nowrap;
	font-size: 0.8125rem;
	color: var(--color-secondary);
}
.is-disabled .name {
	color: var(--color-secondary);
}
.version {
	display: flex;
	align-items: center;
	gap: 0.5rem;
	min-width: 0;
	font-size: 0.8125rem;
	font-variant-numeric: tabular-nums;
}
.update {
	display: inline-flex;
	align-items: center;
	gap: 0.25rem;
	flex-shrink: 0;
	height: 1.625rem;
	padding: 0 0.5rem;
	border: 1px solid color-mix(in srgb, var(--color-orange) 45%, transparent);
	border-radius: 0.4375rem;
	background: color-mix(in srgb, var(--color-orange) 13%, transparent);
	color: var(--color-orange);
	font-size: 0.75rem;
	font-weight: 700;
	cursor: pointer;
}
.update:hover:not(:disabled) {
	background: color-mix(in srgb, var(--color-orange) 24%, transparent);
}
.update:focus-visible {
	outline: 2px solid var(--color-brand);
	outline-offset: 1px;
}
.update:disabled {
	opacity: 0.5;
	cursor: default;
}
@container modlist (max-width: 42rem) {
	.mod-row {
		grid-template-columns: 1.25rem minmax(0, 1fr) 6.25rem;
	}
	.version {
		display: none;
	}
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
