<script setup>
import {
  CheckIcon,
  DownloadIcon,
  RefreshCwIcon,
  SearchIcon,
  XIcon,
} from '@modrinth/assets'
import { ButtonStyled, injectNotificationManager } from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, ref } from 'vue'
import { useRoute } from 'vue-router'

import { useBreadcrumbs } from '@/store/breadcrumbs'

const { handleError } = injectNotificationManager()
const route = useRoute()
const breadcrumbs = useBreadcrumbs()

breadcrumbs.setRootContext({ name: 'Library', link: route.path })

const catalog = ref(null)
const catalogLoading = ref(true)
const catalogError = ref(null)
const searchQuery = ref('')
const activeFilter = ref('all')
const busyMods = ref(new Set())
const activeGame = ref('hollow_knight')
const switchingGame = ref(false)

async function loadGame() {
  try {
    const settings = await invoke('load_settings')
    activeGame.value = settings.game || 'hollow_knight'
  } catch {
    activeGame.value = 'hollow_knight'
  }
}

async function switchGame(game) {
  if (game === activeGame.value || switchingGame.value) return
  switchingGame.value = true
  try {
    const settings = await invoke('load_settings')
    settings.game = game
    await invoke('save_settings', { settings })
    activeGame.value = game
    await fetchCatalog()
  } catch (err) {
    handleError(err)
  } finally {
    switchingGame.value = false
  }
}

async function fetchCatalog() {
  catalogLoading.value = true
  catalogError.value = null
  try {
    catalog.value = await invoke('refresh_catalog', { fetchOfficial: true })
  } catch (err) {
    catalogError.value = err
    console.warn('Failed to fetch mod catalog:', err)
  } finally {
    catalogLoading.value = false
  }
}

const allMods = computed(() => catalog.value?.items ?? [])

const installedMods = computed(() =>
  allMods.value.filter(
    (m) => m.state?.kind === 'Installed' || m.state?.kind === 'NotInModlinks',
  ),
)

const availableMods = computed(() =>
  allMods.value.filter((m) => m.state?.kind === 'NotInstalled'),
)

const filteredMods = computed(() => {
  let list
  switch (activeFilter.value) {
    case 'installed':
      list = installedMods.value
      break
    case 'available':
      list = availableMods.value
      break
    default:
      list = allMods.value
  }

  if (searchQuery.value.trim()) {
    const q = searchQuery.value.toLowerCase().trim()
    list = list.filter(
      (m) =>
        m.name.toLowerCase().includes(q) ||
        m.description?.toLowerCase().includes(q) ||
        m.authors?.some((a) => a.toLowerCase().includes(q)) ||
        m.tags?.some((t) => t.toLowerCase().includes(q)),
    )
  }

  return list
})

function isInstalled(mod) {
  return mod.state?.kind === 'Installed' || mod.state?.kind === 'NotInModlinks'
}

function isEnabled(mod) {
  return mod.state?.enabled !== false
}

async function installMod(modName) {
  busyMods.value.add(modName)
  try {
    await invoke('install_mod', { name: modName })
    await fetchCatalog()
  } catch (err) {
    handleError(err)
  } finally {
    busyMods.value.delete(modName)
  }
}

async function uninstallMod(modName) {
  busyMods.value.add(modName)
  try {
    await invoke('uninstall_mod', { name: modName })
    await fetchCatalog()
  } catch (err) {
    handleError(err)
  } finally {
    busyMods.value.delete(modName)
  }
}

async function toggleMod(modName, enable) {
  busyMods.value.add(modName)
  try {
    await invoke('toggle_mod', { name: modName, enable })
    await fetchCatalog()
  } catch (err) {
    handleError(err)
  } finally {
    busyMods.value.delete(modName)
  }
}

onMounted(async () => {
  await loadGame()
  await fetchCatalog()
})
</script>

