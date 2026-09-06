<script setup>
import { DownloadIcon, RefreshCwIcon, ShieldIcon, SpinnerIcon } from '@modrinth/assets'
import { ButtonStyled, injectNotificationManager } from '@modrinth/ui'
import ProgressBar from '@/components/ui/ProgressBar.vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { computed, onMounted, onUnmounted, ref } from 'vue'
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
const installProgress = ref(0)
const installStage = ref('Preparing download...')
let unlistenProgress = null

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

async function installApi() {
  if (!hasFolder.value) return
  installing.value = true
  installProgress.value = 0
  installStage.value = 'Preparing download...'
  try {
    await invoke('install_api')
    installProgress.value = 100
    installStage.value = 'Installation complete'
    await loadState()
  } catch (err) {
    handleError(err)
  } finally {
    installing.value = false
  }
}

onMounted(async () => {
  unlistenProgress = await listen('api-install-progress', (event) => {
    const payload = event.payload || {}
    installProgress.value = Math.max(0, Math.min(100, Number(payload.progress ?? 0)))
    if (payload.stage) installStage.value = payload.stage
  })
  await loadState()
})

onUnmounted(() => {
  unlistenProgress?.()
})
</script>

<template>
  <div class="min-h-full p-8">
    <div v-if="loading" class="min-h-[70vh] flex flex-col items-center justify-center text-secondary gap-3">
      <SpinnerIcon class="w-5 h-5 animate-spin" />
      <span class="text-sm">Loading Modding API...</span>
    </div>

    <div v-else class="max-w-4xl mx-auto">
      <header class="flex items-start justify-between gap-6 pb-6 border-b border-solid border-surface-5">
        <div class="flex items-start gap-4 min-w-0">
          <div class="text-brand pt-1 shrink-0"><ShieldIcon class="w-6 h-6" /></div>
          <div class="min-w-0">
            <div class="flex items-center gap-2 flex-wrap">
              <h1 class="m-0 text-2xl font-extrabold text-contrast">Modding API</h1>
              <span class="text-sm font-bold text-secondary">{{ apiVersion }}</span>
            </div>
            <p class="m-0 mt-2 text-sm text-secondary">{{ statusText }}</p>
          </div>
        </div>
        <ButtonStyled color="brand" :disabled="installing || !hasFolder">
          <button @click="installApi">
            <RefreshCwIcon v-if="installing" class="animate-spin" />
            <DownloadIcon v-else />
            {{ installing ? 'Installing...' : ctaLabel }}
          </button>
        </ButtonStyled>
      </header>

      <section class="pt-7">
        <p class="m-0 text-sm text-secondary leading-relaxed max-w-3xl">The Modding API provides the runtime that lets Hollow Knight load mods together. Needlelight currently supports the official v77 API for the legacy Hollow Knight build.</p>

        <div v-if="!hasFolder" class="mt-4 text-sm text-secondary">Select a game directory from <span class="text-contrast font-semibold">Browse</span> before installing the API.</div>

        <div v-if="installing" class="mt-7 max-w-3xl">
          <div class="flex items-center justify-between gap-4 mb-2">
            <span class="text-sm font-semibold text-contrast">{{ installStage }}</span>
            <span class="text-xs text-secondary tabular-nums">{{ installProgress }}%</span>
          </div>
          <ProgressBar :progress="installProgress" />
        </div>
      </section>

      <section class="pt-9 mt-9 border-t border-solid border-surface-5">
        <h2 class="m-0 text-lg font-bold text-contrast">Hollow Knight version compatibility</h2>
        <p class="m-0 mt-3 text-sm text-secondary leading-relaxed max-w-3xl">The current Hollow Knight release is <span class="text-contrast font-semibold">1.5.12620</span>, which moved the game to Unity 6. The official Modding API v77 is for <span class="text-contrast font-semibold">1.5.78.11833</span>, so it cannot be used with the Unity 6 build. Unity 6 support and compatible mod ports are still being worked on by the modding community.</p>

        <div class="mt-6">
          <h3 class="m-0 text-sm font-bold text-contrast">Use the supported legacy build</h3>
          <ol class="m-0 mt-3 pl-5 text-sm text-secondary leading-7 max-w-3xl">
            <li>Open <span class="text-contrast font-semibold">Steam</span> and right-click Hollow Knight.</li>
            <li>Select <span class="text-contrast font-semibold">Properties</span>.</li>
            <li>Open <span class="text-contrast font-semibold">Betas</span> / <span class="text-contrast font-semibold">Game Versions and Betas</span>.</li>
            <li>Select <span class="text-contrast font-semibold">1.5.78.11833</span>, the previous game build.</li>
            <li>Let Steam finish the download, then launch Hollow Knight through Needlelight.</li>
          </ol>
        </div>

        <p class="m-0 mt-5 text-xs text-secondary leading-relaxed max-w-3xl">Until the official Modding API has a Unity 6 release, Needlelight stays on v77 and the supported pre-Unity 6 game build to avoid installing an incompatible API.</p>
      </section>

      <div v-if="error" class="pt-7 mt-7 border-t border-solid border-surface-5 text-sm text-secondary">
        <p class="m-0 font-semibold text-contrast">Could not load Modding API information.</p>
        <p class="m-0 mt-1">You can retry without changing your installation.</p>
        <ButtonStyled size="small" class="mt-3"><button @click="loadState">Retry</button></ButtonStyled>
      </div>
    </div>
  </div>
</template>
