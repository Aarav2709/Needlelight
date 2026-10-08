<script setup lang="ts" generic="T extends string">
// a row of single choice filter chips, the selected one takes the active game's accent
defineProps<{
	options: { value: T; label: string; count?: number; disabled?: boolean }[]
	label: string
}>()

const model = defineModel<T>({ required: true })
</script>

<template>
	<div class="chips" role="group" :aria-label="label">
		<button
			v-for="option in options"
			:key="option.value"
			type="button"
			class="chip"
			:aria-pressed="model === option.value"
			:disabled="option.disabled"
			@click="model = option.value"
		>
			<span class="truncate">{{ option.label }}</span>
			<span v-if="option.count !== undefined" class="count">{{ option.count }}</span>
		</button>
	</div>
</template>

<style scoped>
.chips {
	display: flex;
	flex-wrap: wrap;
	gap: 0.375rem;
	min-width: 0;
}
.chip {
	display: inline-flex;
	align-items: center;
	gap: 0.4375rem;
	max-width: 16rem;
	height: 2rem;
	padding: 0 0.75rem;
	border-radius: 0.5rem;
	border: 1px solid var(--color-divider);
	background: var(--surface-2);
	color: var(--color-base);
	font-size: 0.8125rem;
	font-weight: 600;
	white-space: nowrap;
	cursor: pointer;
	transition:
		background-color 0.14s ease,
		border-color 0.14s ease,
		color 0.14s ease,
		transform 0.08s ease;
}
.chip:hover:not(:disabled):not([aria-pressed='true']) {
	background: var(--surface-3);
	border-color: var(--surface-5);
	color: var(--color-contrast);
}
.chip:active:not(:disabled) {
	transform: scale(0.97);
}
.chip:focus-visible {
	outline: 2px solid var(--color-brand);
	outline-offset: 2px;
}
.chip[aria-pressed='true'] {
	background: var(--color-brand);
	border-color: var(--color-brand);
	color: var(--color-accent-contrast);
}
.chip[aria-pressed='true']:hover:not(:disabled) {
	filter: brightness(1.08);
}
.chip:disabled {
	opacity: 0.45;
	cursor: not-allowed;
}
.count {
	font-size: 0.75rem;
	font-variant-numeric: tabular-nums;
	opacity: 0.65;
}
.chip[aria-pressed='true'] .count {
	opacity: 0.8;
}
</style>
