<script setup lang="ts">
import { Toggle } from '@modrinth/ui'
import { onMounted, ref, watch } from 'vue'

import { get as getSettings, set as setSettings } from '@/helpers/settings.ts'
import { useTheming } from '@/store/state'
import { DEFAULT_FEATURE_FLAGS, type FeatureFlag } from '@/store/theme.ts'

const themeStore = useTheming()

const settings = ref<Record<string, any> | null>(null)
const ready = ref(false)

onMounted(async () => {
	try { settings.value = await getSettings() } catch { /* ignore */ }
	ready.value = true
})
const options = ref<FeatureFlag[]>(Object.keys(DEFAULT_FEATURE_FLAGS))

function setFeatureFlag(key: string, value: boolean) {
	themeStore.featureFlags[key] = value
	settings.value.feature_flags[key] = value
}

watch(
	settings,
	async (val) => {
		if (val) await setSettings(val)
	},
	{ deep: true },
)
</script>
<template>
	<div v-if="!ready" class="text-secondary text-sm p-4">Loading feature flags...</div>
	<div v-else-if="settings">
	<div v-for="option in options" :key="option" class="mt-4 flex items-center justify-between">
		<div>
			<h2 class="m-0 text-lg font-extrabold text-contrast capitalize">
				{{ option.replaceAll('_', ' ') }}
			</h2>
		</div>

		<Toggle
			id="advanced-rendering"
			:model-value="themeStore.getFeatureFlag(option)"
			@update:model-value="() => setFeatureFlag(option, !themeStore.getFeatureFlag(option))"
		/>
	</div>
	</div>
</template>
