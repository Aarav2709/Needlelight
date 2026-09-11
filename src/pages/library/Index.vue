<script setup>
import {
  DownloadIcon,
  RefreshCwIcon,
  SearchIcon,
  SpinnerIcon,
  XIcon,
  FolderSearchIcon,
} from '@modrinth/assets'
import { ButtonStyled, Toggle, injectNotificationManager } from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useRoute } from 'vue-router'

import { applyGameTheme } from '@/helpers/game-theme'
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
const managedFolder = ref('')
const promptingForFolder = ref(false)
const modProgress = ref(new Map())
let unlistenModProgress = null

const isSilksong = computed(() => activeGame.value === 'silksong')
const hasGameFolder = computed(() => managedFolder.value.trim().length > 0)
const gameName = computed(() => isSilksong.value ? 'Hollow Knight Silksong' : 'Hollow Knight')

function setBusy(name, busy) {
  const next = new Set(busyMods.value)
  if (busy) next.add(name)
  else next.delete(name)
  busyMods.value = next
}

async function loadGame() {
  try {
    const settings = await invoke('load_settings')
    activeGame.value = settings.game || 'hollow_knight'
    managedFolder.value = settings.managed_folder || ''
    applyGameTheme(activeGame.value)
    return settings
  } catch (err) {
    activeGame.value = 'hollow_knight'
    managedFolder.value = ''
    applyGameTheme('hollow_knight')
    throw err
  }
}

async function chooseManagedFolder(force = false) {
  if (promptingForFolder.value) return false
  promptingForFolder.value = true
  try {
    if (!force && hasGameFolder.value) return true
    const folder = await open({
      directory: true,
      title: `Select ${gameName.value}'s Managed Folder`,
    })
    if (!folder) return false

    const settings = await invoke('load_settings')
    settings.managed_folder = folder
    settings.managed_folders = settings.managed_folders || {}
    settings.managed_folders[activeGame.value] = folder
    await invoke('save_settings', { settings })

    managedFolder.value = folder
    catalog.value = null
    catalogError.value = null
    await fetchCatalog()
    return true
  } catch (err) {
    handleError(err)
    return false
  } finally {
    promptingForFolder.value = false
  }
}

async function switchGame(game) {
  if (game === activeGame.value || switchingGame.value) return
  switchingGame.value = true
  try {
    const settings = await invoke('load_settings')
    settings.game = game
    settings.managed_folder = settings.managed_folders?.[game] || ''
    await invoke('save_settings', { settings })
    await loadGame()

    // Let backend auto-detection win before showing the picker.
    if (!hasGameFolder.value) {
      await loadGame()
    }
    if (!hasGameFolder.value) {
      await chooseManagedFolder(true)
      return
    }
    await fetchCatalog()
  } catch (err) {
    handleError(err)
  } finally {
    switchingGame.value = false
  }
}

async function fetchCatalog() {
  // Never render stale mods while the current game has no configured path.
  if (!hasGameFolder.value) {
    catalog.value = null
    catalogLoading.value = false
    catalogError.value = null
    return
  }

  catalogLoading.value = true
  catalogError.value = null
  try {
    catalog.value = await invoke('refresh_catalog', { fetchOfficial: true })
  } catch (err) {
    catalog.value = null
    catalogError.value = err
  } finally {
    catalogLoading.value = false
  }
}

const allMods = computed(() => catalog.value?.items ?? [])
const installedMods = computed(() => allMods.value.filter(m => m.state?.kind === 'installed' || m.state?.kind === 'not_in_modlinks'))
const availableMods = computed(() => allMods.value.filter(m => m.state?.kind === 'not_installed'))
const filteredMods = computed(() => {
  let list = activeFilter.value === 'installed'
    ? installedMods.value
    : activeFilter.value === 'available'
      ? availableMods.value
      : allMods.value

  if (searchQuery.value.trim()) {
    const q = searchQuery.value.toLowerCase().trim()
    list = list.filter(m =>
      m.name.toLowerCase().includes(q) ||
      m.description?.toLowerCase().includes(q) ||
      m.authors?.some(a => a.toLowerCase().includes(q)) ||
      m.tags?.some(t => t.toLowerCase().includes(q)),
    )
  }
  return list
})

