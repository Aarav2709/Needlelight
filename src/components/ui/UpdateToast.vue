<script setup lang="ts">
// tells the player a downloaded update is ready and restarts into it
import { ExternalIcon, RefreshCwIcon, SpinnerIcon, XIcon } from '@modrinth/assets'
import { Button, IconButton } from '@modrinth/ui'

import { openExternal, RELEASES_URL } from '@/helpers/app'
import { useUpdater } from '@/store/updater'

const updater = useUpdater()
</script>

<template>
	<div
		class="fixed right-6 top-[--top-bar-height] z-10 mt-6 grid w-[25rem] rounded-2xl border-[2px] border-solid border-surface-5 bg-bg-raised p-4 card-shadow"
		role="status"
	>
		<div class="flex items-center gap-4">
			<h2 class="m-0 grow text-base font-semibold text-contrast">Update ready</h2>
			<IconButton v-tooltip="'Close'" size="sm" label="Close" @click="updater.dismissed = true">
				<XIcon />
			</IconButton>
		</div>
		<p class="mb-0 mt-2 text-sm">
			Needlelight {{ updater.version }} has downloaded. Restart now to finish updating, your
			modpacks stay as they are.
		</p>
		<div class="mt-4 flex gap-2">
			<Button
				type="colored"
				color="brand"
				:disabled="updater.status === 'installing'"
				@click="updater.installAndRestart()"
			>
				<SpinnerIcon v-if="updater.status === 'installing'" class="animate-spin" />
				<RefreshCwIcon v-else />
				{{ updater.status === 'installing' ? 'Restarting…' : 'Restart now' }}
			</Button>
			<Button @click="openExternal(`${RELEASES_URL}/latest`)">
				What's new <ExternalIcon />
			</Button>
		</div>
	</div>
</template>
