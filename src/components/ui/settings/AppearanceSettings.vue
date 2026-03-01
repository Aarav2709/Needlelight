<script setup lang="ts">
import { ThemeSelector, Toggle } from '@modrinth/ui'
import { onMounted, ref, watch } from 'vue'

import { get, set } from '@/helpers/settings.ts'
import { getOS } from '@/helpers/utils'
import { useTheming } from '@/store/state'
import type { ColorTheme } from '@/store/theme.ts'

const themeStore = useTheming()

const os = ref('')
const settings = ref<Record<string, any> | null>(null)
const ready = ref(false)

onMounted(async () => {
	try { os.value = await getOS() } catch { /* ignore */ }
	try { settings.value = await get() } catch { /* ignore */ }
	ready.value = true
})

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
		:update-color-theme="
			(theme: ColorTheme) => {
				themeStore.setThemeState(theme)
				settings.theme = theme
			}
		"
		:current-theme="settings.theme"
		:theme-options="themeStore.getThemeOptions()"
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
			@update:model-value="
				(e) => {
					themeStore.advancedRendering = !!e
					settings.advanced_rendering = themeStore.advancedRendering
				}
			"
		/>
	</div>

	<div v-if="os !== 'MacOS'" class="mt-4 flex items-center justify-between gap-4">
		<div>
			<h2 class="m-0 text-lg font-extrabold text-contrast">Native decorations</h2>
			<p class="m-0 mt-1">Use system window frame (app restart required).</p>
		</div>
		<Toggle id="native-decorations" v-model="settings.native_decorations" />
	</div>

	<div class="mt-4 flex items-center justify-between">
		<div>
			<h2 class="m-0 text-lg font-extrabold text-contrast">Minimize launcher</h2>
			<p class="m-0 mt-1">Minimize the launcher when the game starts.</p>
		</div>
		<Toggle id="minimize-launcher" v-model="settings.hide_on_process_start" />
	</div>

	<div class="mt-4 flex items-center justify-between">
		<div>
			<h2 class="m-0 text-lg font-extrabold text-contrast">Toggle sidebar</h2>
			<p class="m-0 mt-1">Enables the ability to toggle the sidebar.</p>
		</div>
		<Toggle
			id="toggle-sidebar"
			:model-value="settings.toggle_sidebar"
			@update:model-value="
				(e) => {
					settings.toggle_sidebar = !!e
					themeStore.toggleSidebar = settings.toggle_sidebar
				}
			"
		/>
	</div>
	</div>
</template>
