<script setup>
import { LibraryIcon, ShieldIcon } from '@modrinth/assets'
import { ButtonStyled, injectNotificationManager } from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'
import { onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import { useBreadcrumbs } from '@/store/breadcrumbs'

const { handleError } = injectNotificationManager()
const route = useRoute()
const router = useRouter()
const breadcrumbs = useBreadcrumbs()

breadcrumbs.setRootContext({ name: 'Home', link: route.path })

const settings = ref(null)
const stats = ref({ total: 0, installed: 0, enabled: 0 })
const loading = ref(true)

onMounted(async () => {
  try {
    settings.value = await invoke('load_settings')
    const catalog = await invoke('refresh_catalog', { fetchOfficial: true })
    const items = catalog?.items ?? []
    stats.value = {
      total: items.length,
      installed: items.filter((m) => m.state?.kind === 'Installed' || m.state?.kind === 'NotInModlinks').length,
      enabled: items.filter((m) => (m.state?.kind === 'Installed' || m.state?.kind === 'NotInModlinks') && m.state?.enabled !== false).length,
    }
  } catch (err) {
    console.warn('Failed to load dashboard data:', err)
  } finally {
    loading.value = false
  }
})

const gameName = ref('')
const gameConfigured = ref(false)

onMounted(async () => {
  try {
    const s = await invoke('load_settings')
    gameName.value = s.game === 'silksong' ? 'Hollow Knight: Silksong' : 'Hollow Knight'
    gameConfigured.value = !!s.managed_folder
  } catch {
    gameName.value = 'Hollow Knight'
  }
})
</script>

<template>
  <div class="p-6 flex flex-col gap-6">
    <!-- Welcome banner -->
    <div class="rounded-2xl bg-bg-raised border border-solid border-surface-5 p-6">
      <h1 class="m-0 text-2xl font-extrabold text-contrast">
        Welcome to Needlelight
      </h1>
      <p class="text-secondary mt-2 mb-0">
        Manage your <strong class="text-contrast">{{ gameName }}</strong> mods offline.
        Browse, install, and toggle mods from the modlinks repository.
      </p>
      <div v-if="!gameConfigured && !loading" class="mt-4 p-3 rounded-lg bg-orange-500/10 border border-solid border-orange-500/30 text-orange-400 text-sm">
        ⚠ Game folder not configured. Go to <strong>Settings → Game</strong> to set up your game path.
      </div>
    </div>

    <!-- Stats cards -->
    <div v-if="!loading" class="grid grid-cols-3 gap-4">
      <div class="rounded-xl bg-bg-raised border border-solid border-surface-5 p-4 text-center">
        <p class="m-0 text-2xl font-bold text-contrast">{{ stats.total }}</p>
        <p class="m-0 text-xs text-secondary mt-1">Mods Available</p>
      </div>
      <div class="rounded-xl bg-bg-raised border border-solid border-surface-5 p-4 text-center">
        <p class="m-0 text-2xl font-bold text-brand">{{ stats.installed }}</p>
        <p class="m-0 text-xs text-secondary mt-1">Installed</p>
      </div>
      <div class="rounded-xl bg-bg-raised border border-solid border-surface-5 p-4 text-center">
        <p class="m-0 text-2xl font-bold text-green-500">{{ stats.enabled }}</p>
        <p class="m-0 text-xs text-secondary mt-1">Enabled</p>
      </div>
    </div>
    <div v-else class="text-secondary text-sm text-center py-8">
      Loading...
    </div>

    <!-- Quick actions -->
    <div class="grid grid-cols-2 gap-4">
      <button
        class="rounded-xl bg-bg-raised border border-solid border-surface-5 p-5 flex items-center gap-4 cursor-pointer hover:border-brand/30 transition-all text-left"
        @click="router.push('/library')"
      >
        <LibraryIcon class="w-8 h-8 text-brand shrink-0" />
        <div>
          <h3 class="m-0 text-sm font-bold text-contrast">Browse Mods</h3>
          <p class="m-0 text-xs text-secondary mt-1">
            Search, install, and manage mods for {{ gameName }}.
          </p>
        </div>
      </button>
      <button
        class="rounded-xl bg-bg-raised border border-solid border-surface-5 p-5 flex items-center gap-4 cursor-pointer hover:border-brand/30 transition-all text-left"
        @click="router.push('/modding-api')"
      >
        <ShieldIcon class="w-8 h-8 text-brand shrink-0" />
        <div>
          <h3 class="m-0 text-sm font-bold text-contrast">Modding API</h3>
          <p class="m-0 text-xs text-secondary mt-1">
            Install or update the modding API required for mods to load.
          </p>
        </div>
      </button>
    </div>
  </div>
</template>
