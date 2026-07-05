<script setup>
import { DownloadIcon, RefreshCwIcon, ShieldIcon } from '@modrinth/assets'
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
const gameName = computed(() =>
  activeGame.value === 'silksong' ? 'Hollow Knight: Silksong' : 'Hollow Knight',
)
const apiTitle = computed(() => (isSilksong.value ? 'BepInEx' : 'Modding API'))
const apiCtaLabel = computed(() =>
  apiInstalled.value
    ? isSilksong.value
      ? 'Reinstall BepInEx'
      : 'Reinstall API'
    : isSilksong.value
      ? 'Install BepInEx'
      : 'Install API',
)
const statusLabel = computed(() => (apiInstalled.value ? 'Installed' : 'Not Installed'))
const installLocationLabel = computed(() =>
  isSilksong.value ? 'Game Root' : 'Managed Folder',
)
const installLocationValue = computed(() => {
  if (!managedFolder.value) return 'Not set'
  if (!isSilksong.value) return managedFolder.value

  const parts = managedFolder.value.split(/[/\\]+/)
  if (
    parts.length >= 2 &&
    parts[parts.length - 1].toLowerCase() === 'managed' &&
    parts[parts.length - 2].toLowerCase().endsWith('_data')
  ) {
    return parts.slice(0, -2).join('/')
  }

  return managedFolder.value
})

const installSummary = computed(() =>
  isSilksong.value
    ? 'BepInEx is the loader Silksong mods need before they can run.'
    : 'The Modding API patches Hollow Knight so mods can load and run.',
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
  <div class="p-6 flex flex-col gap-5">
    <div v-if="loading" class="rounded-2xl bg-bg-raised border border-solid border-surface-5 p-6 text-secondary text-sm">
      Loading Modding API status...
    </div>

    <div v-else-if="error" class="rounded-2xl bg-bg-raised border border-solid border-surface-5 p-6 text-sm text-secondary">
      <p class="m-0 mb-3">Could not fetch Modding API status. You may be offline.</p>
      <ButtonStyled size="small">
        <button @click="fetchApiStatus">Retry</button>
      </ButtonStyled>
    </div>

    <div v-else class="rounded-2xl bg-bg-raised border border-solid border-surface-5 p-6 flex flex-col gap-4">
      <div class="flex items-start justify-between gap-4 flex-wrap">
        <div class="max-w-2xl">
          <div class="inline-flex items-center gap-2 rounded-full bg-button-bg px-3 py-1 text-xs font-semibold text-secondary">
            <ShieldIcon class="w-4 h-4 text-brand" />
            {{ isSilksong ? 'Silksong Runtime' : 'Hollow Knight Runtime' }}
          </div>
          <h1 class="m-0 mt-4 text-2xl font-black tracking-tight text-contrast">
            {{ apiTitle }}
          </h1>
          <p class="m-0 mt-2 text-sm leading-relaxed text-secondary">
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

      <div v-if="installing" class="space-y-2">
        <ProgressBar :progress="0.7" color="brand" />
        <p class="m-0 text-xs text-secondary">Installing the runtime. This may take a moment.</p>
      </div>

      <div class="flex flex-wrap items-center gap-2 text-xs">
        <span class="px-2 py-1 rounded-full font-semibold" :class="statusLabel === 'Installed' ? 'bg-green-500/10 text-green-500' : 'bg-orange-500/10 text-orange-400'">
          {{ statusLabel }}
        </span>
        <span class="px-2 py-1 rounded-full bg-button-bg text-secondary font-medium">
          {{ apiInfo?.version ? `Version ${apiInfo.version}` : 'Version unknown' }}
        </span>
        <span class="px-2 py-1 rounded-full bg-button-bg text-secondary font-medium">
          {{ installLocationLabel }}: {{ installLocationValue }}
        </span>
      </div>

      <p class="m-0 text-sm text-secondary">
        What it does: {{ installSummary }}
      </p>
    </div>
  </div>
</template>
