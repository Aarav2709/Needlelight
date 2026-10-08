<script setup lang="ts">
defineProps<{ title?: string; note?: string }>()
</script>

<template>
	<section class="flex flex-col gap-2">
		<h3 v-if="title" class="nl-section-label">{{ title }}</h3>
		<div class="nl-panel overflow-hidden">
			<slot />
		</div>
		<p v-if="note" class="m-0 text-xs leading-relaxed text-secondary">{{ note }}</p>
	</section>
</template>
