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
const hasFolder = ref(false)
const error = ref(null)
const installProgress = ref(0)
const installStage = ref('Preparing download...')
let unlistenProgress = null

const game = ref('hollow_knight')
const isSilksong = computed(() => game.value === 'silksong')
const apiName = computed(() => isSilksong.value ? 'BepInEx' : 'Modding API')
const apiVersion = computed(() => apiInfo.value?.version ? `v${apiInfo.value.version}` : 'Version unavailable')
const ctaLabel = computed(() => apiInstalled.value ? (isSilksong.value ? 'Reinstall BepInEx' : 'Reinstall API') : (isSilksong.value ? 'Install BepInEx' : 'Install API'))
const statusText = computed(() => {
  if (!apiInstalled.value) return 'Not installed'
  return apiEnabled.value ? 'Installed and ready' : 'Installed, currently disabled'
})

async function loadState() {
  loading.value = true
  error.value = null
  try {
    const settings = await invoke('load_settings')
    game.value = settings.game || 'hollow_knight'
    hasFolder.value = !!settings.managed_folder?.trim()
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
  if (!hasFolder.value || installing.value) return
  installing.value = true
  installProgress.value = 0
  installStage.value = isSilksong.value ? 'Downloading BepInEx...' : 'Downloading Modding API...'
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
  <div class="min-h-full px-8 py-7">
    <div v-if="loading" class="min-h-[70vh] flex items-center justify-center text-secondary">
      <SpinnerIcon class="w-5 h-5 animate-spin" />
    </div>

    <main v-else class="max-w-5xl mx-auto">
      <header class="flex items-center justify-between gap-6 pb-5 border-b border-solid border-surface-5">
        <div class="flex items-center gap-3 min-w-0">
          <ShieldIcon class="w-6 h-6 text-brand shrink-0" />
          <div class="min-w-0">
            <div class="flex items-center gap-2 flex-wrap">
              <h1 class="m-0 text-2xl font-extrabold text-contrast">{{ apiName }}</h1>
              <span class="text-sm font-semibold text-secondary">{{ apiVersion }}</span>
            </div>
            <p class="m-0 mt-1 text-sm text-secondary">{{ statusText }}</p>
          </div>
        </div>

        <ButtonStyled color="brand" :disabled="!hasFolder || installing">
          <button @click="installApi">
            <RefreshCwIcon v-if="installing" class="animate-spin" />
            <DownloadIcon v-else />
            {{ installing ? 'Installing...' : ctaLabel }}
          </button>
        </ButtonStyled>
      </header>

      <section class="pt-6">
        <p v-if="isSilksong" class="m-0 max-w-4xl text-sm text-secondary leading-relaxed">
          Silksong uses <span class="text-contrast font-semibold">BepInEx</span> as its mod loader. Needlelight installs and manages the maintained Silksong BepInEx pack separately from Hollow Knight's legacy Modding API.
        </p>
        <p v-else class="m-0 max-w-4xl text-sm text-secondary leading-relaxed">
          The Modding API provides the runtime that lets Hollow Knight load mods together. Needlelight currently supports the official <span class="text-contrast font-semibold">v77</span> API for the legacy Hollow Knight build.
        </p>

        <p v-if="!hasFolder" class="m-0 mt-4 text-sm text-secondary">
          Select a game directory from <span class="text-contrast font-semibold">Browse</span> before installing.
        </p>

        <div v-if="installing" class="mt-6 max-w-4xl">
          <div class="flex items-center gap-3 mb-2">
            <span class="text-sm font-semibold text-contrast">{{ installStage }}</span>
          </div>
          <ProgressBar :progress="installProgress" />
        </div>
      </section>

      <section v-if="!isSilksong" class="pt-8 mt-8 border-t border-solid border-surface-5">
        <h2 class="m-0 text-lg font-bold text-contrast">Hollow Knight version compatibility</h2>
        <p class="m-0 mt-3 max-w-4xl text-sm text-secondary leading-relaxed">
          Hollow Knight <span class="text-contrast font-semibold">1.5.12620</span> launched on March 27, 2026 and moved the game to Unity 6. The official Modding API <span class="text-contrast font-semibold">v77</span> targets <span class="text-contrast font-semibold">1.5.78.11833</span>, so it is not compatible with the Unity 6 build yet. The community is still working on the Unity 6 API transition and compatible mod ports.
        </p>

        <div class="mt-6">
          <h3 class="m-0 text-sm font-bold text-contrast">Use the supported legacy build</h3>
          <ol class="m-0 mt-3 pl-5 max-w-4xl text-sm text-secondary leading-7">
            <li>Open <span class="text-contrast font-semibold">Steam</span> and right-click <span class="text-contrast font-semibold">Hollow Knight</span>.</li>
            <li>Select <span class="text-contrast font-semibold">Properties</span>.</li>
            <li>Open <span class="text-contrast font-semibold">Betas</span> or <span class="text-contrast font-semibold">Game Versions and Betas</span>.</li>
            <li>Select <span class="text-contrast font-semibold">1.5.78.11833</span> (Previous version).</li>
            <li>Wait for Steam to finish updating, then launch Hollow Knight through Needlelight.</li>
          </ol>
        </div>

        <p class="m-0 mt-4 max-w-4xl text-sm text-secondary leading-relaxed">
          Until the Modding API has an official Unity 6 release, Needlelight stays on <span class="text-contrast font-semibold">v77</span> and the supported pre-Unity 6 game build to avoid installing an incompatible API.
        </p>
      </section>

      <div v-if="error" class="pt-6 mt-6 border-t border-solid border-surface-5 text-sm text-secondary">
        <span class="text-contrast font-semibold">Could not load API information.</span>
        <ButtonStyled size="small" class="ml-3"><button @click="loadState">Retry</button></ButtonStyled>
      </div>
    </main>
  </div>
</template>
