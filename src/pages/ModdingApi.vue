<script setup>
import { DownloadIcon, RefreshCwIcon, SpinnerIcon } from '@modrinth/assets'
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
const error = ref(null)
const managedFolder = ref('')
const game = ref('hollow_knight')

const apiCtaLabel = computed(() => (apiInstalled.value ? 'Reinstall API' : 'Install API'))
const canInstall = computed(() => managedFolder.value.trim().length > 0)
const gameName = computed(() => game.value === 'silksong' ? 'Hollow Knight Silksong' : 'Hollow Knight')

async function loadStatus() {
  loading.value = true
  error.value = null
  try {
    const settings = await invoke('load_settings')
    game.value = settings.game || 'hollow_knight'
    managedFolder.value = settings.managed_folder || ''

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

async function selectManagedFolder() {
  try {
    const detected = await invoke('auto_detect_managed_folder', { game: game.value }).catch(() => null)
    if (detected) {
      const settings = await invoke('load_settings')
      settings.managed_folder = detected
      await invoke('save_settings', { settings })
      await loadStatus()
      return
    }

    const folder = await open({
      directory: true,
      title: `Select ${gameName.value}'s Managed Folder`,
    })
    if (!folder) return

    const settings = await invoke('load_settings')
    settings.managed_folder = folder
    await invoke('save_settings', { settings })
    await loadStatus()
  } catch (err) {
    handleError(err)
  }
}

async function installApi() {
  if (!canInstall.value) {
    await selectManagedFolder()
    if (!canInstall.value) return
  }

  installing.value = true
  try {
    await invoke('install_api')
    await loadStatus()
  } catch (err) {
    handleError(err)
  } finally {
    installing.value = false
  }
}

onMounted(async () => {
  await loadStatus()
  if (!managedFolder.value) {
    await selectManagedFolder()
  }
})
</script>

<template>
  <div class="p-6 min-h-full flex items-center justify-center">
    <div v-if="loading" class="flex min-h-[60vh] w-full items-center justify-center">
      <div class="inline-flex flex-col items-center gap-3 text-secondary">
        <SpinnerIcon class="w-5 h-5 animate-spin" />
        <span class="text-sm">Loading Modding API...</span>
      </div>
    </div>

    <div v-else-if="error" class="w-full max-w-3xl rounded-2xl bg-bg-raised border border-solid border-surface-5 p-6 text-sm text-secondary">
      <p class="m-0 mb-3">Could not fetch Modding API status. You may be offline.</p>
      <ButtonStyled size="small"><button @click="loadStatus">Retry</button></ButtonStyled>
    </div>

    <section v-else class="w-full max-w-3xl rounded-2xl bg-bg-raised border border-solid border-surface-5 overflow-hidden">
      <div class="p-6 md:p-7">
        <div class="flex items-start justify-between gap-6 flex-wrap">
          <div class="min-w-0">
            <div class="flex items-center gap-2 flex-wrap">
              <h1 class="m-0 text-3xl font-black tracking-tight text-contrast">Modding API</h1>
              <span class="px-2 py-1 rounded-md bg-button-bg text-secondary text-xs font-bold">v{{ apiInfo?.version || '?' }}</span>
            </div>
            <div class="flex items-center gap-2 mt-2 text-sm">
              <span class="w-2 h-2 rounded-full" :class="apiInstalled ? 'bg-green-500' : 'bg-secondary'" />
              <span :class="apiInstalled ? 'text-green-500' : 'text-secondary'">
                {{ apiInstalled ? (apiEnabled ? 'Installed and enabled' : 'Installed and disabled') : 'Not installed' }}
              </span>
            </div>
          </div>

          <ButtonStyled color="brand" :disabled="installing || !canInstall">
            <button @click="installApi">
              <RefreshCwIcon v-if="installing" class="animate-spin" />
              <DownloadIcon v-else />
              {{ installing ? 'Installing...' : apiCtaLabel }}
            </button>
          </ButtonStyled>
        </div>

        <p class="m-0 mt-6 max-w-2xl text-sm text-secondary leading-relaxed">
          The Modding API is the runtime Hollow Knight uses to load mods. Install it before installing mods, and reinstall it when you need to refresh the API files for the current game version.
        </p>

        <div class="mt-6 flex items-center justify-between gap-4 flex-wrap border-t border-solid border-surface-5 pt-4">
          <div class="min-w-0">
            <div class="text-xs uppercase tracking-wider text-secondary font-semibold">Managed folder</div>
            <div class="mt-1 text-sm text-contrast truncate max-w-2xl">{{ managedFolder || 'No directory selected' }}</div>
          </div>
          <ButtonStyled type="transparent" size="small">
            <button @click="selectManagedFolder">Change directory</button>
          </ButtonStyled>
        </div>

        <div v-if="installing" class="mt-5 flex items-center gap-2 text-xs text-secondary">
          <SpinnerIcon class="w-4 h-4 animate-spin" />
          Writing and verifying the API files...
        </div>
      </div>
    </section>
  </div>
</template>
