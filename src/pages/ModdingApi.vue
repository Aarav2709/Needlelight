<script setup>
import { DownloadIcon, RefreshCwIcon, ShieldIcon } from '@modrinth/assets'
import { ButtonStyled, injectNotificationManager } from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'
import { onMounted, ref, computed } from 'vue'
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
const error = ref(null)
const activeGame = ref('hollow_knight')

const isSilksong = computed(() => activeGame.value === 'silksong')
const gameName = computed(() =>
  activeGame.value === 'silksong' ? 'Hollow Knight: Silksong' : 'Hollow Knight'
)
const apiTitle = computed(() =>
  isSilksong.value ? 'BepInEx' : `${gameName.value} Modding API`
)
const apiCtaLabel = computed(() => {
  if (apiInstalled.value) {
    return isSilksong.value ? 'Reinstall BepInEx' : 'Reinstall API'
  }
  return isSilksong.value ? 'Install BepInEx' : 'Install API'
})
const statusBadgeClass = computed(() =>
  apiInstalled.value ? 'bg-green-500/10 text-green-500' : 'bg-orange-500/10 text-orange-400'
)
const statusLabel = computed(() => (apiInstalled.value ? 'Installed' : 'Not installed'))

async function fetchApiStatus() {
  loading.value = true
  error.value = null
  try {
    const settings = await invoke('load_settings')
    activeGame.value = settings.game || 'hollow_knight'

    const catalog = await invoke('refresh_catalog', { fetchOfficial: true })
    apiInfo.value = catalog.api || null
    apiInstalled.value = !!catalog.api_installed
    apiEnabled.value = !!catalog.api_enabled
  } catch (err) {
    error.value = err
    console.warn('Failed to fetch API status:', err)
  } finally {
    loading.value = false
  }
}

async function installApi() {
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
  <div class="p-6 flex flex-col gap-6">
    <!-- Header -->
    <div>
      <h1 class="m-0 text-2xl font-extrabold flex items-center gap-3">
        <ShieldIcon class="w-7 h-7 text-brand" />
        {{ isSilksong ? 'BepInEx' : 'Modding API' }}
      </h1>
      <p class="text-secondary mt-1 mb-0">
        {{ isSilksong ? 'BepInEx is required for' : 'The Modding API is required for' }}
        {{ gameName }} mods to load. Install or update it here.
      </p>
    </div>

    <!-- Loading -->
    <div v-if="loading" class="text-secondary text-sm py-4">
      Loading API information...
    </div>

    <!-- Error -->
    <div v-else-if="error" class="text-secondary text-sm py-4">
      <p class="m-0 mb-2">Could not fetch API information. You may be offline.</p>
      <ButtonStyled size="small">
        <button @click="fetchApiStatus">Retry</button>
      </ButtonStyled>
    </div>

    <template v-else>
      <div class="rounded-2xl bg-bg-raised p-6 border border-solid border-surface-5 flex flex-col gap-6">
        <div class="flex items-start justify-between flex-wrap gap-4">
          <div class="flex items-start gap-4">
            <div class="w-12 h-12 rounded-xl bg-brand/10 text-brand flex items-center justify-center">
              <ShieldIcon class="w-6 h-6" />
            </div>
            <div>
              <h2 class="m-0 text-xl font-extrabold text-contrast">{{ apiTitle }}</h2>
              <div class="mt-2 flex items-center gap-2 text-xs">
                <span v-if="apiInfo" class="px-2 py-0.5 rounded bg-button-bg text-contrast font-semibold">
                  v{{ apiInfo.version }}
                </span>
                <span class="px-2 py-0.5 rounded font-semibold" :class="statusBadgeClass">
                  {{ statusLabel }}
                </span>
              </div>
            </div>
          </div>
          <ButtonStyled color="brand" :disabled="installing">
            <button @click="installApi">
              <DownloadIcon v-if="!installing" />
              <RefreshCwIcon v-else class="animate-spin" />
              {{ installing ? 'Installing...' : apiCtaLabel }}
            </button>
          </ButtonStyled>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="rounded-xl bg-bg border border-solid border-surface-5 p-4">
            <h4 class="m-0 text-sm font-semibold mb-2 text-contrast">
              {{ isSilksong ? 'What does the mod loader do?' : 'What does the API do?' }}
            </h4>
            <ul class="m-0 pl-4 text-sm text-secondary flex flex-col gap-1.5">
              <li>Patches the game to enable mod loading</li>
              <li>Provides hooks and APIs for mods to interact with the game</li>
              <li>Required for all mods from the modlinks repository</li>
              <li>Use <strong>Launch Modded</strong> vs <strong>Launch Vanilla</strong> in the sidebar</li>
            </ul>
          </div>
          <div class="rounded-xl bg-bg border border-solid border-surface-5 p-4">
            <h4 class="m-0 text-sm font-semibold mb-2 text-contrast">Where it installs</h4>
            <p class="m-0 text-sm text-secondary">
              {{ isSilksong
                ? 'Installs into the game root and creates the BepInEx folder.'
                : 'Installs into the game data Managed folder.' }}
            </p>
            <p class="m-0 mt-3 text-sm text-secondary">
              Maintained by the <strong>hk-modding</strong> community at
              <span class="text-brand underline decoration-brand/60">github.com/hk-modding/api</span>
            </p>
          </div>
        </div>
      </div>

      <div>
        <ButtonStyled type="transparent" size="small">
          <button @click="fetchApiStatus">
            <RefreshCwIcon />
            Refresh status
          </button>
        </ButtonStyled>
      </div>
    </template>
  </div>
</template>
