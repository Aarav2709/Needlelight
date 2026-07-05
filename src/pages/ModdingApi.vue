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
const error = ref(null)
const activeGame = ref('hollow_knight')
const managedFolder = ref('')

const isSilksong = computed(() => activeGame.value === 'silksong')
const gameName = computed(() =>
  activeGame.value === 'silksong' ? 'Hollow Knight: Silksong' : 'Hollow Knight'
)
const apiTitle = computed(() => (isSilksong.value ? 'BepInEx' : `${gameName.value} Modding API`))
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
const resourceUrl = computed(() =>
  isSilksong.value
    ? 'https://bepinex.org/'
    : 'https://github.com/hk-modding/api/releases/tag/1.5.78.11833-77'
)
const resourceLabel = computed(() => (isSilksong.value ? 'BepInEx' : 'HK Modding API v77'))
const resourceHost = computed(() => (isSilksong.value ? 'bepinex.org' : 'github.com/hk-modding/api'))
const installLocationLabel = computed(() =>
  isSilksong.value ? 'Game root' : 'Managed folder'
)
const installSummary = computed(() =>
  apiInstalled.value
    ? 'the runtime is present and Needlelight can install mods that depend on it.'
    : 'install the runtime first so mod installs have a target to patch or load from.',
)
const canInstall = computed(() => managedFolder.value.trim().length > 0)
const installLocationValue = computed(() => {
  if (!managedFolder.value) {
    return 'Not set'
  }
  if (!isSilksong.value) {
    return managedFolder.value
  }

  const separator = managedFolder.value.includes('\\') ? '\\' : '/'
  const parts = managedFolder.value.split(/[/\\]+/)
  if (
    parts.length >= 2 &&
    parts[parts.length - 1].toLowerCase() === 'managed' &&
    parts[parts.length - 2].toLowerCase().endsWith('_data')
  ) {
    return parts.slice(0, -2).join(separator)
  }
  return managedFolder.value
})

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
    handleError('Game folder not configured. Go to Settings > Game to set it up.')
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
  <div class="p-6 flex flex-col gap-6">
    <div v-if="loading" class="rounded-2xl bg-bg-raised border border-solid border-surface-5 p-6 text-secondary text-sm">
      loading api information...
    </div>

    <div v-else-if="error" class="rounded-2xl bg-bg-raised border border-solid border-surface-5 p-6 text-sm text-secondary">
      <p class="m-0 mb-3">could not fetch api information. you may be offline.</p>
      <ButtonStyled size="small">
        <button @click="fetchApiStatus">retry</button>
      </ButtonStyled>
    </div>

    <template v-else>
      <div class="rounded-2xl bg-bg-raised border border-solid border-surface-5 p-6 relative overflow-hidden">
        <div class="absolute inset-x-0 top-0 h-1 bg-gradient-to-r from-brand via-brand/60 to-transparent"></div>
        <div class="flex items-start justify-between gap-4 flex-wrap">
          <div class="max-w-2xl">
            <div class="inline-flex items-center gap-2 rounded-full bg-button-bg px-3 py-1 text-xs font-semibold text-secondary">
              <ShieldIcon class="w-4 h-4 text-brand" />
              {{ isSilksong ? 'silksong runtime' : 'hollow knight runtime' }}
            </div>
            <h1 class="m-0 mt-4 text-3xl font-black tracking-tight text-contrast">
              {{ apiTitle }}
            </h1>
            <p class="m-0 mt-3 text-sm leading-relaxed text-secondary max-w-xl">
              {{ isSilksong ? 'BepInEx' : 'the modding api' }} keeps Needlelight usable for
              {{ gameName }} mods.
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

        <div class="mt-6 grid grid-cols-1 md:grid-cols-3 gap-3">
          <div class="rounded-xl bg-bg border border-solid border-surface-5 p-4">
            <p class="m-0 text-xs uppercase tracking-wide text-secondary">status</p>
            <p class="m-0 mt-2 text-lg font-semibold text-contrast">{{ statusLabel }}</p>
            <p class="m-0 mt-1 text-xs text-secondary">{{ apiInstalled ? 'ready for installs' : 'needs setup before mod installs' }}</p>
          </div>
          <div class="rounded-xl bg-bg border border-solid border-surface-5 p-4">
            <p class="m-0 text-xs uppercase tracking-wide text-secondary">version</p>
            <p class="m-0 mt-2 text-lg font-semibold text-contrast">
              {{ apiInfo?.version ? `v${apiInfo.version}` : 'unknown' }}
            </p>
            <p class="m-0 mt-1 text-xs text-secondary">pulled from the live catalog</p>
          </div>
          <div class="rounded-xl bg-bg border border-solid border-surface-5 p-4">
            <p class="m-0 text-xs uppercase tracking-wide text-secondary">target</p>
            <p class="m-0 mt-2 text-lg font-semibold text-contrast">{{ installLocationLabel }}</p>
            <p class="m-0 mt-1 text-xs text-secondary">{{ installLocationValue }}</p>
          </div>
        </div>
      </div>

      <div class="grid grid-cols-1 xl:grid-cols-[minmax(0,1.4fr)_minmax(280px,0.8fr)] gap-4">
        <div class="rounded-2xl bg-bg-raised border border-solid border-surface-5 p-5 flex flex-col gap-4">
          <div class="flex items-start justify-between gap-4 flex-wrap">
            <div>
              <h2 class="m-0 text-base font-bold text-contrast">what it changes</h2>
              <p class="m-0 mt-1 text-sm text-secondary">
                Needlelight installs the runtime into the right game folder and keeps it ready for mod downloads.
              </p>
            </div>
            <span class="px-2 py-1 rounded-full text-xs font-semibold" :class="statusBadgeClass">
              {{ statusLabel }}
            </span>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
            <div class="rounded-xl bg-bg border border-solid border-surface-5 p-4">
              <h3 class="m-0 text-sm font-semibold text-contrast">install path</h3>
              <p class="m-0 mt-2 text-sm text-secondary">
                {{ isSilksong ? 'drops BepInEx into the game root' : 'writes the modding api into the managed folder' }}
              </p>
            </div>
            <div class="rounded-xl bg-bg border border-solid border-surface-5 p-4">
              <h3 class="m-0 text-sm font-semibold text-contrast">why it matters</h3>
              <p class="m-0 mt-2 text-sm text-secondary">
                {{ isSilksong ? 'mods need BepInEx to load at startup' : 'mods need the patched assembly before they can launch' }}
              </p>
            </div>
          </div>

          <div class="rounded-xl bg-bg border border-solid border-surface-5 p-4">
            <h3 class="m-0 text-sm font-semibold text-contrast">official resource</h3>
            <a
              class="mt-2 inline-flex text-brand underline decoration-brand/60 text-sm font-semibold"
              :href="resourceUrl"
              target="_blank"
              rel="noreferrer"
            >
              {{ resourceLabel }}
            </a>
            <p class="m-0 mt-2 text-xs text-secondary">{{ resourceHost }}</p>
          </div>
        </div>

        <div class="rounded-2xl bg-bg-raised border border-solid border-surface-5 p-5 flex flex-col gap-4">
          <div>
            <h2 class="m-0 text-base font-bold text-contrast">quick status</h2>
            <p class="m-0 mt-1 text-sm text-secondary">
              check the runtime, then go back to the library to install mods.
            </p>
          </div>

          <div class="rounded-xl bg-bg border border-solid border-surface-5 p-4">
            <p class="m-0 text-xs uppercase tracking-wide text-secondary">current folder</p>
            <div class="mt-2 break-all rounded-lg bg-button-bg px-3 py-2 text-xs text-contrast">
              {{ installLocationValue }}
            </div>
          </div>

          <div class="rounded-xl bg-bg border border-solid border-surface-5 p-4">
            <p class="m-0 text-xs uppercase tracking-wide text-secondary">refresh</p>
            <p class="m-0 mt-2 text-sm text-secondary">
              use this if the runtime was installed outside of Needlelight.
            </p>
            <div class="mt-3">
              <ButtonStyled type="transparent" size="small">
                <button @click="fetchApiStatus">
                  <RefreshCwIcon />
                  refresh status
                </button>
              </ButtonStyled>
            </div>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>
