<template>
<div>
<div
class="p-6 pr-2 pb-4"
@contextmenu.prevent.stop="(event) => handleRightClick(event, instance.path)"
>
<InstanceSettingsModal ref="settingsModal" :instance="instance" :offline="offline" />
<ContentPageHeader>
<template #icon>
<Avatar :src="icon" :alt="instance.name" size="96px" :tint-by="instance.path" />
</template>
<template #title>
{{ instance.name }}
</template>
<template #summary> </template>
<template #stats>
<div
class="flex items-center gap-2 font-semibold transform capitalize border-0 border-solid border-divider pr-4 md:border-r"
>
<GameIcon class="h-6 w-6 text-secondary" />
Hollow Knight
</div>
</template>
<template #actions>
<div class="flex gap-2">
<ButtonStyled size="large" circular>
<button v-tooltip="'Instance settings'" @click="settingsModal.show()">
<SettingsIcon />
</button>
</ButtonStyled>
<ButtonStyled size="large" type="transparent" circular>
<OverflowMenu
:options="[
{
id: 'open-folder',
action: () => showProfileInFolder(instance.path),
},
]"
>
<MoreVerticalIcon />
<template #open-folder> <FolderOpenIcon /> Open folder </template>
</OverflowMenu>
</ButtonStyled>
</div>
</template>
</ContentPageHeader>
</div>
<div class="px-6">
<NavTabs :links="tabs" />
</div>
<div v-if="!!instance" class="p-6 pt-4">
<RouterView v-slot="{ Component }" :key="instance.path">
<template v-if="Component">
<Suspense
:key="instance.path"
@pending="loadingBar.startLoading()"
@resolve="loadingBar.stopLoading()"
>
<component
:is="Component"
:instance="instance"
:options="options"
:offline="offline"
:versions="[]"
:installed="instance.install_stage !== 'installed'"
></component>
<template #fallback>
<LoadingIndicator />
</template>
</Suspense>
</template>
</RouterView>
</div>
<ContextMenu ref="options" @option-clicked="handleOptionsClick">
<template #edit> <EyeIcon /> View instance </template>
<template #copy_path> <ClipboardCopyIcon /> Copy path </template>
<template #open_folder> <FolderOpenIcon /> Open folder </template>
</ContextMenu>
</div>
</template>
<script setup>
import {
ClipboardCopyIcon,
EyeIcon,
FolderOpenIcon,
GameIcon,
MoreVerticalIcon,
SettingsIcon,
} from '@modrinth/assets'
import {
Avatar,
ButtonStyled,
ContentPageHeader,
injectNotificationManager,
LoadingIndicator,
OverflowMenu,
} from '@modrinth/ui'
import { convertFileSrc } from '@tauri-apps/api/core'
import { computed, onUnmounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import ContextMenu from '@/components/ui/ContextMenu.vue'
import InstanceSettingsModal from '@/components/ui/modal/InstanceSettingsModal.vue'
import NavTabs from '@/components/ui/NavTabs.vue'
import { profile_listener } from '@/helpers/events'
import { get, get_full_path } from '@/helpers/profile'
import { showProfileInFolder } from '@/helpers/utils.js'
import { useBreadcrumbs, useLoading } from '@/store/state'

const { handleError } = injectNotificationManager()
const route = useRoute()

const router = useRouter()
const breadcrumbs = useBreadcrumbs()

const offline = ref(!navigator.onLine)
window.addEventListener('offline', () => {
offline.value = true
})
window.addEventListener('online', () => {
offline.value = false
})

const instance = ref()

async function fetchInstance() {
instance.value = await get(route.params.id).catch(handleError)
}

await fetchInstance()
watch(
() => route.params.id,
async () => {
if (route.params.id && route.path.startsWith('/instance')) {
await fetchInstance()
}
},
)

const basePath = computed(() => `/instance/${encodeURIComponent(route.params.id)}`)

const tabs = computed(() => [
{
label: 'Mods',
href: `${basePath.value}`,
},
{
label: 'Logs',
href: `${basePath.value}/logs`,
},
])

breadcrumbs.setName(
'Instance',
instance.value.name.length > 40
? instance.value.name.substring(0, 40) + '...'
: instance.value.name,
)

breadcrumbs.setContext({
name: instance.value.name,
link: route.path,
query: route.query,
})

const loadingBar = useLoading()

const options = ref(null)

const handleRightClick = (event) => {
const baseOptions = [
{ type: 'divider' },
{ name: 'edit' },
{ name: 'open_folder' },
{ name: 'copy_path' },
]

options.value.showMenu(event, instance.value, baseOptions)
}

const handleOptionsClick = async (args) => {
switch (args.option) {
case 'edit':
await router.push({
path: `/instance/${encodeURIComponent(route.params.id)}/options`,
})
break
case 'open_folder':
await showProfileInFolder(instance.value.path)
break
case 'copy_path': {
const fullPath = await get_full_path(instance.value.path)
await navigator.clipboard.writeText(fullPath)
break
}
}
}

const unlistenProfiles = await profile_listener(async (event) => {
if (event.profile_path_id === route.params.id) {
if (event.event === 'removed') {
await router.push({
path: '/',
})
return
}
instance.value = await get(route.params.id).catch(handleError)
}
})

const icon = computed(() =>
instance.value.icon_path ? convertFileSrc(instance.value.icon_path) : null,
)

const settingsModal = ref()

onUnmounted(() => {
unlistenProfiles()
})
</script>

<style scoped lang="scss">
.instance-card {
display: flex;
flex-direction: column;
gap: 1rem;
}

Button {
width: 100%;
}

.button-group {
display: flex;
flex-direction: row;
gap: 0.5rem;
}
</style>
