<script setup lang="ts">
import { CheckIcon, ChevronRightIcon, FolderSearchIcon, SpinnerIcon } from '@modrinth/assets'
import { Button, injectNotificationManager } from '@modrinth/ui'
import { open } from '@tauri-apps/plugin-dialog'
import { onMounted, reactive } from 'vue'

import { type GameInfo, type GameKey, GAMES, modpackCount } from '@/helpers/games'
import { useGames } from '@/store/games'
import { useModpacks } from '@/store/modpacks'

const games = useGames()
const modpacks = useModpacks()
const { handleError, addNotification } = injectNotificationManager()

// a folder the player picked that didn't contain the game
const locateError = reactive<Partial<Record<GameKey, string>>>({})

onMounted(async () => {
	try {
		await Promise.all([games.load(), modpacks.ensureLoaded()])
	} catch (err) {
		handleError(err as Error)
		return
	}
	// look for any game that isn't found yet, once per session
	for (const game of GAMES) if (games.found[game.key] === false) void games.findGame(game.key)
})

function status(game: GameInfo) {
	if (games.searching.has(game.key)) return 'Looking for the game…'
	if (games.found[game.key] === false) return 'Not found on this computer'
	if (games.found[game.key] === null) return 'Checking…'
	return 'Installed'
}

async function choose(game: GameKey) {
	try {
		await games.switchGame(game)
	} catch (err) {
		handleError(err as Error)
	}
}

async function locate(game: GameInfo) {
	const folder = await open({ directory: true, title: `Where is ${game.name} installed?` })
	if (typeof folder !== 'string') return
	Reflect.deleteProperty(locateError, game.key)
	try {
		if (await games.locateGame(game.key, folder)) {
			addNotification({ title: `Found ${game.name}`, type: 'success' })
		} else {
			locateError[game.key] = `That folder doesn't contain ${game.name}. Choose the folder the game is installed in.`
		}
	} catch (err) {
		handleError(err as Error)
	}
}

const HOLLOW_KNIGHT_STEPS = [
	'In Steam, right-click Hollow Knight and choose Properties.',
	'Open Betas (or Game Versions and Betas).',
	'Choose 1.5.78.11833 and let Steam finish updating.',
]
</script>

<template>
	<div class="flex flex-col gap-5">
		<p class="m-0 text-sm leading-relaxed text-secondary">
			Needlelight finds your games automatically. The active game is the one whose modpacks you're
			working on.
		</p>

		<div class="flex flex-col gap-3" role="radiogroup" aria-label="Active game">
			<section
				v-for="game in GAMES"
				:key="game.key"
				class="game-card"
				:class="{ 'is-active': games.activeGame === game.key }"
			>
				<button
					type="button"
					role="radio"
					class="game-select"
					:aria-checked="games.activeGame === game.key"
					:disabled="!!games.switching"
					@click="choose(game.key)"
				>
					<span class="radio" aria-hidden="true">
						<CheckIcon v-if="games.activeGame === game.key" class="h-3.5 w-3.5" />
					</span>
					<span class="flex min-w-0 flex-1 flex-col gap-0.5 text-left">
						<span class="truncate text-base font-bold text-contrast">{{ game.name }}</span>
						<span class="flex min-w-0 items-center gap-1.5 text-sm text-secondary">
							<SpinnerIcon v-if="games.searching.has(game.key)" class="h-3.5 w-3.5 animate-spin" />
							<span v-else-if="games.found[game.key] === false" class="nl-dot text-orange" />
							<span class="truncate">
								{{ status(game) }} · {{ modpackCount(modpacks.forGame(game.key).length) }}
							</span>
						</span>
					</span>
					<span
						class="nl-badge"
						:class="games.activeGame === game.key ? 'nl-badge--brand' : 'nl-badge--neutral'"
					>
						<SpinnerIcon v-if="games.switching === game.key" class="animate-spin" />
						{{ games.activeGame === game.key ? 'Active' : 'Inactive' }}
					</span>
				</button>

				<div
					v-if="games.found[game.key] === false && !games.searching.has(game.key)"
					class="card-extra"
				>
					<p class="m-0 min-w-0 flex-1 text-sm leading-relaxed text-secondary">
						If {{ game.name }} is installed, show Needlelight where so you can play its modpacks.
						<span v-if="locateError[game.key]" class="mt-1 block text-red">{{ locateError[game.key] }}</span>
					</p>
					<Button size="sm" @click="locate(game)"><FolderSearchIcon /> Locate game</Button>
				</div>

				<details v-if="game.key === 'hollow_knight'" class="compat">
					<summary>
						<ChevronRightIcon class="chevron h-4 w-4" aria-hidden="true" />
						Which version of Hollow Knight works with mods?
					</summary>
					<div class="flex flex-col gap-3 pb-1 pl-6 pt-2 text-sm leading-relaxed text-secondary">
						<p class="m-0">
							Mods currently need Hollow Knight
							<span class="font-semibold text-contrast">1.5.78.11833</span>. The newer 1.5.12620
							update (March 2026) moved the game to Unity 6, and most mods don't support it yet.
						</p>
						<ol class="m-0 flex flex-col gap-1.5 pl-5">
							<li v-for="step in HOLLOW_KNIGHT_STEPS" :key="step">{{ step }}</li>
						</ol>
					</div>
				</details>
			</section>
		</div>
	</div>
