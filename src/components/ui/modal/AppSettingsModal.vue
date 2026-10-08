<script setup lang="ts">
import {
	CodeIcon,
	GameIcon,
	LayersIcon,
	PaintbrushIcon,
	RefreshCwIcon,
	Settings2Icon,
	SettingsIcon,
} from '@modrinth/assets'
import { Button, defineMessage, ProgressBar, TabbedModal } from '@modrinth/ui'
import { computed, ref } from 'vue'

import AdvancedSettings from '@/components/ui/settings/AdvancedSettings.vue'
import AppearanceSettings from '@/components/ui/settings/AppearanceSettings.vue'
import GameSettings from '@/components/ui/settings/GameSettings.vue'
import GeneralSettings from '@/components/ui/settings/GeneralSettings.vue'
import ModpackSettings from '@/components/ui/settings/ModpackSettings.vue'
import { APP_VERSION } from '@/helpers/version'
import type { SettingsTab } from '@/store/ui'
import { useUpdater } from '@/store/updater'

// modrinth's tabbed modal already wraps a modal, so it is used directly to avoid stacking two overlays
const updater = useUpdater()

const tabs = [
	{
		key: 'general' as SettingsTab,
		name: defineMessage({ id: 'app.settings.tabs.general', defaultMessage: 'General' }),
		icon: Settings2Icon,
		content: GeneralSettings,
	},
	{
		key: 'appearance' as SettingsTab,
		name: defineMessage({ id: 'app.settings.tabs.appearance', defaultMessage: 'Appearance' }),
		icon: PaintbrushIcon,
		content: AppearanceSettings,
	},
	{
		key: 'games' as SettingsTab,
		name: defineMessage({ id: 'app.settings.tabs.games', defaultMessage: 'Games' }),
		icon: GameIcon,
		content: GameSettings,
	},
	{
		key: 'modpacks' as SettingsTab,
		name: defineMessage({ id: 'app.settings.tabs.modpacks', defaultMessage: 'Modpacks' }),
		icon: LayersIcon,
		content: ModpackSettings,
	},
	{
		key: 'advanced' as SettingsTab,
		name: defineMessage({ id: 'app.settings.tabs.advanced', defaultMessage: 'Advanced' }),
		icon: CodeIcon,
		content: AdvancedSettings,
	},
]

const modal = ref<InstanceType<typeof TabbedModal> | null>(null)

function show(tab?: SettingsTab) {
	modal.value?.show()
	const index = tab ? tabs.findIndex((t) => t.key === tab) : -1
	if (index >= 0) modal.value?.setTab(index)
}

function hide() {
	modal.value?.hide()
}

defineExpose({ show, hide })

const updateLine = computed(() => {
	switch (updater.status) {
		case 'checking':
			return 'Checking for updates…'
		case 'none':
			return "You're up to date"
		case 'downloading':
			return `Downloading ${updater.version}`
		case 'ready':
			return `${updater.version} is ready to install`
		case 'installing':
			return 'Restarting…'
		case 'error':
			return "Couldn't install the update"
		default:
			return null
	}
})
</script>
<template>
	<TabbedModal ref="modal" :tabs="tabs" width="min(54rem, calc(100vw - 6rem))">
		<template #title>
			<span class="flex items-center gap-2 text-xl font-extrabold text-contrast">
				<SettingsIcon class="h-5 w-5" /> Settings
			</span>
		</template>
		<template #footer>
			<div class="mt-auto flex flex-col gap-3 pt-3 text-sm text-secondary">
				<ProgressBar
					v-if="updater.status === 'downloading' && updater.progress != null"
					:progress="updater.progress"
					class="px-4"
				/>
				<div class="flex flex-col gap-2 px-4">
					<div class="flex items-center gap-2">
						<span class="font-bold text-contrast">Needlelight</span>
						<span class="nl-badge nl-badge--neutral">v{{ APP_VERSION }}</span>
					</div>
					<p v-if="updateLine" class="m-0 text-xs" :title="updater.error ?? undefined">
						{{ updateLine }}
					</p>
					<Button
						v-if="updater.status === 'ready'"
						size="sm"
						color="brand"
						type="colored"
						@click="updater.installAndRestart()"
					>
						<RefreshCwIcon /> Restart to update
					</Button>
				</div>
			</div>
		</template>
	</TabbedModal>
</template>
