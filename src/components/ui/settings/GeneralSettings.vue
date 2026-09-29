<script setup lang="ts">
import { BookOpenIcon, KeyboardIcon } from '@modrinth/assets'
import { Button, Toggle } from '@modrinth/ui'

import SettingRow from '@/components/ui/settings/SettingRow.vue'
import SettingsGroup from '@/components/ui/settings/SettingsGroup.vue'
import { usePreferences } from '@/store/preferences'
import { useUi } from '@/store/ui'

const { prefs } = usePreferences()
const ui = useUi()
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

		<SettingsGroup title="Help">
			<SettingRow
				title="Welcome guide"
				description="A quick walkthrough of games, modpacks and mods."
			>
				<template #control>
					<Button size="sm" @click="ui.showWelcome()"><BookOpenIcon /> Show guide</Button>
				</template>
			</SettingRow>
			<SettingRow title="Keyboard shortcuts" description="Press ? anywhere to see them.">
				<template #control>
					<Button size="sm" @click="ui.showShortcuts()"><KeyboardIcon /> Show shortcuts</Button>
				</template>
			</SettingRow>
		</SettingsGroup>
	</div>
</template>
