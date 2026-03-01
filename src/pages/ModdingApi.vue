<script setup>
import { DownloadIcon, RefreshCwIcon, ShieldIcon } from "@modrinth/assets";
import { ButtonStyled, injectNotificationManager } from "@modrinth/ui";
import { invoke } from "@tauri-apps/api/core";
import { ref } from "vue";
import { useRoute } from "vue-router";

import { useBreadcrumbs } from "@/store/breadcrumbs";

const { handleError } = injectNotificationManager();
const route = useRoute();
const breadcrumbs = useBreadcrumbs();

breadcrumbs.setRootContext({ name: "Modding API", link: route.path });

const loading = ref(true);
const installing = ref(false);
const apiInfo = ref(null);
const apiInstalled = ref(null);
const error = ref(null);

async function fetchApiStatus() {
  loading.value = true;
  error.value = null;
  try {
    const catalog = await invoke("refresh_catalog", { fetchOfficial: true });
    apiInfo.value = catalog.api;
    // Check installed state from backend settings
    const settings = await invoke("load_settings");
    // Re-fetch installed status
    const freshCatalog = await invoke("refresh_catalog", {
      fetchOfficial: false,
    });
    apiInstalled.value = freshCatalog;
  } catch (err) {
    error.value = err;
    console.warn("Failed to fetch API status:", err);
  } finally {
    loading.value = false;
  }
}

async function installApi() {
  installing.value = true;
  try {
    await invoke("install_api");
    await fetchApiStatus();
  } catch (err) {
    handleError(err);
  } finally {
    installing.value = false;
  }
}

await fetchApiStatus();
</script>

<template>
  <div class="p-6 flex flex-col gap-6">
    <div>
      <h1 class="m-0 text-2xl font-extrabold flex items-center gap-3">
        <ShieldIcon class="w-7 h-7 text-brand" />
        Modding API
      </h1>
      <p class="text-secondary mt-1 mb-0">
        The Modding API is required for Hollow Knight mods to load. Install or
        update it here.
      </p>
    </div>

    <div v-if="loading" class="text-secondary text-sm">
      Loading API information...
    </div>

    <div v-else-if="error" class="text-secondary text-sm">
      Could not fetch API information. You may be offline.
      <button
        class="ml-2 px-3 py-1 text-xs rounded-lg border border-solid border-surface-5 bg-button-bg cursor-pointer hover:brightness-90 transition-all"
        @click="fetchApiStatus"
      >
        Retry
      </button>
    </div>

    <template v-else>
      <!-- API Info Card -->
      <div
        class="rounded-2xl bg-bg-raised p-6 border border-solid border-surface-5 flex flex-col gap-4"
      >
        <div class="flex items-start justify-between">
          <div>
            <h3 class="m-0 text-lg font-bold text-contrast">
              Hollow Knight Modding API
            </h3>
            <p v-if="apiInfo" class="text-secondary text-sm mt-1 mb-0">
              Latest version: <strong>{{ apiInfo.version }}</strong>
            </p>
          </div>
          <div class="flex gap-2">
            <ButtonStyled color="brand" :disabled="installing">
              <button @click="installApi">
                <DownloadIcon v-if="!installing" />
                <RefreshCwIcon v-else class="animate-spin" />
                {{ installing ? "Installing..." : "Install / Update API" }}
              </button>
            </ButtonStyled>
          </div>
        </div>

        <div class="border-t border-solid border-surface-5 pt-4">
          <h4 class="m-0 text-sm font-semibold mb-2 text-contrast">
            What does the API do?
          </h4>
          <ul class="m-0 pl-5 text-sm text-secondary flex flex-col gap-1">
            <li>Patches the game assembly to enable mod loading</li>
            <li>Provides hooks and APIs for mods to interact with the game</li>
            <li>Required for all mods from the official modlinks repository</li>
            <li>
              You can toggle between modded and vanilla by managing the API
              files
            </li>
          </ul>
        </div>

        <div class="border-t border-solid border-surface-5 pt-4">
          <h4 class="m-0 text-sm font-semibold mb-2 text-contrast">Source</h4>
          <p class="text-secondary text-sm m-0">
            The API is maintained by the
            <strong>hk-modding</strong> community at
            <span class="text-brand">github.com/hk-modding/api</span>
          </p>
        </div>
      </div>

      <!-- Refresh Button -->
      <div>
        <ButtonStyled type="transparent">
          <button @click="fetchApiStatus">
            <RefreshCwIcon />
            Refresh status
          </button>
        </ButtonStyled>
      </div>
    </template>
  </div>
</template>
