<script setup lang="ts">
import { Toggle } from '@modrinth/ui'

import SettingRow from '@/components/ui/settings/SettingRow.vue'
import SettingsGroup from '@/components/ui/settings/SettingsGroup.vue'
import { usePreferences } from '@/store/preferences'

const { prefs } = usePreferences()
</script>

<template>
	<div class="flex flex-col gap-6">
		<SettingsGroup title="Playing">
			<SettingRow
				title="Minimize when a game starts"
				description="Tuck Needlelight away after you press Play."
				label-for="pref-minimize"
			>
				<template #control>
					<Toggle id="pref-minimize" v-model="prefs.minimizeOnLaunch" />
				</template>
			</SettingRow>
		</SettingsGroup>
	</div>
</template>