</template>

<style scoped>
.game-card {
	display: flex;
	flex-direction: column;
	border-radius: var(--nl-panel-radius);
	border: 1px solid var(--color-divider);
	background: var(--surface-2);
	overflow: hidden;
	transition: border-color 0.15s ease;
}
.game-card.is-active {
	border-color: color-mix(in srgb, var(--color-brand) 55%, var(--color-divider));
}
.game-select {
	display: flex;
	align-items: center;
	gap: 0.875rem;
	width: 100%;
	min-width: 0;
	padding: 0.875rem 1rem;
	border: none;
	background: transparent;
	color: inherit;
	cursor: pointer;
	transition: background-color 0.12s ease;
}
.game-select:hover:not(:disabled) {
	background: var(--surface-2-5);
}
.game-select:focus-visible {
	outline: 2px solid var(--color-brand);
	outline-offset: -2px;
}
.game-select:disabled {
	cursor: progress;
}
.radio {
	display: inline-flex;
	align-items: center;
	justify-content: center;
	width: 1.25rem;
	height: 1.25rem;
	flex-shrink: 0;
	border-radius: 50%;
	border: 2px solid var(--surface-5);
	color: var(--color-accent-contrast);
	transition:
		background-color 0.15s ease,
		border-color 0.15s ease;
}
.is-active .radio {
	border-color: var(--color-brand);
	background: var(--color-brand);
}
.card-extra {
	display: flex;
	align-items: center;
	gap: 1rem;
	padding: 0.75rem 1rem;
	border-top: 1px solid var(--color-divider);
	background: color-mix(in srgb, var(--color-orange) 5%, transparent);
}
.compat {
	padding: 0.625rem 1rem 0.75rem;
	border-top: 1px solid var(--color-divider);
}
.compat summary {
	display: flex;
	align-items: center;
	gap: 0.5rem;
	font-size: 0.875rem;
	font-weight: 600;
	color: var(--color-base);
	cursor: pointer;
	list-style: none;
}
.compat summary::-webkit-details-marker {
	display: none;
}
.compat summary:hover {
	color: var(--color-contrast);
}
.compat summary:focus-visible {
	outline: 2px solid var(--color-brand);
	outline-offset: 2px;
	border-radius: 0.25rem;
}
.chevron {
	transition: transform 0.15s ease;
}
.compat[open] .chevron {
	transform: rotate(90deg);
}
</style>
