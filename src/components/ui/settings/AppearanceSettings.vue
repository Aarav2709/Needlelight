<script setup lang="ts">
import { Toggle } from '@modrinth/ui'
import { usePreferredDark } from '@vueuse/core'

import FilterChips from '@/components/ui/FilterChips.vue'
import SettingRow from '@/components/ui/settings/SettingRow.vue'
import SettingsGroup from '@/components/ui/settings/SettingsGroup.vue'
import ThemeSelector from '@/components/ui/settings/ThemeSelector.vue'
import { usePreferences } from '@/store/preferences'
import { THEME_OPTIONS } from '@/store/theme'

const { prefs } = usePreferences()
const prefersDark = usePreferredDark()
</script>

<template>
	<div class="flex flex-col gap-6">
		<section class="flex flex-col gap-2">
			<h3 class="nl-section-label">Theme</h3>
			<ThemeSelector
				class="!mt-0"
				:update-color-theme="(theme) => (prefs.theme = theme)"
				:current-theme="prefs.theme"
				:theme-options="THEME_OPTIONS"
				:system-theme-color="prefersDark ? 'dark' : 'light'"
			/>
			<p class="m-0 text-xs text-secondary">
				Accents follow the game you're working on: purple for Hollow Knight, red for Hollow Knight:
				Silksong.
			</p>
		</section>

		<SettingsGroup title="Layout and motion">
			<SettingRow
				title="Density"
				description="Compact fits more mods on screen by tightening lists and margins."
			>
				<template #control>
					<FilterChips
						v-model="prefs.density"
						label="Density"
						:options="[
							{ value: 'comfortable', label: 'Comfortable' },
							{ value: 'compact', label: 'Compact' },
						]"
					/>
				</template>
			</SettingRow>
			<SettingRow
				title="Reduce motion"
				description="Turn off animations and transitions. Your system's reduced-motion setting is always respected."
				label-for="pref-motion"
			>
				<template #control>
					<Toggle id="pref-motion" v-model="prefs.reduceMotion" />
				</template>
			</SettingRow>
		</SettingsGroup>
	</div>
</template>
