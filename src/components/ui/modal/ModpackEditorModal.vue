<script setup lang="ts">
import { PlusIcon, SaveIcon, SpinnerIcon } from '@modrinth/assets'
import { Button, Input, NewModal, Textarea } from '@modrinth/ui'
import { computed, nextTick, ref } from 'vue'

import { gameInfo, type GameKey } from '@/helpers/games'
import type { Modpack } from '@/helpers/modpacks'
import { useGames } from '@/store/games'
import { useModpacks } from '@/store/modpacks'

const emit = defineEmits<{ saved: [modpack: Modpack | null] }>()

const games = useGames()
const modpacks = useModpacks()

const modal = ref<InstanceType<typeof NewModal> | null>(null)
const nameInput = ref<HTMLElement | null>(null)

const NAME_MAX = 64
const DESCRIPTION_MAX = 300

const editing = ref<Modpack | null>(null)
const name = ref('')
const description = ref('')
const game = ref<GameKey>('hollow_knight')
const saving = ref(false)
const error = ref('')

const trimmedName = computed(() => name.value.trim())
const nameTaken = computed(() =>
	modpacks.list.some(
		(m) =>
			m.game === game.value &&
			m.path !== editing.value?.path &&
			m.name.trim().toLowerCase() === trimmedName.value.toLowerCase(),
	),
)
const canSave = computed(
	() =>
		trimmedName.value.length > 0 &&
		name.value.length <= NAME_MAX &&
		!nameTaken.value &&
		!saving.value,
)

function show(modpack?: Modpack, defaultGame?: GameKey) {
	editing.value = modpack ?? null
	name.value = modpack?.name ?? ''
	description.value = modpack?.description ?? ''
	game.value = modpack?.game ?? defaultGame ?? games.activeGame
	error.value = ''
	saving.value = false
	modal.value?.show()
	nextTick(() => nameInput.value?.querySelector('input')?.focus())
}

async function save() {
	if (!canSave.value) return
	saving.value = true
	error.value = ''
	let saved: Modpack | null = null
	try {
		if (editing.value) {
			await modpacks.edit(editing.value.path, {
				name: trimmedName.value,
				description: description.value.trim(),
			})
		} else {
			saved = await modpacks.create({
				name: trimmedName.value,
				game: game.value,
				description: description.value.trim(),
			})
		}
	} catch (err) {
		error.value = String(err)
		saving.value = false
		return
	}
	// `disable-close` follows `saving`; let the modal see it cleared before closing.
	saving.value = false
	await nextTick()
	modal.value?.hide()
	emit('saved', saved)
}

defineExpose({ show })
</script>

<template>
	<NewModal
		ref="modal"
		:header="editing ? 'Edit modpack' : 'New modpack'"
		width="min(32rem, calc(100vw - 4rem))"
		:disable-close="saving"
	>
		<form class="flex flex-col gap-5" @submit.prevent="save">
			<div class="flex items-start gap-4">
				<div class="flex min-w-0 flex-1 flex-col gap-2">
					<label for="modpack-name" class="flex items-baseline justify-between gap-2">
						<span class="font-semibold text-contrast">Name</span>
						<span class="text-xs" :class="name.length > NAME_MAX ? 'text-red' : 'text-secondary'">
							{{ name.length }}/{{ NAME_MAX }}
						</span>
					</label>
					<div ref="nameInput">
						<Input
							id="modpack-name"
							v-model="name"
							wrapper-class="w-full"
							placeholder="e.g. Steel Soul practice"
							:maxlength="NAME_MAX"
							autocomplete="off"
						/>
					</div>
					<p v-if="nameTaken" class="m-0 text-sm text-red">
						You already have a {{ gameInfo(game).name }} modpack with this name.
					</p>
				</div>
			</div>

			<div class="flex flex-col gap-2">
				<label for="modpack-description" class="flex items-baseline justify-between gap-2">
					<span class="font-semibold text-contrast">
						Description <span class="font-normal text-secondary">(optional)</span>
					</span>
					<span class="text-xs text-secondary">{{ description.length }}/{{ DESCRIPTION_MAX }}</span>
				</label>
				<Textarea
					id="modpack-description"
					v-model="description"
					placeholder="What is this modpack for?"
					:maxlength="DESCRIPTION_MAX"
					:rows="3"
					resize="none"
				/>
			</div>

			<!-- The game is whichever one is selected in the Modpacks sidebar. -->
			<p class="m-0 text-sm text-secondary">
				For <span class="font-semibold text-contrast">{{ gameInfo(game).name }}</span>
			</p>

			<p v-if="error" class="m-0 rounded-lg bg-bg-red px-3 py-2 text-sm text-red">{{ error }}</p>
			<button type="submit" class="hidden" aria-hidden="true" tabindex="-1" />
		</form>

		<template #actions>
			<div class="flex justify-end gap-2">
				<Button type="quiet" :disabled="saving" @click="modal?.hide()">Cancel</Button>
				<Button type="colored" color="brand" :disabled="!canSave" @click="save">
					<SpinnerIcon v-if="saving" class="animate-spin" />
					<SaveIcon v-else-if="editing" />
					<PlusIcon v-else />
					<template v-if="saving">{{ editing ? 'Saving…' : 'Creating…' }}</template>
					<template v-else>{{ editing ? 'Save changes' : 'Create modpack' }}</template>
				</Button>
			</div>
		</template>
	</NewModal>
</template>
