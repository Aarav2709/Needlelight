<script setup lang="ts">
import { Toggle } from '@modrinth/ui'
import { onMounted, ref, watch } from 'vue'

import { type AppSettings, get, set } from '@/helpers/settings.ts'
import { useTheming } from '@/store/state'
import type { ColorTheme } from '@/store/theme.ts'

import ThemeSelector from './ThemeSelector.vue'

const themeStore = useTheming()

const settings = ref<AppSettings | null>(null)
const ready = ref(false)

onMounted(async () => {
	try { settings.value = await get() } catch { /* ignore */ }
	if (settings.value?.theme === 'system') {
		settings.value.theme = 'dark'
		themeStore.setThemeState('dark')
	}
	ready.value = true
})

function updateColorTheme(theme: ColorTheme) {
	themeStore.setThemeState(theme)
	if (settings.value) settings.value.theme = theme
}

function updateAdvancedRendering(enabled: boolean) {
	themeStore.advancedRendering = enabled
	if (settings.value) settings.value.advanced_rendering = enabled
}

watch(
	settings,
	async (val) => {
		if (val) await set(val)
	},
	{ deep: true },
)
</script>
<template>
	<div v-if="!ready" class="text-secondary text-sm p-4">Loading appearance settings...</div>
	<div v-else-if="settings">
	<h2 class="m-0 text-lg font-extrabold text-contrast">Color theme</h2>
	<p class="m-0 mt-1">Select your preferred color theme for Needlelight.</p>

	<ThemeSelector
		:update-color-theme="updateColorTheme"
		:current-theme="settings.theme"
		:theme-options="themeStore.getThemeOptions().filter((theme) => theme !== 'system')"
		system-theme-color="system"
	/>

	<div class="mt-4 flex items-center justify-between">
		<div>
			<h2 class="m-0 text-lg font-extrabold text-contrast">Advanced rendering</h2>
			<p class="m-0 mt-1">
				Enables advanced rendering such as blur effects that may cause performance issues without
				hardware-accelerated rendering.
			</p>
		</div>

		<Toggle
			id="advanced-rendering"
			:model-value="themeStore.advancedRendering"
			@update:model-value="(e) => updateAdvancedRendering(!!e)"
		/>
	</div>

	<div class="mt-4 flex items-center justify-between">
		<div>
			<h2 class="m-0 text-lg font-extrabold text-contrast">Minimize launcher</h2>
			<p class="m-0 mt-1">Minimize the launcher when the game starts.</p>
		</div>
		<Toggle id="minimize-launcher" v-model="settings.hide_on_process_start" />
	</div>

	</div>
</template>
