<template>
<div>
<template v-if="projects?.length > 0">
<div class="flex items-center gap-2 mb-4">
<StyledInput
v-model="searchFilter"
:icon="SearchIcon"
type="text"
:placeholder="`Search ${filteredProjects.length} mod${filteredProjects.length === 1 ? '' : 's'}...`"
autocomplete="off"
clearable
wrapper-class="flex-grow"
/>
</div>
<div class="flex items-center justify-between">
<Pagination
v-if="search.length > 0"
:page="currentPage"
:count="Math.ceil(search.length / 20)"
:link-function="(page) => `?page=${page}`"
@switch-page="(page) => (currentPage = page)"
/>
</div>

<ContentListPanel
v-model="selectedFiles"
:locked="false"
:items="
search.map((x) => {
const item = {
path: x.path,
disabled: x.disabled,
filename: x.file_name,
icon: x.icon ?? undefined,
title: x.name,
data: x,
}

if (x.version) {
item.version = x.version
item.versionId = x.version
}

return item
})
"
:sort-column="sortColumn"
:sort-ascending="ascending"
:update-sort="sortProjects"
:current-page="currentPage"
>
<template v-if="selectedProjects.length > 0" #headers>
<div class="flex gap-2">
<ButtonStyled v-if="selectedProjects.some((m) => m.disabled)">
<button @click="enableAll()"><CheckCircleIcon /> Enable</button>
</ButtonStyled>
<ButtonStyled v-if="selectedProjects.some((m) => !m.disabled)">
<button @click="disableAll()"><SlashIcon /> Disable</button>
</ButtonStyled>
<ButtonStyled color="red">
<button @click="deleteSelected()"><TrashIcon /> Remove</button>
</ButtonStyled>
</div>
</template>
<template #header-actions>
<ButtonStyled type="transparent" color-fill="text" hover-color-fill="text">
<button :disabled="refreshingProjects" class="w-max" @click="refreshProjects">
<UpdatedIcon />
Refresh
</button>
</ButtonStyled>
</template>
<template #actions="{ item }">
<div class="w-[36px]"></div>
<Toggle
class="!mx-2"
:model-value="!item.data.disabled"
@update:model-value="toggleDisableMod(item.data)"
/>
<ButtonStyled type="transparent" circular>
<button v-tooltip="'Remove'" @click="removeMod(item)">
<TrashIcon />
</button>
</ButtonStyled>
<ButtonStyled type="transparent" circular>
<OverflowMenu
:options="[
{
id: 'show-file',
action: () => highlightModInProfile(instance.path, item.path),
},
]"
direction="left"
>
<MoreVerticalIcon />
<template #show-file> <ExternalIcon /> Show file </template>
</OverflowMenu>
</ButtonStyled>
</template>
</ContentListPanel>
<div class="flex justify-end mt-4">
<Pagination
v-if="search.length > 0"
:page="currentPage"
:count="Math.ceil(search.length / 20)"
:link-function="(page) => `?page=${page}`"
@switch-page="(page) => (currentPage = page)"
/>
</div>
</template>
<div v-else class="w-full max-w-[48rem] mx-auto flex flex-col mt-6">
<RadialHeader>
<div class="flex items-center gap-6 w-[32rem] mx-auto">
<span class="text-contrast font-bold text-xl"
>No mods installed in this instance yet.</span
>
</div>
</RadialHeader>
</div>
</div>
</template>
<script setup lang="ts">
import {
CheckCircleIcon,
ExternalIcon,
MoreVerticalIcon,
SearchIcon,
SlashIcon,
TrashIcon,
UpdatedIcon,
} from '@modrinth/assets'
import {
ButtonStyled,
ContentListPanel,
injectNotificationManager,
OverflowMenu,
Pagination,
RadialHeader,
StyledInput,
Toggle,
} from '@modrinth/ui'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import dayjs from 'dayjs'
import { computed, onUnmounted, ref, watch } from 'vue'

import type ContextMenu from '@/components/ui/ContextMenu.vue'
import { profile_listener } from '@/helpers/events.js'
import {
add_project_from_path,
get,
get_projects,
remove_project,
toggle_disable_project,
} from '@/helpers/profile.js'
import type { ContentFile, GameInstance } from '@/helpers/types'
import { highlightModInProfile } from '@/helpers/utils.js'