function isInstalled(mod) {
  return mod.state?.kind === 'installed' || mod.state?.kind === 'not_in_modlinks'
}
function isEnabled(mod) {
  return mod.state?.enabled !== false
}
function needsUpdate(mod) {
  return mod.state?.kind === 'installed' && mod.state?.updated === false
}
function formatModName(name) {
  if (!isSilksong.value) return name
  const parts = name.split('-')
  const withoutAuthor = parts.length <= 1 ? name : parts.slice(1).join('-')
  return withoutAuthor.replace(/_/g, ' ')
}
function formatDependency(name) {
  return isSilksong.value ? formatModName(name) : name
}

async function installMod(modName) {
  if (!hasGameFolder.value) {
    await chooseManagedFolder(true)
    if (!hasGameFolder.value) return
  }
  setBusy(modName, true)
  try {
    const started = new Map(modProgress.value)
    started.set(modName, 0)
    modProgress.value = started
    await invoke('install_mod', { name: modName })
    const finished = new Map(modProgress.value)
    finished.delete(modName)
    modProgress.value = finished
    await fetchCatalog()
  } catch (err) {
    handleError(err)
  } finally {
    setBusy(modName, false)
  }
}

async function updateMod(modName) {
  if (!hasGameFolder.value) return
  setBusy(modName, true)
  try {
    const started = new Map(modProgress.value)
    started.set(modName, 0)
    modProgress.value = started
    await invoke('install_mod', { name: modName })
    const finished = new Map(modProgress.value)
    finished.delete(modName)
    modProgress.value = finished
    await fetchCatalog()
  } catch (err) {
    handleError(err)
  } finally {
    setBusy(modName, false)
  }
}

async function uninstallMod(modName) {
  if (!hasGameFolder.value) return
  setBusy(modName, true)
  try {
    await invoke('uninstall_mod', { name: modName })
    await fetchCatalog()
  } catch (err) {
    handleError(err)
  } finally {
    setBusy(modName, false)
  }
}

async function toggleMod(modName, enable) {
  if (!hasGameFolder.value) return
  setBusy(modName, true)
  try {
    await invoke('toggle_mod', { name: modName, enable })
    await fetchCatalog()
  } catch (err) {
    handleError(err)
  } finally {
    setBusy(modName, false)
  }
}

onMounted(async () => {
  unlistenModProgress = await listen('mod-install-progress', (event) => {
    const payload = event.payload || {}
    const name = payload.item_name
    if (!name) return
    const progress = Number(payload.progress ?? 0)
    const next = new Map(modProgress.value)
    if (progress >= 100) next.delete(name)
    else next.set(name, Math.max(0, Math.min(100, progress)))
    modProgress.value = next
  })
  try {
    await loadGame()
  } catch (err) {
    console.warn(err)
  }
  if (!hasGameFolder.value) {
    // Backend auto-detection has a chance first; otherwise prompt immediately.
    await chooseManagedFolder(true)
  } else {
    await fetchCatalog()
  }
})

onUnmounted(() => {
  unlistenModProgress?.()
})
</script>

