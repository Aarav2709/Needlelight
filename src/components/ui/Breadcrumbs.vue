<template>
	<nav data-tauri-drag-region class="flex min-w-0 items-center gap-1 pl-3" aria-label="Breadcrumb">
		{{ breadcrumbData.resetToNames(breadcrumbs) }}
		<template v-for="(breadcrumb, index) in breadcrumbs" :key="breadcrumb.name">
			<router-link
				v-if="breadcrumb.link"
				:to="target(breadcrumb.link)"
				class="crumb min-w-0 shrink truncate text-secondary hover:text-contrast"
				:title="label(breadcrumb)"
				>{{ label(breadcrumb) }}</router-link
			>
			<span
				v-else
				data-tauri-drag-region
				class="crumb min-w-0 cursor-default select-none truncate font-semibold text-contrast"
				:title="label(breadcrumb)"
				:aria-current="index === breadcrumbs.length - 1 ? 'page' : undefined"
				>{{ label(breadcrumb) }}</span
			>
			<ChevronRightIcon
				v-if="index < breadcrumbs.length - 1"
				data-tauri-drag-region
				class="h-4 w-4 shrink-0 text-secondary"
				aria-hidden="true"
			/>
		</template>
	</nav>
</template>

<script setup>
import { ChevronRightIcon } from '@modrinth/assets'
import { computed } from 'vue'
import { useRoute } from 'vue-router'

import { useBreadcrumbs } from '@/store/breadcrumbs'

const route = useRoute()

const breadcrumbData = useBreadcrumbs()
const breadcrumbs = computed(() => route.meta.breadcrumb ?? [])

// a .. link goes to the parent page, like from browse back to the modpack
function target(link) {
	return link === '..' ? route.path.replace(/\/[^/]+\/?$/, '') : link
}

function label(breadcrumb) {
	return breadcrumb.name.charAt(0) === '?'
		? breadcrumbData.getName(breadcrumb.name.slice(1)) || '…'
		: breadcrumb.name
}
</script>

<style scoped>
.crumb {
	max-width: 28rem;
	font-size: 0.9375rem;
	text-decoration: none;
	transition: color 0.12s ease;
}
</style>
