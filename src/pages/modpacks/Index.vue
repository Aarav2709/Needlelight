<script setup lang="ts">
/** The Modpacks area. Modpacks themselves are listed in the app's sidebar (NavRail). */
import { injectNotificationManager } from '@modrinth/ui'
import { onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import { useShortcut } from '@/composables/shortcuts'
import { modpackRoute } from '@/helpers/modpacks'
import { useGames } from '@/store/games'
import { useModpacks } from '@/store/modpacks'

const route = useRoute()
const router = useRouter()
const games = useGames()
const modpacks = useModpacks()
const { handleError } = injectNotificationManager()

/** Ctrl+Alt+↑/↓: the previous or next modpack in the sidebar. */
function step(direction: 1 | -1) {
	const list = modpacks.ordered
	if (!list.length) return
	const current = list.findIndex((m) => m.path === String(route.params.path ?? ''))
	const next = list[(current + direction + list.length) % list.length]
	void router.push(modpackRoute(next))
}
useShortcut('mod+alt+arrowdown', () => step(1))
useShortcut('mod+alt+arrowup', () => step(-1))

onMounted(async () => {
	try {
		await Promise.all([modpacks.ensureLoaded(), games.ensureLoaded()])
	} catch (err) {
		handleError(err as Error)
	}
})
</script>

<template>
	<main class="h-full min-h-0 min-w-0">
		<RouterView />
	</main>
</template>
