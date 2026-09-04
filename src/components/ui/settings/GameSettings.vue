<script setup>
import { FolderSearchIcon } from "@modrinth/assets";
import { ButtonStyled, Toggle, injectNotificationManager } from "@modrinth/ui";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { onMounted, ref, computed } from "vue";


const backendSettings = ref(null);
const loading = ref(true);
const { handleError } = injectNotificationManager();

const gameName = computed(() =>
  backendSettings.value?.game === "silksong"
    ? "Hollow Knight Silksong"
    : "Hollow Knight",
);

async function loadBackendSettings() {
  loading.value = true;
  try {
    backendSettings.value = await invoke("load_settings");
  } catch (err) {
    handleError(err);
  } finally {
    loading.value = false;
  }
}

async function saveBackendSettings() {
  if (!backendSettings.value) return;
  try {
    await invoke("save_settings", { settings: backendSettings.value });
  } catch (err) {
    handleError(err);
  }
}

async function browseFolder() {
  if (!backendSettings.value) return;
  const selected = await open({
    directory: true,
    title: `Select ${gameName.value} Managed folder`,
  });
  if (selected) {
    backendSettings.value.managed_folder = selected;
    await saveBackendSettings();
  }
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
    <!-- Managed Folder -->
    <div>
      <h3 class="m-0 text-sm font-semibold text-contrast mb-1">
        {{ gameName }}: Game Folder
      </h3>
      <p class="text-secondary text-xs mb-3">
        Path to the game's <code class="text-xs bg-button-bg px-1 py-0.5 rounded">Managed</code> directory
        (e.g. <code class="text-xs bg-button-bg px-1 py-0.5 rounded">{{ gameName }}_Data/Managed</code>).
      </p>

      <div class="flex gap-2 items-center w-full min-w-0">
        <input
          v-model="backendSettings.managed_folder"
          type="text"
          :placeholder="`/path/to/${gameName}_Data/Managed`"
          class="flex-1 min-w-0 bg-bg-raised rounded-lg border border-solid border-surface-5 px-3 py-2 text-sm text-contrast outline-none placeholder:text-secondary"
          @input="debouncedSave"
        />
        <ButtonStyled size="small">
          <button @click="browseFolder">
            <FolderSearchIcon />
            Browse
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
        Use a custom ModLinks.xml URL for this game instead of its default
        catalog.
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

  </div>
  <div v-else-if="loading" class="text-secondary text-sm p-4">
    Loading game settings...
  </div>
</template>
