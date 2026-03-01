<script setup>
import { CheckIcon, FolderSearchIcon, XIcon } from "@modrinth/assets";
import { ButtonStyled, Toggle } from "@modrinth/ui";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { onMounted, ref, computed } from "vue";

import { applyGameTheme } from "@/helpers/game-theme";

const backendSettings = ref(null);
const loading = ref(true);
const saving = ref(false);
const detecting = ref(false);
const error = ref(null);
const success = ref(null);

const gameName = computed(() =>
  backendSettings.value?.game === "silksong"
    ? "Hollow Knight: Silksong"
    : "Hollow Knight",
);

const folderConfigured = computed(
  () => backendSettings.value?.managed_folder?.length > 0,
);

async function loadBackendSettings() {
  loading.value = true;
  error.value = null;
  try {
    backendSettings.value = await invoke("load_settings");
  } catch (err) {
    error.value =
      typeof err === "string" ? err : "Failed to load game settings.";
  } finally {
    loading.value = false;
  }
}

async function saveBackendSettings() {
  if (!backendSettings.value) return;
  saving.value = true;
  try {
    await invoke("save_settings", { settings: backendSettings.value });
    error.value = null;
  } catch (err) {
    error.value = typeof err === "string" ? err : "Failed to save settings.";
  } finally {
    saving.value = false;
  }
}

function showSuccess(msg) {
  success.value = msg;
  setTimeout(() => (success.value = null), 4000);
}

function showError(msg) {
  error.value = msg;
  setTimeout(() => (error.value = null), 8000);
}

async function autoDetect() {
  detecting.value = true;
  error.value = null;
  success.value = null;
  try {
    const folder = await invoke("auto_detect_managed_folder", {
      game: backendSettings.value.game,
    });
    if (folder) {
      backendSettings.value.managed_folder = folder;
      await saveBackendSettings();
      showSuccess(`Found ${gameName.value} at: ${folder}`);
    } else {
      showError(
        `${gameName.value} was not detected on this system. Please browse to the game's Managed folder manually, or check that the game is installed.`,
      );
    }
  } catch (err) {
    const msg = typeof err === "string" ? err : String(err);
    if (msg.includes("not found") || msg.includes("not detect")) {
      showError(
        `Could not find ${gameName.value}. Make sure the game is installed, then use "Browse" to select the Managed folder manually.`,
      );
    } else {
      showError(msg);
    }
  } finally {
    detecting.value = false;
  }
}

async function browseFolder() {
  const selected = await open({
    directory: true,
    title: `Select ${gameName.value} Managed folder`,
  });
  if (selected) {
    backendSettings.value.managed_folder = selected;
    await saveBackendSettings();
    showSuccess(`Game folder set to: ${selected}`);
  }
}

function setGame(game) {
  backendSettings.value.game = game;
  error.value = null;
  success.value = null;
  applyGameTheme(game);
  saveBackendSettings();
}

// Debounced save for text inputs
let saveTimeout;
function debouncedSave() {
  clearTimeout(saveTimeout);
  saveTimeout = setTimeout(() => saveBackendSettings(), 500);
}

onMounted(() => loadBackendSettings());
</script>

