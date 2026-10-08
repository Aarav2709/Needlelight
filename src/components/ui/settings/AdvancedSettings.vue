<script setup lang="ts">
import { FileTextIcon, FolderOpenIcon, UndoIcon } from '@modrinth/assets'
import { Button, injectNotificationManager, Toggle } from '@modrinth/ui'

import SettingRow from '@/components/ui/settings/SettingRow.vue'
import SettingsGroup from '@/components/ui/settings/SettingsGroup.vue'
import { type AppFolder, openAppFolder } from '@/helpers/app'
import { usePreferences } from '@/store/preferences'

const preferences = usePreferences()
const { prefs } = preferences
const { addNotification, handleError } = injectNotificationManager()

async function open(folder: AppFolder) {
	try {
		await openAppFolder(folder)
	} catch (err) {
		handleError(err as Error)
	}
}

function resetPreferences() {
	preferences.reset()
	addNotification({
		title: 'Settings reset',
		text: 'Appearance and behavior are back to their defaults. Your modpacks and mods are unchanged.',
		type: 'success',
	})
}
</script>

<template>
	<div class="flex flex-col gap-6">
		<SettingsGroup title="Performance">
			<SettingRow
				title="Blur effects"
				description="Frosted-glass effects behind dialogs and menus. Turn off if Needlelight feels slow on your computer."
				label-for="pref-blur"
			>
				<template #control>
					<Toggle id="pref-blur" v-model="prefs.advancedRendering" />
				</template>
			</SettingRow>
		</SettingsGroup>

		<SettingsGroup title="Files">
			<SettingRow title="Data folder" description="Settings, logs and everything else Needlelight saves.">
				<template #control>
					<Button size="sm" @click="open('data')"><FolderOpenIcon /> Open</Button>
				</template>
			</SettingRow>
			<SettingRow title="Modpacks folder" description="Each modpack is a folder in here, with its own mods.">
				<template #control>
					<Button size="sm" @click="open('modpacks')"><FolderOpenIcon /> Open</Button>
				</template>
			</SettingRow>
			<SettingRow
				title="Install log"
				description="A record of installs and launches. Attach it when you report a problem."
			>
				<template #control>
					<Button size="sm" @click="open('install_log')"><FileTextIcon /> Show</Button>
				</template>
			</SettingRow>
		</SettingsGroup>

		<SettingsGroup title="Reset">
			<SettingRow
				title="Reset settings"
				description="Put appearance and behavior settings back to their defaults. Modpacks, mods and games aren't affected."
			>
				<template #control>
					<Button size="sm" @click="resetPreferences"><UndoIcon /> Reset</Button>
				</template>
			</SettingRow>
		</SettingsGroup>
	</div>
</template>
