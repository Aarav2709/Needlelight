<script setup>
import { DownloadIcon, RefreshCwIcon, SpinnerIcon } from '@modrinth/assets'
import { ButtonStyled, ProgressBar, injectNotificationManager } from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'
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
const error = ref(null)
const managedFolder = ref('')

const apiCtaLabel = computed(() =>
  apiInstalled.value ? 'Reinstall API' : 'Install API',
)
const canInstall = computed(() => managedFolder.value.trim().length > 0)

async function fetchApiStatus() {
  loading.value = true
  error.value = null
  try {
    const settings = await invoke('load_settings')
    managedFolder.value = settings.managed_folder || ''

    const catalog = await invoke('refresh_catalog', { fetchOfficial: true })
    apiInfo.value = catalog.api || null
    apiInstalled.value = !!catalog.api_installed
  } catch (err) {
    error.value = err
    console.warn('Failed to fetch API status:', err)
  } finally {
    loading.value = false
  }
}

async function installApi() {
  if (!canInstall.value) {
    handleError('Game folder not configured. Go to Settings > Game first.')
    return
  }

  installing.value = true
  try {
    await invoke('install_api')
    await fetchApiStatus()
  } catch (err) {
    handleError(err)
  } finally {
    installing.value = false
  }
}

onMounted(() => fetchApiStatus())
</script>

<template>
  <div class="p-6 min-h-full flex items-center justify-center">
    <div v-if="loading" class="flex min-h-[60vh] w-full items-center justify-center">
      <div class="inline-flex flex-col items-center gap-3 text-secondary">
        <span class="w-12 h-12 rounded-full bg-bg-raised border border-solid border-surface-5 flex items-center justify-center">
          <SpinnerIcon class="w-5 h-5 animate-spin" />
        </span>
        <span class="text-sm">Loading Modding API...</span>
      </div>
    </div>

    <div v-else-if="error" class="w-full max-w-3xl rounded-2xl bg-bg-raised border border-solid border-surface-5 p-6 text-sm text-secondary">
      <p class="m-0 mb-3">Could not fetch Modding API status. You may be offline.</p>
      <ButtonStyled size="small">
        <button @click="fetchApiStatus">Retry</button>
      </ButtonStyled>
    </div>

    <div v-else class="w-full max-w-2xl rounded-2xl bg-bg-raised border border-solid border-surface-5 p-6 flex flex-col gap-5 shadow-lg shadow-black/10">
      <div class="flex items-start justify-between gap-5 flex-wrap">
        <div class="min-w-0">
          <div class="flex items-center gap-2 flex-wrap">
            <h1 class="m-0 text-2xl font-black tracking-tight text-contrast">Modding API</h1>
            <span class="px-2 py-0.5 rounded-md bg-button-bg text-secondary text-xs font-bold">
              {{ apiInfo?.version ? `v${apiInfo.version}` : 'v?' }}
            </span>
          </div>
          <p class="m-0 mt-1 text-sm" :class="apiInstalled ? 'text-green-500' : 'text-secondary'">
            <span class="inline-block w-1.5 h-1.5 rounded-full mr-1.5 align-middle" :class="apiInstalled ? 'bg-green-500' : 'bg-secondary'" />
            {{ apiInstalled ? 'Installed and ready' : 'Not installed' }}
          </p>
        </div>

        <ButtonStyled color="brand" :disabled="installing || !canInstall">
          <button @click="installApi">
            <DownloadIcon v-if="!installing" />
            <RefreshCwIcon v-else class="animate-spin" />
            {{ installing ? 'Installing...' : apiCtaLabel }}
          </button>
        </ButtonStyled>
      </div>

      <div class="border-t border-solid border-surface-5 pt-4">
        <p class="m-0 text-sm text-secondary leading-relaxed">
          The Modding API provides the runtime Hollow Knight and its mods use to load
          together. Install it once before using mods; reinstall it to refresh the
          current API files. Needlelight installs it directly into the selected game's
          Managed folder.
        </p>
      </div>

      <div v-if="installing" class="flex flex-col gap-3">
        <div class="w-full">
          <ProgressBar :progress="0.65" color="brand" />
        </div>
        <p class="m-0 text-xs text-secondary">Installing the runtime. This may take a moment.</p>
      </div>

    </div>
  </div>
</template>
