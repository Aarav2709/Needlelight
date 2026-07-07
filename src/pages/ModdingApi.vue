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
const activeGame = ref('hollow_knight')
const managedFolder = ref('')

const isSilksong = computed(() => activeGame.value === 'silksong')
const apiTitle = computed(() => 'Modding API')
const apiCtaLabel = computed(() =>
  apiInstalled.value ? 'Reinstall API' : 'Install API',
)
const statusLabel = computed(() => (apiInstalled.value ? 'Installed' : 'Not Installed'))
const installLocationLabel = computed(() => (isSilksong.value ? 'Game Root' : 'Managed Folder'))
const installLocationValue = computed(() => {
  if (!managedFolder.value) return 'Not set'
  return managedFolder.value
})

const installSummary = computed(() =>
  isSilksong.value
    ? 'installs the loader that Silksong mods need before they can run.'
    : 'patches Hollow Knight so mods can load and run.',
)
const canInstall = computed(() => managedFolder.value.trim().length > 0)

async function fetchApiStatus() {
  loading.value = true
  error.value = null
  try {
    const settings = await invoke('load_settings')
    activeGame.value = settings.game || 'hollow_knight'
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
        <span class="text-sm">Loading modding API...</span>
      </div>
    </div>

    <div v-else-if="error" class="w-full max-w-3xl rounded-2xl bg-bg-raised border border-solid border-surface-5 p-6 text-sm text-secondary">
      <p class="m-0 mb-3">Could not fetch modding API status. You may be offline.</p>
      <ButtonStyled size="small">
        <button @click="fetchApiStatus">Retry</button>
      </ButtonStyled>
    </div>

    <div v-else class="w-full max-w-4xl rounded-2xl bg-bg-raised border border-solid border-surface-5 p-6 flex flex-col gap-5">
      <div class="flex items-start justify-between gap-4 flex-wrap">
        <div class="max-w-2xl">
          <h1 class="m-0 text-2xl font-black tracking-tight text-contrast">
            {{ apiTitle }}
          </h1>
          <p class="m-0 mt-2 text-sm leading-relaxed text-secondary max-w-xl">
            {{ installSummary }}
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

      <div v-if="installing" class="flex flex-col items-center gap-3 py-2">
        <div class="w-full max-w-md">
          <ProgressBar :progress="0.65" color="brand" />
        </div>
        <p class="m-0 text-xs text-secondary">Installing the runtime. This may take a moment.</p>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
        <div class="rounded-xl bg-bg border border-solid border-surface-5 p-4">
          <h2 class="m-0 text-sm font-semibold text-contrast">what it does</h2>
          <p class="m-0 mt-2 text-sm text-secondary">
            {{ installSummary }}
          </p>
        </div>
        <div class="rounded-xl bg-bg border border-solid border-surface-5 p-4">
          <h2 class="m-0 text-sm font-semibold text-contrast">install location</h2>
          <p class="m-0 mt-2 text-sm text-secondary">
            {{ installLocationLabel }}
          </p>
          <p class="m-0 mt-1 text-xs text-secondary break-all">
            {{ installLocationValue }}
          </p>
        </div>
      </div>

      <div class="flex flex-wrap items-center gap-2 text-xs">
        <span class="px-2 py-1 rounded-full font-semibold" :class="statusLabel === 'Installed' ? 'bg-green-500/10 text-green-500' : 'bg-orange-500/10 text-orange-400'">
          {{ statusLabel }}
        </span>
        <span class="px-2 py-1 rounded-full bg-button-bg text-secondary font-medium">
          {{ apiInfo?.version ? `Version ${apiInfo.version}` : 'Version unknown' }}
        </span>
      </div>
    </div>
  </div>
</template>