<template>
  <div class="flex flex-col gap-6" v-if="backendSettings">
    <!-- Game Selection -->
    <div>
      <h3 class="m-0 text-sm font-semibold text-contrast mb-1">Active Game</h3>
      <p class="text-secondary text-xs mb-3">
        Switch between Hollow Knight and Hollow Knight: Silksong mod profiles.
      </p>
      <div class="flex gap-2">
        <button
          class="px-4 py-2 text-sm rounded-lg border border-solid cursor-pointer transition-all font-medium"
          :class="
            backendSettings.game === 'hollow_knight'
              ? 'bg-brand text-white border-brand'
              : 'bg-button-bg text-secondary border-surface-5 hover:text-contrast hover:brightness-90'
          "
          @click="setGame('hollow_knight')"
        >
          Hollow Knight
        </button>
        <button
          class="px-4 py-2 text-sm rounded-lg border border-solid cursor-pointer transition-all font-medium"
          :class="
            backendSettings.game === 'silksong'
              ? 'bg-brand text-white border-brand'
              : 'bg-button-bg text-secondary border-surface-5 hover:text-contrast hover:brightness-90'
          "
          @click="setGame('silksong')"
        >
          Silksong
        </button>
      </div>
    </div>

    <!-- Managed Folder -->
    <div>
      <h3 class="m-0 text-sm font-semibold text-contrast mb-1">
        {{ gameName }} — Game Folder
      </h3>
      <p class="text-secondary text-xs mb-3">
        Path to the game's <code class="text-xs bg-button-bg px-1 py-0.5 rounded">Managed</code> directory
        (e.g. <code class="text-xs bg-button-bg px-1 py-0.5 rounded">{{ gameName }}_Data/Managed</code>).
      </p>

      <!-- Folder status indicator -->
      <div
        v-if="folderConfigured"
        class="flex items-center gap-2 mb-3 px-3 py-2 rounded-lg text-xs"
        :class="'bg-green-500/10 text-green-500'"
      >
        <CheckIcon class="w-3.5 h-3.5 shrink-0" />
        Game folder configured
      </div>
      <div
        v-else
        class="flex items-center gap-2 mb-3 px-3 py-2 rounded-lg text-xs bg-orange-500/10 text-orange-500"
      >
        <XIcon class="w-3.5 h-3.5 shrink-0" />
        No game folder set — use Auto-detect or Browse to configure
      </div>

      <div class="flex gap-2 items-center">
        <input
          v-model="backendSettings.managed_folder"
          type="text"
          :placeholder="`/path/to/${gameName}_Data/Managed`"
          class="flex-1 bg-bg-raised rounded-lg border border-solid border-surface-5 px-3 py-2 text-sm text-contrast outline-none placeholder:text-secondary"
          @input="debouncedSave"
        />
        <ButtonStyled size="small">
          <button @click="browseFolder">
            <FolderSearchIcon />
            Browse
          </button>
        </ButtonStyled>
      </div>
      <div class="mt-2 flex items-center gap-2">
        <ButtonStyled type="transparent" size="small">
          <button @click="autoDetect" :disabled="detecting">
            {{ detecting ? "Searching..." : "Auto-detect" }}
          </button>
        </ButtonStyled>
        <span v-if="detecting" class="text-xs text-secondary">
          Looking for {{ gameName }} on your system...
        </span>
      </div>
    </div>

    <!-- Custom Modlinks (HK only) -->
    <div v-if="backendSettings.game === 'hollow_knight'">
      <h3 class="m-0 text-sm font-semibold text-contrast mb-1">
        Custom Modlinks
      </h3>
      <p class="text-secondary text-xs mb-3">
        Use a custom ModLinks.xml URL instead of the official hk-modding
        repository.
      </p>
      <div class="flex items-center gap-3 mb-2">
        <Toggle
          :model-value="backendSettings.use_custom_modlinks"
          @update:model-value="
            (v) => {
              backendSettings.use_custom_modlinks = v;
              saveBackendSettings();
            }
          "
        />
        <span class="text-sm text-contrast">Enable custom modlinks</span>
      </div>
      <input
        v-if="backendSettings.use_custom_modlinks"
        v-model="backendSettings.custom_modlinks_uri"
        type="text"
        placeholder="https://example.com/ModLinks.xml"
        class="w-full bg-bg-raised rounded-lg border border-solid border-surface-5 px-3 py-2 text-sm text-contrast outline-none placeholder:text-secondary"
        @input="debouncedSave"
      />
    </div>

    <!-- Silksong note -->
    <div
      v-if="backendSettings.game === 'silksong'"
      class="px-4 py-3 rounded-lg bg-brand/5 border border-solid border-brand/20"
    >
      <h3 class="m-0 text-sm font-semibold text-contrast mb-1">
        Silksong Mods
      </h3>
      <p class="text-secondary text-xs m-0 leading-relaxed">
        Silksong mods are sourced from
        <strong class="text-contrast">Thunderstore</strong>. The mod catalog
        in your Library will show available community packages for Silksong.
      </p>
    </div>

    <!-- GitHub Mirror -->
    <div>
      <h3 class="m-0 text-sm font-semibold text-contrast mb-1">
        GitHub Mirror
      </h3>
      <p class="text-secondary text-xs mb-3">
        If GitHub is blocked, use a mirror to download mods.
      </p>
      <div class="flex items-center gap-3 mb-2">
        <Toggle
          :model-value="backendSettings.use_github_mirror"
          @update:model-value="
            (v) => {
              backendSettings.use_github_mirror = v;
              saveBackendSettings();
            }
          "
        />
        <span class="text-sm text-contrast">Enable GitHub mirror</span>
      </div>
      <input
        v-if="backendSettings.use_github_mirror"
        v-model="backendSettings.github_mirror_format"
        type="text"
        placeholder="https://mirror.example.com/{url}"
        class="w-full bg-bg-raised rounded-lg border border-solid border-surface-5 px-3 py-2 text-sm text-contrast outline-none placeholder:text-secondary"
        @input="debouncedSave"
      />
    </div>

    <!-- Low Storage Mode -->
    <div>
      <div class="flex items-center gap-3">
        <Toggle
          :model-value="backendSettings.low_storage_mode"
          @update:model-value="
            (v) => {
              backendSettings.low_storage_mode = v;
              saveBackendSettings();
            }
          "
        />
        <div>
          <h3 class="m-0 text-sm font-semibold text-contrast">
            Low Storage Mode
          </h3>
          <p class="text-secondary text-xs mt-0.5 mb-0">
            Skip caching downloaded archives to save disk space.
          </p>
        </div>
      </div>
    </div>

    <!-- Success message -->
    <Transition name="fade">
      <div
        v-if="success"
        class="flex items-center gap-2 px-3 py-2 rounded-lg text-xs bg-green-500/10 text-green-500"
      >
        <CheckIcon class="w-3.5 h-3.5 shrink-0" />
        {{ success }}
      </div>
    </Transition>

    <!-- Error display -->
    <Transition name="fade">
      <div
        v-if="error"
        class="flex items-start gap-2 px-3 py-2 rounded-lg text-xs bg-red-500/10 text-red-500"
      >
        <XIcon class="w-3.5 h-3.5 shrink-0 mt-0.5" />
        <span>{{ typeof error === "string" ? error : JSON.stringify(error) }}</span>
      </div>
    </Transition>
  </div>
  <div v-else-if="loading" class="text-secondary text-sm p-4">
    Loading game settings...
  </div>
</template>
