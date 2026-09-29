<script setup lang="ts">
/** /modpacks: reopens the last modpack used, or offers to make the first one. */
import { EmptyIllustration, PlusIcon, SpinnerIcon } from '@modrinth/assets'
import { Button } from '@modrinth/ui'
import { watch } from 'vue'
import { useRouter } from 'vue-router'

import { GAMES } from '@/helpers/games'
import { modpackRoute } from '@/helpers/modpacks'
import { useGames } from '@/store/games'
import { useModpacks } from '@/store/modpacks'
import { usePreferences } from '@/store/preferences'
import { useUi } from '@/store/ui'

const router = useRouter()
const games = useGames()
const modpacks = useModpacks()
const { prefs } = usePreferences()
const ui = useUi()

// Reopen the last modpack used for the selected game, else the first one in the sidebar.
watch(
	[() => modpacks.loaded, () => modpacks.ordered],
	([loaded, list]) => {
		if (!loaded || !list.length) return
		const remembered = prefs.lastModpack[games.activeGame]
		const target =
			list.find((m) => m.path === remembered) ??
			list.find((m) => m.game === games.activeGame) ??
			list[0]
		void router.replace(modpackRoute(target))
	},
	{ immediate: true },
)
</script>

<template>
	<div class="overview">
		<SpinnerIcon
			v-if="!modpacks.loaded || modpacks.list.length"
			class="h-6 w-6 animate-spin text-secondary"
		/>
		<div v-else class="flex max-w-md flex-col items-center gap-4 text-center">
			<EmptyIllustration class="w-44" aria-hidden="true" />
			<div class="flex flex-col gap-2">
				<h1 class="m-0 text-2xl font-extrabold tracking-tight text-contrast">
					Create your first modpack
				</h1>
				<p class="m-0 text-[0.9375rem] leading-relaxed text-secondary">
					A modpack is a set of mods you can play in one click. Make one for each way you like to
					play; each keeps its own mods and versions.
				</p>
			</div>
			<div class="flex flex-wrap justify-center gap-2">
				<Button
					v-for="game in GAMES"
					:key="game.key"
					:type="game.key === games.activeGame ? 'colored' : 'base'"
					:color="game.key === games.activeGame ? 'brand' : undefined"
					size="lg"
					@click="ui.createModpack(game.key)"
				>
					<PlusIcon /> {{ game.name }}
				</Button>
			</div>
		</div>
	</div>
</template>

<style scoped>
.overview {
	display: flex;
	align-items: center;
	justify-content: center;
	height: 100%;
	padding: 2rem;
	background: radial-gradient(
		40rem 22rem at 50% 38%,
		color-mix(in srgb, var(--color-brand) 8%, transparent),
		transparent 70%
	);
}
</style>
