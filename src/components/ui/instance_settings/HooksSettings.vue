<script setup lang="ts">
import {
	Checkbox,
	defineMessages,
	injectNotificationManager,
	Input,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref, watch } from 'vue'

import { edit } from '@/helpers/profile'
import { get } from '@/helpers/settings.ts'

import type { AppSettings, Hooks, InstanceSettingsTabProps } from '../../../helpers/types'

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()

const props = defineProps<InstanceSettingsTabProps>()

const globalSettings = (await get().catch(handleError)) as unknown as AppSettings

const overrideHooks = ref(
	!!props.instance.hooks.pre_launch ||
		!!props.instance.hooks.wrapper ||
		!!props.instance.hooks.post_exit,
)
// The backend models hooks as Option<String>, so values arrive as `string | null`,
// but Input's v-model only accepts `string | number | undefined`. Keep the editable
// state as plain strings and convert empty back to null when saving.
const sourceHooks = props.instance.hooks ?? globalSettings.hooks ?? {}
const hooks = ref({
	pre_launch: sourceHooks.pre_launch ?? '',
	wrapper: sourceHooks.wrapper ?? '',
	post_exit: sourceHooks.post_exit ?? '',
})

const editProfileObject = computed(() => {
	const editProfile: {
		hooks?: Hooks
	} = {}

	// When hooks are not overridden per-instance, we want to clear them
	editProfile.hooks = overrideHooks.value
		? {
				pre_launch: hooks.value.pre_launch || null,
				wrapper: hooks.value.wrapper || null,
				post_exit: hooks.value.post_exit || null,
			}
		: {}

	return editProfile
})

watch(
	[overrideHooks, hooks],
	async () => {
		await edit(props.instance.path, editProfileObject.value)
	},
	{ deep: true },
)
const messages = defineMessages({
	hooks: {
		id: 'instance.settings.tabs.hooks.title',
		defaultMessage: 'Game launch hooks',
	},
	hooksDescription: {
		id: 'instance.settings.tabs.hooks.description',
		defaultMessage:
			'Hooks allow advanced users to run certain system commands before and after launching the game.',
	},
	customHooks: {
		id: 'instance.settings.tabs.hooks.custom-hooks',
		defaultMessage: 'Custom launch hooks',
	},
	preLaunch: {
		id: 'instance.settings.tabs.hooks.pre-launch',
		defaultMessage: 'Pre-launch',
	},
	preLaunchDescription: {
		id: 'instance.settings.tabs.hooks.pre-launch.description',
		defaultMessage: 'Ran before the instance is launched.',
	},
	preLaunchEnter: {
		id: 'instance.settings.tabs.hooks.pre-launch.enter',
		defaultMessage: 'Enter pre-launch command...',
	},
	wrapper: {
		id: 'instance.settings.tabs.hooks.wrapper',
		defaultMessage: 'Wrapper',
	},
	wrapperDescription: {
		id: 'instance.settings.tabs.hooks.wrapper.description',
		defaultMessage: 'Wrapper command for launching Minecraft.',
	},
	wrapperEnter: {
		id: 'instance.settings.tabs.hooks.wrapper.enter',
		defaultMessage: 'Enter wrapper command...',
	},
	postExit: {
		id: 'instance.settings.tabs.hooks.post-exit',
		defaultMessage: 'Post-exit',
	},
	postExitDescription: {
		id: 'instance.settings.tabs.hooks.post-exit.description',
		defaultMessage: 'Ran after the game closes.',
	},
	postExitEnter: {
		id: 'instance.settings.tabs.hooks.post-exit.enter',
		defaultMessage: 'Enter post-exit command...',
	},
})
</script>

<template>
	<div>
		<h2 class="m-0 mb-1 text-lg font-extrabold text-contrast">
			{{ formatMessage(messages.hooks) }}
		</h2>
		<p class="m-0">
			{{ formatMessage(messages.hooksDescription) }}
		</p>
		<Checkbox v-model="overrideHooks" :label="formatMessage(messages.customHooks)" class="mt-2" />

		<h2 class="mt-2 mb-1 text-lg font-extrabold text-contrast">
			{{ formatMessage(messages.preLaunch) }}
		</h2>
		<p class="m-0">
			{{ formatMessage(messages.preLaunchDescription) }}
		</p>
		<Input
			id="pre-launch"
			v-model="hooks.pre_launch"
			autocomplete="off"
			:disabled="!overrideHooks"
			:placeholder="formatMessage(messages.preLaunchEnter)"
			wrapper-class="w-full mt-2"
		/>

		<h2 class="mt-4 mb-1 text-lg font-extrabold text-contrast">
			{{ formatMessage(messages.wrapper) }}
		</h2>
		<p class="m-0">
			{{ formatMessage(messages.wrapperDescription) }}
		</p>
		<Input
			id="wrapper"
			v-model="hooks.wrapper"
			autocomplete="off"
			:disabled="!overrideHooks"
			:placeholder="formatMessage(messages.wrapperEnter)"
			wrapper-class="w-full mt-2"
		/>

		<h2 class="mt-4 mb-1 text-lg font-extrabold text-contrast">
			{{ formatMessage(messages.postExit) }}
		</h2>
		<p class="m-0">
			{{ formatMessage(messages.postExitDescription) }}
		</p>
		<Input
			id="post-exit"
			v-model="hooks.post_exit"
			autocomplete="off"
			:disabled="!overrideHooks"
			:placeholder="formatMessage(messages.postExitEnter)"
			wrapper-class="w-full mt-2"
		/>
	</div>
</template>
