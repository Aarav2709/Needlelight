<script setup>
import { DownloadIcon, RefreshCwIcon, ShieldIcon, CheckIcon } from '@modrinth/assets'
import { ButtonStyled, injectNotificationManager } from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'
import { onMounted, ref } from 'vue'
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

async function fetchApiStatus() {
  loading.value = true
  error.value = null
  try {
    const catalog = await invoke('refresh_catalog', { fetchOfficial: true })
    apiInfo.value = catalog.api || null

    // Check if API is installed by looking at the installed mods data
    const items = catalog.items || []
    // The API state can also be inferred from catalog metadata
    // For now, check if the managed folder has the API files
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
        Modding API
      </h1>
      <p class="text-secondary mt-1 mb-0">
        The Modding API is required for Hollow Knight mods to load. Install or update it here.
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
      <!-- API Actions Card -->
      <div class="rounded-2xl bg-bg-raised p-6 border border-solid border-surface-5 flex flex-col gap-5">
        <!-- Status + Install -->
        <div class="flex items-center justify-between flex-wrap gap-4">
          <div>
            <h3 class="m-0 text-lg font-bold text-contrast">Hollow Knight Modding API</h3>
            <p v-if="apiInfo" class="text-secondary text-sm mt-1 mb-0">
              Latest version: <strong class="text-contrast">{{ apiInfo.version }}</strong>
            </p>
          </div>
          <ButtonStyled color="brand" :disabled="installing">
            <button @click="installApi">
              <DownloadIcon v-if="!installing" />
              <RefreshCwIcon v-else class="animate-spin" />
              {{ installing ? 'Installing...' : 'Install / Update API' }}
            </button>
          </ButtonStyled>
        </div>

        <!-- Info -->
        <div class="border-t border-solid border-surface-5 pt-4">
          <h4 class="m-0 text-sm font-semibold mb-3 text-contrast">What does the API do?</h4>
          <ul class="m-0 pl-5 text-sm text-secondary flex flex-col gap-1.5">
            <li>Patches the game to enable mod loading</li>
            <li>Provides hooks and APIs for mods to interact with the game</li>
            <li>Required for all mods from the modlinks repository</li>
            <li>Use <strong>Launch Modded</strong> vs <strong>Launch Vanilla</strong> in the sidebar to play with or without mods</li>
          </ul>
        </div>

        <div class="border-t border-solid border-surface-5 pt-4">
          <h4 class="m-0 text-sm font-semibold mb-2 text-contrast">Source</h4>
          <p class="text-secondary text-sm m-0">
            Maintained by the <strong>hk-modding</strong> community at
            <span class="text-brand">github.com/hk-modding/api</span>
          </p>
        </div>
      </div>

      <!-- Refresh -->
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
