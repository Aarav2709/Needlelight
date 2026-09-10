<script setup>
import { DownloadIcon, RefreshCwIcon, SpinnerIcon, SteamColorIcon } from '@modrinth/assets'
import { Admonition, Badge, ButtonStyled, ProgressBar, injectNotificationManager } from '@modrinth/ui'
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
  if (installing.value) return

  try {
    // re-check folder state at click time, not just on page load
    const settings = await invoke('load_settings')
    game.value = settings.game || game.value
    hasFolder.value = !!settings.managed_folder?.trim()

    if (!hasFolder.value) {
      handleError('Please select a game directory by clicking Browse in the Library first.')
      return
    }

    installing.value = true
    installProgress.value = 0
    installStage.value = isSilksong.value ? 'Downloading BepInEx...' : 'Downloading Modding API...'

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
  <div class="p-6">
    <div v-if="loading" class="min-h-[56vh] w-full flex items-center justify-center text-center">
      <div class="inline-flex flex-col items-center gap-3 text-secondary">
        <span class="w-12 h-12 rounded-full bg-bg-raised border border-solid border-surface-5 flex items-center justify-center">
          <SpinnerIcon class="w-5 h-5 animate-spin" />
        </span>
        <span class="text-sm">Loading API information...</span>
      </div>
    </div>

    <main v-else class="max-w-3xl w-full mx-auto flex flex-col gap-6">
      <div class="flex items-center justify-between gap-6 flex-wrap pb-6 border-b border-solid border-surface-5">
        <div class="min-w-0">
          <div class="flex items-center gap-2 flex-wrap">
            <h1 class="m-0 text-xl font-extrabold text-contrast">{{ apiName }}</h1>
            <span class="text-xs font-semibold text-secondary bg-button-bg px-2.5 py-1 rounded-full">{{ apiVersion }}</span>
          </div>
          <Badge class="mt-1.5" :type="statusText" :color="!apiInstalled ? 'gray' : (apiEnabled ? 'green' : 'orange')" />
        </div>

        <ButtonStyled color="brand" :disabled="installing">
          <button @click="installApi">
            <RefreshCwIcon v-if="installing" class="animate-spin" />
            <DownloadIcon v-else />
            {{ installing ? 'Installing...' : ctaLabel }}
          </button>
        </ButtonStyled>
      </div>

      <p class="m-0 -mt-2 text-sm text-secondary leading-relaxed">
        <template v-if="isSilksong">Silksong uses <span class="text-contrast font-semibold">BepInEx</span> as its mod loader. Needlelight installs and manages the maintained Silksong BepInEx pack separately from Hollow Knight's legacy Modding API.</template>
        <template v-else>The Modding API provides the runtime that lets Hollow Knight load mods together. Needlelight currently supports the official <span class="text-contrast font-semibold">v77</span> API for the legacy Hollow Knight build.</template>
      </p>

      <p v-if="!hasFolder" class="m-0 -mt-3 text-sm text-secondary">
        Select a game directory from <span class="text-contrast font-semibold">Browse</span> before installing.
      </p>

      <div v-if="installing" class="max-w-sm">
        <ProgressBar
          :progress="installProgress"
          :max="100"
          color="brand"
          :label="installStage"
          label-class="text-sm font-semibold text-contrast"
          show-progress
          full-width
        />
      </div>

      <!-- hollow knight version compatibility -->
      <div v-if="!isSilksong" class="flex flex-col gap-4">
        <Admonition type="warning">
          <template #header>Hollow Knight version compatibility</template>
          <p class="m-0 text-sm leading-relaxed">
            Hollow Knight <span class="font-semibold">1.5.12620</span>, released March 27, 2026, moved the game to Unity 6. The official Modding API <span class="font-semibold">v77</span> targets <span class="font-semibold">1.5.78.11833</span>, the last pre-Unity 6 build. Unity 6 API support and compatible mod ports are still in development.
          </p>
        </Admonition>

        <div class="rounded-2xl border border-solid border-surface-5 bg-bg-raised p-5">
          <h3 class="m-0 flex items-center gap-2 text-sm font-bold text-contrast">
            <SteamColorIcon class="w-4 h-4 shrink-0" />
            Use the supported version
          </h3>

          <ol class="relative m-0 mt-5 p-0 list-none flex flex-col gap-5">
            <li class="relative pl-8">
              <span class="absolute left-0 top-0.5 w-5 h-5 rounded-full bg-brand text-white text-[11px] font-bold flex items-center justify-center">1</span>
              <span class="absolute left-[9px] top-6 bottom-[-1.25rem] w-px bg-surface-5"></span>
              <p class="m-0 text-sm leading-5">Open <span class="font-semibold text-contrast">Steam</span> and right-click <span class="font-semibold text-contrast">Hollow Knight</span>.</p>
            </li>
            <li class="relative pl-8">
              <span class="absolute left-0 top-0.5 w-5 h-5 rounded-full bg-brand text-white text-[11px] font-bold flex items-center justify-center">2</span>
              <span class="absolute left-[9px] top-6 bottom-[-1.25rem] w-px bg-surface-5"></span>
              <p class="m-0 text-sm leading-5">Select <span class="font-semibold text-contrast">Properties</span>.</p>
            </li>
            <li class="relative pl-8">
              <span class="absolute left-0 top-0.5 w-5 h-5 rounded-full bg-brand text-white text-[11px] font-bold flex items-center justify-center">3</span>
              <span class="absolute left-[9px] top-6 bottom-[-1.25rem] w-px bg-surface-5"></span>
              <p class="m-0 text-sm leading-5">Open <span class="font-semibold text-contrast">Betas</span> or <span class="font-semibold text-contrast">Game Versions and Betas</span>.</p>
            </li>
            <li class="relative pl-8">
              <span class="absolute left-0 top-0.5 w-5 h-5 rounded-full bg-brand text-white text-[11px] font-bold flex items-center justify-center">4</span>
              <span class="absolute left-[9px] top-6 bottom-[-1.25rem] w-px bg-surface-5"></span>
              <p class="m-0 text-sm leading-5">Select <span class="font-semibold text-contrast">1.5.78.11833</span> (Previous version).</p>
            </li>
            <li class="relative pl-8">
              <span class="absolute left-0 top-0.5 w-5 h-5 rounded-full bg-brand text-white text-[11px] font-bold flex items-center justify-center">5</span>
              <p class="m-0 text-sm leading-5">Wait for Steam to finish updating, then launch Hollow Knight through Needlelight.</p>
            </li>
          </ol>
        </div>
      </div>

      <div v-if="error" class="pt-5 border-t border-solid border-surface-5 flex items-center gap-3 text-sm text-secondary">
        <span class="text-contrast font-semibold">Could not load API information.</span>
        <ButtonStyled size="small"><button @click="loadState">Retry</button></ButtonStyled>
      </div>
    </main>
  </div>
</template>
