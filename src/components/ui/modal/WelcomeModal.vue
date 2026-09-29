<script setup lang="ts">
/**
 * First launch: find Hollow Knight (or ask where it is), give it a Default modpack, and land the
 * player on that modpack's page. Settings → General → "Welcome guide" shows it again.
 */
import { CheckIcon, CircleAlertIcon, FolderSearchIcon, SpinnerIcon } from '@modrinth/assets'
import { Button, injectNotificationManager, NewModal } from '@modrinth/ui'
import { open } from '@tauri-apps/plugin-dialog'
import { computed, ref, watch } from 'vue'
import { useRouter } from 'vue-router'

import { useGameSetup } from '@/composables/setup'
import { gameInfo, type GameKey } from '@/helpers/games'
import { type Modpack, modpackRoute } from '@/helpers/modpacks'
import { useGames } from '@/store/games'
import { useModpacks } from '@/store/modpacks'
import { isOnboarded, setOnboarded } from '@/store/preferences'

const GAME: GameKey = 'hollow_knight'
const name = gameInfo(GAME).name

const router = useRouter()
const games = useGames()
const modpacks = useModpacks()
const { ensureDefaultModpack } = useGameSetup()
const { handleError } = injectNotificationManager()

const modal = ref<InstanceType<typeof NewModal> | null>(null)
const isOpen = ref(false)
const target = ref<Modpack | null>(null)
const preparing = ref(false)
const locateError = ref('')

const found = computed(() => games.found[GAME])
// The saved location is the game's Managed folder; show the game folder itself.
const folder = computed(() =>
	(games.settings?.managed_folders?.[GAME] ?? '').replace(/[\\/][^\\/]*_Data[\\/]Managed[\\/]?$/i, ''),
)
const state = computed<'checking' | 'found' | 'missing'>(() => {
	if (found.value === true) return 'found'
	if (found.value === null || games.searching.has(GAME)) return 'checking'
	return 'missing'
})

/** Once the game is found, make sure it has a modpack to land on. */
async function prepare() {
	if (preparing.value || target.value) return
	preparing.value = true
	try {
		target.value = (await ensureDefaultModpack(GAME)) ?? modpacks.forGame(GAME)[0] ?? null
	} catch (err) {
		handleError(err as Error)
	} finally {
		preparing.value = false
	}
}

watch(
	[state, isOpen],
	([now, visible]) => {
		if (visible && now === 'found') void prepare()
	},
	{ immediate: true },
)

async function show() {
	target.value = null
	locateError.value = ''
	isOpen.value = true
	modal.value?.show()
	try {
		await Promise.all([games.ensureLoaded(), modpacks.ensureLoaded()])
		await games.refreshAvailability()
		// Not where the backend expected: search the usual install locations once.
		if (found.value === false) void games.findGame(GAME)
	} catch (err) {
		handleError(err as Error)
	}
}

/** Show the guide if it hasn't been seen yet. */
function showIfFirstLaunch() {
	if (!isOnboarded()) void show()
}

async function locate() {
	const picked = await open({ directory: true, title: `Where is ${name} installed?` })
	if (typeof picked !== 'string') return
	locateError.value = ''
	try {
		if (!(await games.locateGame(GAME, picked))) {
			locateError.value = `That folder doesn't contain ${name}. Choose the folder the game is installed in.`
		}
	} catch (err) {
		handleError(err as Error)
	}
}

function onHide() {
	isOpen.value = false
	setOnboarded(true)
}

function finish() {
	setOnboarded(true)
	isOpen.value = false
	modal.value?.hide()
}

async function getStarted() {
	finish()
	if (target.value) await router.push(modpackRoute(target.value))
}

async function skip() {
	finish()
	await router.push('/modpacks')
}

defineExpose({ showIfFirstLaunch, show })
</script>

<template>
	<NewModal
		ref="modal"
		header="Welcome to Needlelight"
		width="min(32rem, calc(100vw - 4rem))"
		:on-hide="onHide"
	>
		<div class="flex flex-col gap-5">
			<p class="m-0 leading-relaxed text-secondary">
				Needlelight keeps your mods in <strong class="text-contrast">modpacks</strong>: separate sets
				of mods you can switch between. First, let's find {{ name }}.
			</p>

			<div class="status" :class="`status--${state}`" role="status" aria-live="polite">
				<span class="status-icon">
					<SpinnerIcon v-if="state === 'checking'" class="h-5 w-5 animate-spin" />
					<CheckIcon v-else-if="state === 'found'" class="h-5 w-5" />
					<CircleAlertIcon v-else class="h-5 w-5" />
				</span>
				<div class="flex min-w-0 flex-1 flex-col gap-1">
					<template v-if="state === 'checking'">
						<span class="font-semibold text-contrast">Looking for {{ name }}…</span>
						<span class="text-sm text-secondary">This can take a moment the first time.</span>
					</template>
					<template v-else-if="state === 'found'">
						<span class="font-semibold text-contrast">Found {{ name }}</span>
						<span v-if="folder" class="nl-mono truncate text-secondary" :title="folder">{{ folder }}</span>
					</template>
					<template v-else>
						<span class="font-semibold text-contrast">Couldn't find {{ name }}</span>
						<span class="text-sm leading-relaxed text-secondary">
							Choose the folder {{ name }} is installed in. In Steam: right-click the game, then
							Manage → Browse local files shows it.
						</span>
						<span v-if="locateError" class="text-sm text-red">{{ locateError }}</span>
					</template>
				</div>
			</div>

			<p v-if="state === 'found'" class="m-0 text-sm leading-relaxed text-secondary">
				<template v-if="preparing">Setting up your first modpack…</template>
				<template v-else-if="target">
					Your modpack <strong class="text-contrast">{{ target.name }}</strong> is ready. Browse mods
					to add some, then press Play.
				</template>
			</p>
		</div>

		<template #actions>
			<div class="flex justify-end gap-2">
				<Button v-if="state !== 'found'" type="quiet" @click="skip">Skip for now</Button>
				<Button v-if="state === 'missing'" type="colored" color="brand" @click="locate">
					<FolderSearchIcon /> Locate {{ name }}
				</Button>
				<Button v-if="state === 'found'" type="colored" color="brand" :disabled="!target" @click="getStarted">
					<SpinnerIcon v-if="preparing" class="animate-spin" />
					Get started
				</Button>
			</div>
		</template>
	</NewModal>
</template>

<style scoped>
.status {
	--_c: var(--color-secondary);
	display: flex;
	align-items: flex-start;
	gap: 0.875rem;
	padding: 0.875rem 1rem;
	border-radius: var(--nl-panel-radius);
	border: 1px solid color-mix(in srgb, var(--_c) 30%, var(--color-divider));
	background: color-mix(in srgb, var(--_c) 7%, var(--surface-2));
}
.status--found {
	--_c: var(--color-green);
}
.status--missing {
	--_c: var(--color-orange);
}
.status-icon {
	display: inline-flex;
	flex-shrink: 0;
	padding-top: 0.0625rem;
	color: var(--_c);
}
</style>
