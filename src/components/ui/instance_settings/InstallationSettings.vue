<script setup lang="ts">
import {
HammerIcon,
SpinnerIcon,
WrenchIcon,
} from '@modrinth/assets'
import {
ButtonStyled,
defineMessages,
injectNotificationManager,
useVIntl,
} from '@modrinth/ui'
import { ref } from 'vue'

import ConfirmModalWrapper from '@/components/ui/modal/ConfirmModalWrapper.vue'
import { install } from '@/helpers/profile'

import type { InstanceSettingsTabProps } from '../../../helpers/types'

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()

const repairConfirmModal = ref()

const props = defineProps<InstanceSettingsTabProps>()

const installing = computed(() => props.instance.install_stage !== 'installed')
const repairing = ref(false)

async function repairProfile() {
repairing.value = true
await install(props.instance.path, true).catch(handleError)
repairing.value = false
}

import { computed } from 'vue'

const messages = defineMessages({
currentlyInstalled: {
id: 'instance.settings.tabs.installation.currently-installed',
defaultMessage: 'Currently installed',
},
repairConfirmTitle: {
id: 'instance.settings.tabs.installation.repair.confirm.title',
defaultMessage: 'Repair instance?',
},
repairConfirmDescription: {
id: 'instance.settings.tabs.installation.repair.confirm.description',
defaultMessage:
'Repairing will re-check the game files and dependencies. This may resolve issues if the game is not launching correctly.',
},
repairButton: {
id: 'instance.settings.tabs.installation.repair.button',
defaultMessage: 'Repair',
},
repairingButton: {
id: 'instance.settings.tabs.installation.repair.button.repairing',
defaultMessage: 'Repairing',
},
})
</script>

<template>
<ConfirmModalWrapper
ref="repairConfirmModal"
:title="formatMessage(messages.repairConfirmTitle)"
:description="formatMessage(messages.repairConfirmDescription)"
:proceed-icon="HammerIcon"
:proceed-label="formatMessage(messages.repairButton)"
:danger="false"
:show-ad-on-close="false"
@proceed="() => repairProfile()"
/>
<div>
<h2 id="project-name" class="m-0 mb-1 text-lg font-extrabold text-contrast block">
{{ formatMessage(messages.currentlyInstalled) }}
</h2>
<div class="flex gap-4 items-center justify-between p-4 bg-bg rounded-2xl">
<div class="flex gap-2 items-center">
<div
class="w-10 h-10 flex items-center justify-center rounded-full bg-button-bg border-solid border-[1px] border-button-border p-2 [&_svg]:h-full [&_svg]:w-full"
>
<WrenchIcon />
</div>
<div class="flex flex-col gap-2 justify-center">
<span class="font-semibold leading-none">Hollow Knight</span>
<span class="text-sm text-secondary leading-none">Modded Instance</span>
</div>
</div>
<div class="flex gap-1">
<ButtonStyled color="orange" type="transparent" hover-color-fill="background">
<button
:disabled="installing || repairing"
@click="repairConfirmModal.show()"
>
<SpinnerIcon v-if="repairing" class="animate-spin" />
<HammerIcon v-else />
{{
repairing
? formatMessage(messages.repairingButton)
: formatMessage(messages.repairButton)
}}
</button>
</ButtonStyled>
</div>
</div>
</div>
</template>