const { handleError } = injectNotificationManager()

const props = defineProps<{
instance: GameInstance
options: InstanceType<typeof ContextMenu>
offline: boolean
playing: boolean
versions: any[]
installed: boolean
}>()

type ProjectListEntry = {
path: string
name: string
version: string | null
file_name: string
icon: string | undefined
disabled: boolean
outdated: boolean
updated: dayjs.Dayjs
project_type: string
}

const projects = ref<ProjectListEntry[]>([])
const selectedFiles = ref<string[]>([])
const selectedProjects = computed(() =>
projects.value.filter((x) => selectedFiles.value.includes(x.file_name)),
)

const initProjects = async () => {
const newProjects: ProjectListEntry[] = []

const profileProjects = (await get_projects(props.instance.path)) as Record<
string,
ContentFile
>

for (const [path, file] of Object.entries(profileProjects)) {
newProjects.push({
path,
name: file.file_name.replace('.disabled', '').replace('.dll', ''),
version: null,
file_name: file.file_name,
icon: undefined,
disabled: file.file_name.endsWith('.disabled'),
outdated: false,
updated: dayjs(0),
project_type: file.project_type || 'mod',
})
}

projects.value = newProjects ?? []
}
await initProjects()

const filteredProjects = computed(() => projects.value)

const searchFilter = ref('')
const ascending = ref(true)
const sortColumn = ref('Name')
const currentPage = ref(1)

const search = computed(() => {
const filtered = filteredProjects.value.filter((mod) => {
return mod.name.toLowerCase().includes(searchFilter.value.toLowerCase())
})

return filtered
.slice()
.sort((a, b) =>
ascending.value ? a.name.localeCompare(b.name) : b.name.localeCompare(a.name),
)
})

watch([sortColumn, ascending, searchFilter], () => (currentPage.value = 1))

const sortProjects = (filter: string) => {
if (sortColumn.value === filter) {
ascending.value = !ascending.value
} else {
sortColumn.value = filter
ascending.value = true
}
}

const locks: Record<string, string | null> = {}

const toggleDisableMod = async (mod: ProjectListEntry) => {
const lock = locks[mod.file_name]

while (lock) {
await new Promise((resolve) => {
setTimeout((value: unknown) => resolve(value), 100)
})
}

locks[mod.file_name] = 'lock'

try {
mod.path = await toggle_disable_project(props.instance.path, mod.path)
mod.disabled = !mod.disabled
} catch (err) {
handleError(err)
}

locks[mod.file_name] = null
}

const removeMod = async (mod: { path: string; data: ProjectListEntry }) => {
await remove_project(props.instance.path, mod.path).catch(handleError)
projects.value = projects.value.filter((x) => mod.path !== x.path)
}

const deleteSelected = async () => {
for (const project of selectedProjects.value) {
await remove_project(props.instance.path, project.path).catch(handleError)
}
projects.value = projects.value.filter(
(x) => !selectedFiles.value.includes(x.file_name),
)
selectedFiles.value = []
}

const enableAll = async () => {
const promises = []
for (const project of selectedProjects.value) {
if (project.disabled) {
promises.push(toggleDisableMod(project))
}
}
await Promise.all(promises).catch(handleError)
}

const disableAll = async () => {
const promises = []
for (const project of selectedProjects.value) {
if (!project.disabled) {
promises.push(toggleDisableMod(project))
}
}
await Promise.all(promises).catch(handleError)
}

const refreshingProjects = ref(false)
async function refreshProjects() {
refreshingProjects.value = true
await initProjects()
refreshingProjects.value = false
}

const unlisten = await getCurrentWebview().onDragDropEvent(async (event) => {
if (event.payload.type !== 'drop') return

for (const file of event.payload.paths) {
await add_project_from_path(props.instance.path, file).catch(handleError)
}
await initProjects()
})

const unlistenProfiles = await profile_listener(
async (event: { event: string; profile_path_id: string }) => {
if (
event.profile_path_id === props.instance.path &&
event.event === 'synced'
) {
await initProjects()
}
},
)

onUnmounted(() => {
unlisten()
unlistenProfiles()
})
</script>

<style scoped lang="scss">
.text-input {
width: 100%;
}
</style>
