<script setup lang="ts">
import {
ChevronRightIcon,
CodeIcon,
InfoIcon,
WrenchIcon,
} from '@modrinth/assets'
import {
Avatar,
commonMessages,
defineMessage,
TabbedModal,
type TabbedModalTab,
useVIntl,
} from '@modrinth/ui'
import { convertFileSrc } from '@tauri-apps/api/core'
import { ref } from 'vue'

import GeneralSettings from '@/components/ui/instance_settings/GeneralSettings.vue'
import HooksSettings from '@/components/ui/instance_settings/HooksSettings.vue'
import InstallationSettings from '@/components/ui/instance_settings/InstallationSettings.vue'
import ModalWrapper from '@/components/ui/modal/ModalWrapper.vue'

import type { InstanceSettingsTabProps } from '../../../helpers/types'

const { formatMessage } = useVIntl()

const props = defineProps<InstanceSettingsTabProps>()

// NOTE (Needlelight): @modrinth/ui's Tab type is no longer generic and no longer carries a
// per-tab `props` field, so tab content is rendered by TabbedModal with no props forwarded.
// We therefore render the active tab ourselves via the new #content slot (below) and bind
// the instance settings props explicitly.
const tabs: TabbedModalTab[] = [
{
name: defineMessage({
id: 'instance.settings.tabs.general',
defaultMessage: 'General',
}),
icon: InfoIcon,
content: GeneralSettings,
},
{
name: defineMessage({
id: 'instance.settings.tabs.installation',
defaultMessage: 'Installation',
}),
icon: WrenchIcon,
content: InstallationSettings,
},
{
name: defineMessage({
id: 'instance.settings.tabs.hooks',
defaultMessage: 'Launch hooks',
}),
icon: CodeIcon,
content: HooksSettings,
},
]

const modal = ref()

function show() {
modal.value.show()
}

defineExpose({ show })
</script>
<template>
<ModalWrapper ref="modal">
<template #title>
<span class="flex items-center gap-2 text-lg font-semibold text-primary">
<Avatar
:src="instance.icon_path ? convertFileSrc(instance.icon_path) : undefined"
size="24px"
:tint-by="props.instance.path"
/>
{{ instance.name }} <ChevronRightIcon />
<span class="font-extrabold text-contrast">{{
formatMessage(commonMessages.settingsLabel)
}}</span>
</span>
</template>

<TabbedModal :tabs="tabs">
<template #content="{ tab }">
<component :is="tab.content" v-if="tab?.content" v-bind="props" />
</template>
</TabbedModal>
</ModalWrapper>
</template>
