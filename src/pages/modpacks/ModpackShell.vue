<script setup lang="ts">
// parent of a modpack's mods and browse pages, it loads the modpack, shares its context, and owns their confirmation dialog
import { ConfirmModal, injectNotificationManager } from '@modrinth/ui'
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import { provideModpackContext } from '@/composables/modpack'
import { useShortcut } from '@/composables/shortcuts'
import { useBreadcrumbs } from '@/store/breadcrumbs'
import { useCatalog } from '@/store/catalog'
import { useGames } from '@/store/games'
import { useModpacks } from '@/store/modpacks'
import { usePreferences } from '@/store/preferences'

const route = useRoute()
const router = useRouter()
const games = useGames()
const modpacks = useModpacks()
const catalog = useCatalog()
const { prefs } = usePreferences()
const breadcrumbs = useBreadcrumbs()
const { handleError, addNotification } = injectNotificationManager()

// vue router already decodes params, decoding again would break names containing a percent sign
const path = computed(() => String(route.params.path ?? ''))
const ctx = provideModpackContext(path)
const { modpack, game } = ctx

async function load() {
	try {
		await modpacks.ensureLoaded()
	} catch (err) {
		handleError(err as Error)
		return
	}
	if (!modpack.value) {
		addNotification({ title: 'That modpack no longer exists', type: 'warning' })
		await router.replace('/modpacks')
		return
	}
	if (modpack.value.game !== games.activeGame) {
		// opened a modpack for the other game, like right after creating it, so follow it
		await games.switchGame(modpack.value.game).catch(() => {})
	}
	prefs.lastModpack[modpack.value.game] = path.value
	void modpacks.loadInstalled(path.value)
	void catalog.load(game.value)
}

watch(path, () => void load())
onMounted(load)

watch(
	() => modpack.value?.name,
	(name) => name && breadcrumbs.setName('modpack', name),
	{ immediate: true },
)

useShortcut('mod+enter', () => void ctx.play(), { inInputs: true, when: () => !modpacks.launching })

// confirmations requested by either page

const confirmModal = ref<InstanceType<typeof ConfirmModal> | null>(null)
watch(ctx.confirmRequest, (request) => {
	if (request) void nextTick(() => confirmModal.value?.show())
})
async function proceed() {
	const request = ctx.confirmRequest.value
	ctx.confirmRequest.value = null
	await request?.onProceed()
}
</script>

<template>
	<RouterView v-if="modpack" v-slot="{ Component }">
		<component :is="Component" :key="path" />
	</RouterView>
	<div v-else class="flex h-full items-center justify-center text-secondary">
		<span class="sr-only">Loading</span>
	</div>

	<ConfirmModal
		ref="confirmModal"
		:title="ctx.confirmRequest.value?.title ?? ''"
		:description="ctx.confirmRequest.value?.description ?? ''"
		:proceed-label="ctx.confirmRequest.value?.proceedLabel ?? 'Continue'"
		:danger="ctx.confirmRequest.value?.danger ?? false"
		:markdown="false"
		@proceed="proceed"
	/>
</template>