<template>
  <div class="p-6 flex flex-col gap-5">
    <!-- Game Toggle Header -->
    <div class="flex items-center justify-between gap-4 flex-wrap">
      <div class="flex items-center gap-3">
        <div class="flex rounded-xl bg-bg-raised border border-solid border-surface-5 p-1">
          <button
            class="px-4 py-2 text-sm font-semibold rounded-lg border-none cursor-pointer transition-all"
            :class="
              activeGame === 'hollow_knight'
                ? 'bg-brand text-white'
                : 'bg-transparent text-secondary hover:text-contrast hover:bg-button-bg'
            "
            :disabled="switchingGame"
            @click="switchGame('hollow_knight')"
          >
            Hollow Knight
          </button>
          <button
            class="px-4 py-2 text-sm font-semibold rounded-lg border-none cursor-pointer transition-all"
            :class="
              activeGame === 'silksong'
                ? 'bg-brand text-white'
                : 'bg-transparent text-secondary hover:text-contrast hover:bg-button-bg'
            "
            :disabled="switchingGame"
            @click="switchGame('silksong')"
          >
            Silksong
          </button>
        </div>
        <p class="text-secondary text-sm m-0">
          {{ allMods.length }} mods &middot; {{ installedMods.length }} installed
        </p>
      </div>
      <ButtonStyled type="transparent" size="small">
        <button @click="fetchCatalog" :disabled="catalogLoading">
          <RefreshCwIcon :class="{ 'animate-spin': catalogLoading }" />
          Refresh
        </button>
      </ButtonStyled>
    </div>

    <!-- Search + Filter bar -->
    <div class="flex gap-3 items-center flex-wrap">
      <div
        class="flex items-center gap-2 flex-1 min-w-[200px] bg-bg-raised rounded-xl border border-solid border-surface-5 px-3 py-2"
      >
        <SearchIcon class="w-4 h-4 text-secondary shrink-0" />
        <input
          v-model="searchQuery"
          type="text"
          placeholder="Search mods by name, author, or tag..."
          class="bg-transparent border-none outline-none text-contrast text-sm w-full placeholder:text-secondary"
        />
        <button
          v-if="searchQuery"
          class="bg-transparent border-none p-0 cursor-pointer text-secondary hover:text-contrast transition-colors"
          @click="searchQuery = ''"
        >
          <XIcon class="w-4 h-4" />
        </button>
      </div>
      <div class="flex gap-1 rounded-xl bg-bg-raised border border-solid border-surface-5 p-1">
        <button
          v-for="f in [
            { key: 'all', label: 'All' },
            { key: 'installed', label: 'Installed' },
            { key: 'available', label: 'Available' },
          ]"
          :key="f.key"
          class="px-3 py-1.5 text-xs font-medium rounded-lg border-none cursor-pointer transition-all"
          :class="
            activeFilter === f.key
              ? 'bg-brand text-white'
              : 'bg-transparent text-secondary hover:text-contrast hover:bg-button-bg'
          "
          @click="activeFilter = f.key"
        >
          {{ f.label }}
          <template v-if="f.key === 'installed'">({{ installedMods.length }})</template>
          <template v-else-if="f.key === 'available'">({{ availableMods.length }})</template>
        </button>
      </div>
    </div>

    <!-- Loading state -->
    <div v-if="catalogLoading" class="text-secondary text-sm py-8 text-center">
      Loading mod catalog from modlinks...
    </div>

    <!-- Error state -->
    <div v-else-if="catalogError" class="text-secondary text-sm py-8 text-center">
      <p class="m-0 mb-2">Could not load the mod catalog. You may be offline.</p>
      <ButtonStyled size="small">
        <button @click="fetchCatalog">Retry</button>
      </ButtonStyled>
    </div>

    <!-- Empty search results -->
    <div
      v-else-if="filteredMods.length === 0"
      class="text-secondary text-sm py-8 text-center"
    >
      <template v-if="searchQuery">
        No mods match "<strong class="text-contrast">{{ searchQuery }}</strong>"
      </template>
      <template v-else-if="activeFilter === 'installed'">
        No mods installed yet. Switch to "Available" to browse the catalog.
      </template>
      <template v-else> No mods available for this game. </template>
    </div>

    <!-- Mod grid -->
    <div v-else class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-3">
      <div
        v-for="mod in filteredMods"
        :key="mod.name"
        class="rounded-xl bg-bg-raised border border-solid border-surface-5 p-4 flex flex-col gap-2 transition-all hover:border-brand/30"
      >
        <div class="flex items-start justify-between gap-2">
          <div class="flex-1 min-w-0">
            <h4 class="m-0 font-semibold text-contrast text-sm truncate">
              {{ mod.name }}
            </h4>
            <p class="text-secondary text-xs mt-1 mb-0 line-clamp-2 leading-relaxed">
              {{ mod.description || 'No description' }}
            </p>
          </div>
          <span
            v-if="isInstalled(mod)"
            class="shrink-0 flex items-center gap-1 text-xs font-medium px-2 py-0.5 rounded-full"
            :class="
              isEnabled(mod)
                ? 'text-green-500 bg-green-500/10'
                : 'text-orange-500 bg-orange-500/10'
            "
          >
            <CheckIcon class="w-3 h-3" />
            {{ isEnabled(mod) ? 'Enabled' : 'Disabled' }}
          </span>
        </div>

        <div class="flex items-center gap-2 flex-wrap">
          <span class="text-xs text-secondary">v{{ mod.version }}</span>
          <span v-if="mod.authors?.length" class="text-xs text-secondary">
            by {{ mod.authors.join(', ') }}
          </span>
          <span
            v-for="tag in (mod.tags || []).slice(0, 3)"
            :key="tag"
            class="text-xs text-secondary bg-button-bg px-1.5 py-0.5 rounded"
          >
            {{ tag }}
          </span>
        </div>

        <div v-if="mod.dependencies?.length" class="text-xs text-secondary">
          Requires: {{ mod.dependencies.join(', ') }}
        </div>

        <div class="flex gap-2 mt-auto pt-1">
          <template v-if="isInstalled(mod)">
            <button
              class="px-3 py-1.5 text-xs rounded-lg border border-solid border-surface-5 bg-button-bg cursor-pointer hover:brightness-90 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
              :disabled="busyMods.has(mod.name)"
              @click="toggleMod(mod.name, !isEnabled(mod))"
            >
              {{ isEnabled(mod) ? 'Disable' : 'Enable' }}
            </button>
            <button
              class="px-3 py-1.5 text-xs rounded-lg border border-solid border-red-500/30 text-red-500 bg-transparent cursor-pointer hover:bg-red-500/10 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
              :disabled="busyMods.has(mod.name)"
              @click="uninstallMod(mod.name)"
            >
              Uninstall
            </button>
          </template>
          <template v-else>
            <button
              class="px-3 py-1.5 text-xs rounded-lg border-none bg-brand text-white cursor-pointer hover:brightness-90 transition-all font-medium flex items-center gap-1 disabled:opacity-50 disabled:cursor-not-allowed"
              :disabled="busyMods.has(mod.name)"
              @click="installMod(mod.name)"
            >
              <DownloadIcon class="w-3.5 h-3.5" />
              {{ busyMods.has(mod.name) ? 'Installing...' : 'Install' }}
            </button>
          </template>
        </div>
      </div>
    </div>
  </div>
</template>
