<script setup lang="ts">
import { Toggle } from '@modrinth/ui'

import SettingRow from '@/components/ui/settings/SettingRow.vue'
import SettingsGroup from '@/components/ui/settings/SettingsGroup.vue'
import { usePreferences } from '@/store/preferences'

const { prefs } = usePreferences()
</script>

<template>
	<div class="flex flex-col gap-6">
		<SettingsGroup
			title="Mods"
			note="Needlelight keeps dependencies working on its own: anything a mod needs is added with it, and turned back on if you enable the mod again."
		>
			<SettingRow
				title="Update mods before playing"
				description="When you press Play, install any available updates for the modpack first."
				label-for="pref-update-before-play"
			>
				<template #control>
					<Toggle id="pref-update-before-play" v-model="prefs.updateBeforePlay" />
				</template>
			</SettingRow>
			<SettingRow
				title="Confirm before uninstalling mods"
				description="Ask before a mod is uninstalled from a modpack. You're always asked when other mods need it."
				label-for="pref-confirm-remove"
			>
				<template #control>
					<Toggle id="pref-confirm-remove" v-model="prefs.confirmModRemoval" />
				</template>
			</SettingRow>
		</SettingsGroup>
	</div>
</template>
