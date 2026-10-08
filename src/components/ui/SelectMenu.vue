<script setup lang="ts" generic="T extends string">
// a compact dropdown for filters and sorting, an option's sub label shows as a count on the right
import { CheckIcon } from '@modrinth/assets'
import { Combobox, type ComboboxOption } from '@modrinth/ui'
import type { Component } from 'vue'

const model = defineModel<T>({ required: true })
defineProps<{ label?: string; options: ComboboxOption<T>[]; icon?: Component }>()
</script>

<template>
	<div class="select-menu" :aria-label="label">
		<Combobox
			v-model="model"
			:options="options"
			trigger-class="select-trigger"
			dropdown-class="nl-select-dropdown"
			:dropdown-min-width="220"
		>
			<template #prefix>
				<component :is="icon" v-if="icon" class="h-4 w-4 shrink-0 text-secondary" aria-hidden="true" />
				<span v-if="label" class="shrink-0 font-medium text-secondary">{{ label }}</span>
			</template>
			<template #option="{ item, isSelected }">
				<span class="flex w-full items-center gap-3">
					<span class="min-w-0 flex-1 truncate font-semibold" :class="isSelected ? 'text-brand' : 'text-contrast'">
						{{ item.label }}
					</span>
					<span v-if="item.subLabel" class="text-sm font-semibold text-secondary">{{ item.subLabel }}</span>
					<CheckIcon v-if="isSelected" class="h-4 w-4 shrink-0 text-brand" aria-hidden="true" />
					<span v-else class="w-4 shrink-0" aria-hidden="true" />
				</span>
			</template>
		</Combobox>
	</div>
</template>

<style scoped>
.select-menu {
	flex-shrink: 0;
}
.select-menu :deep(.select-trigger) {
	height: 2.5rem;
	gap: 0.75rem;
	padding: 0 0.625rem 0 0.75rem;
	border-radius: var(--nl-control-radius);
	border: 1px solid var(--color-divider);
	background: var(--surface-2);
	font-size: 0.875rem;
}
.select-menu :deep(.select-trigger:hover) {
	background: var(--surface-3);
}
</style>

<style>
/* the list is teleported, so use the game accent for the chosen option instead of modrinth green */
.nl-select-dropdown [role='option'] {
	padding: 0.625rem 0.875rem !important;
}
.nl-select-dropdown [role='option'][aria-selected='true'] {
	background: color-mix(in srgb, var(--color-brand) 14%, var(--surface-4)) !important;
}
</style>
