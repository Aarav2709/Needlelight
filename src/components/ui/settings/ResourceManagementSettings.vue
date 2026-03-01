<script setup>
import { BoxIcon, FolderSearchIcon } from '@modrinth/assets'
import { Button, injectNotificationManager, Slider, StyledInput } from '@modrinth/ui'
import { open } from '@tauri-apps/plugin-dialog'
import { onMounted, ref, watch } from 'vue'

import { get, set } from '@/helpers/settings.ts'

const { handleError } = injectNotificationManager()
const settings = ref(null)
const ready = ref(false)

onMounted(async () => {
  try { settings.value = await get() } catch { /* ignore */ }
  ready.value = true
})

watch(
settings,
async (val) => {
if (!val) return
const setSettings = JSON.parse(JSON.stringify(val))

if (!setSettings.custom_dir) {
setSettings.custom_dir = null
}

await set(setSettings)
},
{ deep: true },
)

async function findLauncherDir() {
const newDir = await open({
multiple: false,
directory: true,
title: 'Select a new app directory',
})

if (newDir) {
settings.value.custom_dir = newDir
}
}
</script>

<template>
<div v-if="!ready" class="text-secondary text-sm p-4">Loading resource settings...</div>
<div v-else-if="settings">
<h2 class="m-0 text-lg font-extrabold text-contrast">App directory</h2>
<p class="m-0 mt-1 mb-2 leading-tight text-secondary">
The directory where Needlelight stores all of its files. Changes will be applied after
restarting the app.
</p>

<div class="m-1 my-2">
<StyledInput
id="appDir"
v-model="settings.custom_dir"
:icon="BoxIcon"
type="text"
wrapper-class="w-full"
>
<template #right>
<Button class="r-btn" @click="findLauncherDir">
<FolderSearchIcon />
</Button>
</template>
</StyledInput>
</div>

<h2 class="m-0 text-lg font-extrabold text-contrast mt-4">Maximum concurrent downloads</h2>
<p class="m-0 mt-1 mb-2 leading-tight text-secondary">
The maximum amount of files the app can download at the same time. Set this to a lower
value if you have a poor internet connection. (app restart required to take effect)
</p>
<Slider
id="max-downloads"
v-model="settings.max_concurrent_downloads"
:min="1"
:max="10"
:step="1"
/>

<h2 class="mt-4 m-0 text-lg font-extrabold text-contrast">Maximum concurrent writes</h2>
<p class="m-0 mt-1 mb-2 leading-tight text-secondary">
The maximum amount of files the app can write to the disk at once. Set this to a lower
value if you are frequently getting I/O errors. (app restart required to take effect)
</p>
<Slider id="max-writes" v-model="settings.max_concurrent_writes" :min="1" :max="50" :step="1" />
</div>
</template>
