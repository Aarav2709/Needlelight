<script setup>
import { FolderSearchIcon } from "@modrinth/assets";
import { ButtonStyled, Toggle } from "@modrinth/ui";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { ref, watch } from "vue";

const backendSettings = ref(null);
const loading = ref(true);
const saving = ref(false);
const detecting = ref(false);
const error = ref(null);

async function loadBackendSettings() {
  loading.value = true;
  try {
    backendSettings.value = await invoke("load_settings");
  } catch (err) {
    error.value = err;
  } finally {
    loading.value = false;
  }
}

async function saveBackendSettings() {
  if (!backendSettings.value) return;
  saving.value = true;
  try {
    await invoke("save_settings", { settings: backendSettings.value });
  } catch (err) {
    error.value = err;
  } finally {
    saving.value = false;
  }
}

async function autoDetect() {
  detecting.value = true;
  try {
    const folder = await invoke("auto_detect_managed_folder", {
      game: backendSettings.value.game,
    });
    if (folder) {
      backendSettings.value.managed_folder = folder;
      await saveBackendSettings();
    } else {
      error.value =
        "Could not auto-detect game folder. Please browse manually.";
      setTimeout(() => (error.value = null), 5000);
    }
  } catch (err) {
    error.value = err;
  } finally {
    detecting.value = false;
  }
}

async function browseFolder() {
  const selected = await open({
    directory: true,
    title: "Select Hollow Knight Managed folder",
  });
  if (selected) {
    backendSettings.value.managed_folder = selected;
    await saveBackendSettings();
  }
}

function setGame(game) {
  backendSettings.value.game = game;
  saveBackendSettings();
}

// Debounced save for text inputs
let saveTimeout;
function debouncedSave() {
  clearTimeout(saveTimeout);
  saveTimeout = setTimeout(() => saveBackendSettings(), 500);
}

await loadBackendSettings();
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
        Game Managed Folder
      </h3>
      <p class="text-secondary text-xs mb-3">
        Path to the game's <code>Managed</code> directory (e.g.
        <code>Hollow Knight_Data/Managed</code>).
      </p>
      <div class="flex gap-2 items-center">
        <input
          v-model="backendSettings.managed_folder"
          type="text"
          placeholder="/path/to/Hollow Knight_Data/Managed"
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
      <div class="mt-2">
        <ButtonStyled type="transparent" size="small">
          <button @click="autoDetect" :disabled="detecting">
            {{ detecting ? "Detecting..." : "Auto-detect" }}
          </button>
        </ButtonStyled>
      </div>
    </div>

    <!-- Custom Modlinks -->
    <div>
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

    <!-- Error display -->
    <div v-if="error" class="text-red-500 text-xs">
      {{ typeof error === "string" ? error : JSON.stringify(error) }}
    </div>
  </div>
  <div v-else-if="loading" class="text-secondary text-sm">
    Loading settings...
  </div>
</template>
