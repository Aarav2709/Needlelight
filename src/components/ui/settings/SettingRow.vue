<script setup lang="ts">
// one setting with its label and description on the left and its control on the right, the default slot spans the full width below
defineProps<{ title: string; description?: string; labelFor?: string }>()
</script>

<template>
	<div class="setting-row">
		<div class="flex min-w-0 items-center gap-4">
			<div class="flex min-w-0 flex-1 flex-col gap-0.5">
				<label v-if="labelFor" :for="labelFor" class="font-semibold text-contrast">{{ title }}</label>
				<span v-else class="font-semibold text-contrast">{{ title }}</span>
				<p v-if="description || $slots.description" class="m-0 text-sm leading-relaxed text-secondary">
					<slot name="description">{{ description }}</slot>
				</p>
			</div>
			<div v-if="$slots.control" class="flex shrink-0 items-center gap-2">
				<slot name="control" />
			</div>
		</div>
		<div v-if="$slots.default" class="mt-3 min-w-0">
			<slot />
		</div>
	</div>
</template>

<style scoped>
.setting-row {
	padding: 0.875rem 1rem;
}
.setting-row + .setting-row {
	border-top: 1px solid var(--color-divider);
}
</style>