<template>
  <div class="p-6 flex flex-col gap-5">
    <div class="flex items-center justify-between gap-4 flex-wrap">
      <div class="flex items-center gap-3">
        <div class="flex rounded-xl bg-bg-raised border border-solid border-surface-5 p-1">
          <button
            v-for="game in [{ key: 'hollow_knight', label: 'Hollow Knight' }, { key: 'silksong', label: 'Silksong' }]"
            :key="game.key"
            class="px-4 py-2 text-sm font-semibold rounded-lg border-none cursor-pointer transition-all"
            :class="activeGame === game.key ? 'bg-brand text-white' : 'bg-transparent text-secondary hover:text-contrast hover:bg-button-bg'"
            :disabled="switchingGame"
            @click="switchGame(game.key)"
          >{{ game.label }}</button>
        </div>
        <p v-if="hasGameFolder" class="text-secondary text-sm m-0">
          {{ allMods.length }} mods · {{ installedMods.length }} installed
        </p>
      </div>
      <div class="flex items-center gap-2">
        <ButtonStyled type="transparent" size="small">
          <button @click="fetchCatalog" :disabled="catalogLoading || !hasGameFolder">
            <RefreshCwIcon :class="{ 'animate-spin': catalogLoading }" />
            Refresh
          </button>
        </ButtonStyled>
        <ButtonStyled type="transparent" size="small">
          <button @click="chooseManagedFolder(true)">
            <FolderSearchIcon />
            {{ hasGameFolder ? 'Change folder' : 'Browse' }}
          </button>
        </ButtonStyled>
      </div>
    </div>

    <div class="flex gap-3 items-center flex-wrap">
      <div class="flex items-center gap-2 flex-1 min-w-[200px] bg-bg-raised rounded-xl border border-solid border-surface-5 px-3 py-2">
        <SearchIcon class="w-4 h-4 text-secondary shrink-0" />
        <input v-model="searchQuery" type="text" placeholder="Search mods by name, author, or tag..." class="bg-transparent border-none outline-none text-contrast text-sm w-full placeholder:text-secondary" />
        <button v-if="searchQuery" class="bg-transparent border-none p-0 cursor-pointer text-secondary hover:text-contrast transition-colors" @click="searchQuery = ''">
          <XIcon class="w-4 h-4" />
        </button>
      </div>
      <div class="flex gap-1 rounded-xl bg-bg-raised border border-solid border-surface-5 p-1">
        <button
          v-for="f in [{ key: 'all', label: 'All' }, { key: 'installed', label: 'Installed' }, { key: 'available', label: 'Available' }]"
          :key="f.key"
          class="px-3 py-1.5 text-xs font-medium rounded-lg border-none cursor-pointer transition-all"
          :class="activeFilter === f.key ? 'bg-brand text-white' : 'bg-transparent text-secondary hover:text-contrast hover:bg-button-bg'"
          @click="activeFilter = f.key"
        >{{ f.label }}<template v-if="f.key === 'installed'"> ({{ installedMods.length }})</template><template v-else-if="f.key === 'available'"> ({{ availableMods.length }})</template></button>
      </div>
    </div>

    <div v-if="!hasGameFolder" class="min-h-[56vh] flex items-center justify-center px-6 text-center">
      <p class="m-0 max-w-lg text-base font-medium text-secondary">
        Please select a directory by clicking <span class="text-contrast font-semibold">Browse</span> on the top left to continue.
      </p>
    </div>

    <Transition v-else enter-active-class="transition-opacity duration-200" leave-active-class="transition-opacity duration-150" enter-from-class="opacity-0" leave-to-class="opacity-0" mode="out-in">
      <div :key="`${activeGame}-${managedFolder}`">
        <div v-if="catalogLoading" class="flex min-h-[56vh] w-full items-center justify-center text-center">
          <div class="inline-flex flex-col items-center gap-3 text-secondary"><span class="w-12 h-12 rounded-full bg-bg-raised border border-solid border-surface-5 flex items-center justify-center"><SpinnerIcon class="w-5 h-5 animate-spin" /></span><span class="text-sm">Loading mod catalog...</span></div>
        </div>

        <div v-else-if="catalogError" class="rounded-2xl bg-bg-raised border border-solid border-surface-5 p-8 min-h-[40vh] flex flex-col items-center justify-center text-center">
          <h2 class="m-0 text-lg font-bold text-contrast">Could not load the mod catalog</h2>
          <p class="m-0 mt-2 max-w-lg text-sm text-secondary">Check your connection or change the configured game directory.</p>
          <ButtonStyled class="mt-4" color="brand"><button @click="fetchCatalog">Retry</button></ButtonStyled>
        </div>

        <div v-else-if="filteredMods.length === 0" class="text-secondary text-sm py-12 text-center">
          <template v-if="searchQuery">No mods match "<strong class="text-contrast">{{ searchQuery }}</strong>"</template>
          <template v-else-if="activeFilter === 'installed'">No mods installed yet. Switch to Available to browse the catalog.</template>
          <template v-else>No mods available for this game.</template>
        </div>

        <div v-else class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-3">
          <div
            v-for="mod in filteredMods"
            :key="mod.name"
            class="mod-card rounded-xl border border-solid border-surface-5 p-4 flex flex-col gap-2 transition-all hover:border-brand/30 relative overflow-hidden"
            :style="{ '--download-progress': `${modProgress.get(mod.name) ?? 0}%` }"
          >
            <div
              v-if="modProgress.has(mod.name)"
              class="mod-download-fill"
              aria-hidden="true"
            ></div>
            <div class="relative z-[1] flex items-start justify-between gap-2">
              <div class="flex-1 min-w-0">
                <h4 class="m-0 font-semibold text-contrast text-sm truncate">{{ formatModName(mod.name) }}</h4>
                <p class="text-secondary text-xs mt-1 mb-0 line-clamp-2 leading-relaxed">{{ mod.description || 'No description' }}</p>
              </div>
            </div>
            <div class="relative z-[1] flex items-center gap-2 flex-wrap">
              <span class="text-xs text-secondary">v{{ mod.version }}</span>
              <span v-if="mod.authors?.length" class="text-xs text-secondary">by {{ mod.authors.join(', ') }}</span>
              <span v-for="tag in (mod.tags || []).slice(0, 3)" :key="tag" class="text-xs text-secondary bg-button-bg px-1.5 py-0.5 rounded">{{ tag }}</span>
            </div>
            <div v-if="mod.dependencies?.length" class="relative z-[1] text-xs text-secondary italic">Requires: {{ mod.dependencies.map(formatDependency).join(', ') }}</div>
            <div class="relative z-[1] flex items-center gap-3 mt-auto pt-2">
              <template v-if="isInstalled(mod)">
                <div class="flex items-center gap-2">
                  <Toggle :model-value="isEnabled(mod)" :disabled="busyMods.has(mod.name)" @update:model-value="(v) => toggleMod(mod.name, v)" />
                  <span class="text-xs text-secondary">{{ isEnabled(mod) ? 'Enabled' : 'Disabled' }}</span>
                </div>
                <span v-if="needsUpdate(mod)" class="ml-auto text-[11px] font-semibold text-brand bg-brand/10 px-2 py-1 rounded-md">Update available</span>
                <button v-if="needsUpdate(mod)" class="px-3 py-1.5 text-xs rounded-lg border-none bg-brand text-white cursor-pointer hover:brightness-95 transition-all disabled:opacity-50 disabled:cursor-not-allowed font-medium flex items-center gap-1" :disabled="busyMods.has(mod.name)" @click="updateMod(mod.name)">
                  <RefreshCwIcon class="w-3.5 h-3.5" :class="{ 'animate-spin': busyMods.has(mod.name) }" />
                  {{ busyMods.has(mod.name) ? 'Updating...' : 'Update' }}
                </button>
                <button class="ml-auto px-3 py-1.5 text-xs rounded-lg border-0 outline-none text-red-500 bg-red-500/10 cursor-pointer hover:bg-red-500/20 transition-all disabled:opacity-50 disabled:cursor-not-allowed" :class="{ 'ml-0': needsUpdate(mod) }" :disabled="busyMods.has(mod.name)" @click="uninstallMod(mod.name)">Uninstall</button>
              </template>
              <template v-else>
                <button class="px-3 py-1.5 text-xs rounded-lg border-none bg-brand text-white cursor-pointer hover:brightness-95 transition-all font-medium flex items-center gap-1 disabled:opacity-50 disabled:cursor-not-allowed" :disabled="busyMods.has(mod.name)" @click="installMod(mod.name)">
                  <DownloadIcon class="w-3.5 h-3.5" :class="{ 'animate-pulse': busyMods.has(mod.name) }" />
                  {{ busyMods.has(mod.name) ? 'Installing...' : 'Install' }}
                </button>
              </template>
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.mod-card {
  background: var(--color-bg-raised);
}

.mod-download-fill {
  position: absolute;
  inset: 0 auto 0 0;
  width: var(--download-progress);
  background: color-mix(in srgb, var(--color-brand) 18%, transparent);
  pointer-events: none;
  transition: width 120ms ease-out;
}
</style>
