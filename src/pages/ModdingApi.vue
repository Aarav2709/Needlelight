<script setup>
import { DownloadIcon, FolderSearchIcon, RefreshCwIcon, ShieldIcon, SpinnerIcon } from '@modrinth/assets'
import { ButtonStyled, injectNotificationManager } from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { computed, onMounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import { useBreadcrumbs } from '@/store/breadcrumbs'

const { handleError } = injectNotificationManager()
const route = useRoute()
const breadcrumbs = useBreadcrumbs()
breadcrumbs.setRootContext({ name: 'Modding API', link: route.path })

const loading = ref(true)
const installing = ref(false)
const apiInfo = ref(null)
const apiInstalled = ref(false)
const apiEnabled = ref(false)
const managedFolder = ref('')
const error = ref(null)

const hasFolder = computed(() => managedFolder.value.trim().length > 0)
const apiVersion = computed(() => apiInfo.value?.version ? `v${apiInfo.value.version}` : 'Version unavailable')
const ctaLabel = computed(() => apiInstalled.value ? 'Reinstall API' : 'Install API')
const statusText = computed(() => apiInstalled.value ? (apiEnabled.value ? 'Installed and ready' : 'Installed, currently disabled') : 'Not installed')

async function loadState() {
  loading.value = true
  error.value = null
  try {
    const settings = await invoke('load_settings')
    managedFolder.value = settings.managed_folder || ''
    const catalog = await invoke('refresh_catalog', { fetchOfficial: true })
    apiInfo.value = catalog?.api || null
    apiInstalled.value = !!catalog?.api_installed
    apiEnabled.value = catalog?.api_enabled !== false && apiInstalled.value
  } catch (err) {
    error.value = err
  } finally {
    loading.value = false
  }
}

async function chooseFolder() {
  try {
    const folder = await open({ directory: true, title: "Select Hollow Knight's Managed Folder" })
    if (!folder) return
    const settings = await invoke('load_settings')
    settings.managed_folder = folder
    settings.managed_folders = settings.managed_folders || {}
    settings.managed_folders[settings.game] = folder
    await invoke('save_settings', { settings })
    await loadState()
  } catch (err) {
    handleError(err)
  }
}

async function installApi() {
  if (!hasFolder.value) {
    await chooseFolder()
    if (!hasFolder.value) return
  }
  installing.value = true
  try {
    await invoke('install_api')
    await loadState()
  } catch (err) {
    handleError(err)
  } finally {
    installing.value = false
  }
}

onMounted(loadState)
</script>

<template>
  <div class="min-h-full p-6 flex items-center justify-center">
    <div v-if="loading" class="w-full max-w-3xl min-h-[58vh] flex flex-col items-center justify-center text-secondary gap-3">
      <div class="w-12 h-12 rounded-2xl bg-bg-raised border border-solid border-surface-5 flex items-center justify-center"><SpinnerIcon class="w-5 h-5 animate-spin" /></div>
      <span class="text-sm">Loading Modding API...</span>
    </div>

    <div v-else class="w-full max-w-4xl rounded-2xl bg-bg-raised border border-solid border-surface-5 overflow-hidden shadow-lg shadow-black/10">
      <div class="px-6 py-5 border-b border-solid border-surface-5 flex items-center justify-between gap-4 flex-wrap">
        <div class="flex items-center gap-3 min-w-0">
          <div class="w-10 h-10 rounded-xl bg-brand/10 border border-solid border-brand/20 flex items-center justify-center text-brand shrink-0"><ShieldIcon class="w-5 h-5" /></div>
          <div class="min-w-0">
            <div class="flex items-center gap-2 flex-wrap">
              <h1 class="m-0 text-xl font-extrabold text-contrast">Modding API</h1>
              <span class="px-2 py-1 rounded-md bg-button-bg text-secondary text-xs font-bold">{{ apiVersion }}</span>
            </div>
            <div class="flex items-center gap-2 mt-1 text-xs text-secondary">
              <span class="w-1.5 h-1.5 rounded-full" :class="apiInstalled && apiEnabled ? 'bg-green-500' : 'bg-secondary'" />
              {{ statusText }}
            </div>
          </div>
        </div>
        <ButtonStyled color="brand" :disabled="installing || !hasFolder">
          <button @click="installApi">
            <RefreshCwIcon v-if="installing" class="animate-spin" />
            <DownloadIcon v-else />
            {{ installing ? 'Installing...' : ctaLabel }}
          </button>
        </ButtonStyled>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-4 p-6">
        <section class="rounded-xl border border-solid border-surface-5 bg-bg p-4">
          <p class="m-0 text-[11px] font-semibold uppercase tracking-wider text-secondary">What it does</p>
          <p class="m-0 mt-2 text-sm text-secondary leading-relaxed">The Modding API provides the runtime Hollow Knight and its mods use to load together. Install it before installing mods, and reinstall it whenever you need to refresh the API files.</p>
        </section>
        <section class="rounded-xl border border-solid border-surface-5 bg-bg p-4">
          <div class="flex items-center justify-between gap-3">
            <p class="m-0 text-[11px] font-semibold uppercase tracking-wider text-secondary">Managed folder</p>
            <ButtonStyled type="transparent" size="small"><button @click="chooseFolder"><FolderSearchIcon /> Change</button></ButtonStyled>
          </div>
          <p v-if="hasFolder" class="m-0 mt-2 text-xs text-contrast break-all leading-relaxed">{{ managedFolder }}</p>
          <p v-else class="m-0 mt-2 text-sm text-secondary">No directory selected yet.</p>
        </section>
      </div>

      <div v-if="!hasFolder" class="mx-6 mb-6 rounded-xl border border-solid border-brand/25 bg-brand/5 p-4 flex items-center justify-between gap-4 flex-wrap">
        <div><p class="m-0 text-sm font-semibold text-contrast">Select a Managed folder to continue</p><p class="m-0 mt-1 text-xs text-secondary">Needlelight can detect it automatically or you can choose it manually.</p></div>
        <ButtonStyled color="brand"><button @click="chooseFolder"><FolderSearchIcon /> Select directory</button></ButtonStyled>
      </div>

      <div v-if="error" class="mx-6 mb-6 rounded-xl border border-solid border-red-500/20 bg-red-500/5 p-4 text-sm text-secondary">
        <p class="m-0 font-semibold text-contrast">Could not load Modding API information.</p>
        <p class="m-0 mt-1">You can retry without changing your installation.</p>
        <ButtonStyled size="small" class="mt-3"><button @click="loadState">Retry</button></ButtonStyled>
      </div>
    </div>
  </div>
</template>
