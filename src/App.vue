<script setup>
import {
  DownloadIcon,
  LeftArrowIcon,
  MaximizeIcon,
  MinimizeIcon,
  RefreshCwIcon,
  RestoreIcon,
  RightArrowIcon,
  XIcon,
} from "@modrinth/assets";
import {
  defineMessages,
  NotificationPanel,
  ProgressSpinner,
  provideNotificationManager,
  useVIntl,
} from "@modrinth/ui";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { saveWindowState, StateFlags } from "@tauri-apps/plugin-window-state";
import { computed, onMounted, onUnmounted, ref, useTemplateRef, watch } from "vue";
import { RouterView, useRoute, useRouter } from "vue-router";

import Breadcrumbs from "@/components/ui/Breadcrumbs.vue";
import ErrorModal from "@/components/ui/ErrorModal.vue";
import AppSettingsModal from "@/components/ui/modal/AppSettingsModal.vue";
import KeyboardShortcutsModal from "@/components/ui/modal/KeyboardShortcutsModal.vue";
import ModpackEditorModal from "@/components/ui/modal/ModpackEditorModal.vue";
import WelcomeModal from "@/components/ui/modal/WelcomeModal.vue";
import NavRail from "@/components/ui/NavRail.vue";
import RunningAppBar from "@/components/ui/RunningAppBar.vue";
import UpdateToast from "@/components/ui/UpdateToast.vue";
import { useCheckDisableMouseover } from "@/composables/macCssFix.js";
import { useGameSetup } from "@/composables/setup";
import { useShortcut } from "@/composables/shortcuts";
import { command_listener, warning_listener } from "@/helpers/events.js";
import { applyGameTheme } from "@/helpers/game-theme";
import { gameName } from "@/helpers/games";
import { modpackRoute } from "@/helpers/modpacks";
import { initialize_state } from "@/helpers/state";
import {
  areUpdatesEnabled,
  enqueueUpdateForInstallation,
  getUpdateSize,
  isDev,
  isNetworkMetered,
} from "@/helpers/utils.js";
import {
  provideAppUpdateDownloadProgress,
  subscribeToDownloadProgress,
} from "@/providers/download-progress.ts";
import { useError } from "@/store/error.js";
import { useGames } from "@/store/games";
import { useModpacks } from "@/store/modpacks";
import { usePreferences } from "@/store/preferences";
import { useLoading, useTheming } from "@/store/state";
import { useUi } from "@/store/ui";

import { AppNotificationManager } from "./providers/app-notifications";

const themeStore = useTheming();
const preferences = usePreferences();
const games = useGames();
const modpacks = useModpacks();
const ui = useUi();
const settingsModal = useTemplateRef('settingsModal');
const welcomeModal = useTemplateRef('welcomeModal');

// Apply saved UI preferences (theme, density, motion) before the shell first renders.
preferences.apply();

const notificationManager = new AppNotificationManager();
provideNotificationManager(notificationManager);
const { handleError, addNotification } = notificationManager;

const isDevEnvironment = ref(false);

const stateInitialized = ref(false);

const isMaximized = ref(false);

onMounted(async () => {
  await useCheckDisableMouseover();

  document.querySelector("body").addEventListener("click", handleClick);
  document.querySelector("body").addEventListener("auxclick", handleAuxClick);

  checkUpdates();
});

onUnmounted(async () => {
  document.querySelector("body").removeEventListener("click", handleClick);
  document
    .querySelector("body")
    .removeEventListener("auxclick", handleAuxClick);

  await unlistenUpdateDownload?.();
});

const { formatMessage } = useVIntl();
const messages = defineMessages({
  updateInstalledToastTitle: {
    id: "app.update.complete-toast.title",
    defaultMessage: "Version {version} was successfully installed!",
  },
  updateInstalledToastText: {
    id: "app.update.complete-toast.text",
    defaultMessage: "Click here to view the changelog.",
  },
  reloadToUpdate: {
    id: "app.update.reload-to-update",
    defaultMessage: "Reload to install update",
  },
  downloadUpdate: {
    id: "app.update.download-update",
    defaultMessage: "Download update",
  },
  downloadingUpdate: {
    id: "app.update.downloading-update",
    defaultMessage: "Downloading update ({percent}%)",
  },
});

async function setupApp() {
  const dev = await isDev();
  isDevEnvironment.value = dev;
  stateInitialized.value = true;

  // The active game (profile) tints the app with its accent color once settings load.
  applyGameTheme(localStorage.getItem("needlelight.game") ?? "hollow_knight");
  const gamesLoaded = games.load().catch((err) => console.warn("Failed to load settings", err));
  const modpacksLoaded = modpacks
    .load()
    .catch((err) => console.warn("Failed to load modpacks", err));
  // The loading screen stays up until there's something to show.
  Promise.allSettled([gamesLoaded, modpacksLoaded]).then(hideSplash);


  const currentWindow = getCurrentWindow();

  // Needlelight owns the title bar. Keep native OS decorations disabled even
  // if an older persisted window state tries to restore them.
  await currentWindow.setDecorations(false);
  isMaximized.value = await currentWindow.isMaximized();

  await currentWindow.onResized(async () => {
    isMaximized.value = await getCurrentWindow().isMaximized();
  });

  if (!dev)
    document.addEventListener("contextmenu", (event) => event.preventDefault());

  await warning_listener((e) =>
    addNotification({
      title: "Warning",
      text: e.message,
      type: "warn",
    }),
  );
}

/** Fade out the loading screen from index.html. */
function hideSplash() {
  const splash = document.getElementById("splash");
  if (!splash || splash.classList.contains("is-done")) return;
  splash.classList.add("is-done");
  setTimeout(() => splash.remove(), 400);
}
// Never leave it up if something stalls; errors show in the app instead.
setTimeout(hideSplash, 15000);

const stateFailed = ref(false);
watch(stateFailed, (failed) => failed && hideSplash());
initialize_state()
  .then(() => {
    setupApp().catch((err) => {
      stateFailed.value = true;
      console.error(err);
      error.showError(err, null, false, "state_init");
    });
  })
  .catch((err) => {
    stateFailed.value = true;
    console.error("Failed to initialize app", err);
    error.showError(err, null, false, "state_init");
  });

const handleClose = async () => {
  await saveWindowState(StateFlags.ALL);
  await getCurrentWindow().close();
};

const router = useRouter();
const route = useRoute();

const loading = useLoading();
loading.setEnabled(false);

const error = useError();
const errorModal = ref();

void command_listener(handleCommand).catch(() => null);
async function handleCommand(e) {
  if (!e) return;
  // Handle scarab:// URL scheme commands
  console.log("Received command:", e);
}

const appUpdateDownload = {
  progress: ref(0),
  version: ref(),
};
let unlistenUpdateDownload;

const downloadProgress = computed(() => appUpdateDownload.progress.value);
const downloadPercent = computed(() =>
  Math.trunc(appUpdateDownload.progress.value * 100),
);

const metered = ref(true);
const finishedDownloading = ref(false);
const restarting = ref(false);
const updateToastDismissed = ref(false);
const availableUpdate = ref(null);
const updateSize = ref(null);
const updatesEnabled = ref(true);
async function checkUpdates() {
  if (!(await areUpdatesEnabled())) {
    console.log(
      "Skipping update check as updates are disabled in this build or environment",
    );
    updatesEnabled.value = false;
    return;
  }

  async function performCheck() {
    const update = await invoke("plugin:updater|check").catch(() => null);
    if (!update) {
      console.log("No update available");
      return;
    }

    const isExistingUpdate = update.version === availableUpdate.value?.version;

    if (isExistingUpdate) {
      console.log("Update is already known");
      return;
    }

    appUpdateDownload.progress.value = 0;
    finishedDownloading.value = false;
    updateToastDismissed.value = false;

    console.log(`Update ${update.version} is available.`);

    metered.value = await isNetworkMetered();
    if (!metered.value) {
      console.log("Starting download of update");
      downloadUpdate(update);
    } else {
      console.log(`Metered connection detected, not auto-downloading update.`);
    }

    getUpdateSize(update.rid).then((size) => (updateSize.value = size));

    availableUpdate.value = update;
  }

  await performCheck();
  setTimeout(
    () => {
      checkUpdates();
    },
    5 /* min */ * 60 /* sec */ * 1000 /* ms */,
  );
}

async function showUpdateToast() {
  updateToastDismissed.value = false;
}

async function downloadAvailableUpdate() {
  return downloadUpdate(availableUpdate.value);
}

async function downloadUpdate(versionToDownload) {
  if (!versionToDownload) {
    handleError(`Failed to download update: no version available`);
  }

  if (appUpdateDownload.progress.value !== 0) {
    console.error(`Update ${versionToDownload.version} already downloading`);
    return;
  }

  console.log(`Downloading update ${versionToDownload.version}`);

  try {
    enqueueUpdateForInstallation(versionToDownload.rid).then(() => {
      finishedDownloading.value = true;
      unlistenUpdateDownload?.().then(() => {
        unlistenUpdateDownload = null;
      });
      console.log("Finished downloading!");
    });
    unlistenUpdateDownload = await subscribeToDownloadProgress(
      appUpdateDownload,
      versionToDownload.version,
    );
  } catch (e) {
    handleError(e);
  }
}

async function installUpdate() {
  restarting.value = true;
  setTimeout(async () => {
    await handleClose();
  }, 250);
}

function handleClick(e) {
  let target = e.target;
  while (target != null) {
    if (target.matches("a")) {
      let isAllowedProtocol = false;
      let isInternalHost = false;

      if (target.href) {
        try {
          const parsedUrl = new URL(target.href);
          isAllowedProtocol = ["http:", "https:", "mailto:", "tel:"].includes(
            parsedUrl.protocol,
          );
          isInternalHost =
            (parsedUrl.protocol === "http:" || parsedUrl.protocol === "https:") &&
            ["localhost", "tauri.localhost"].includes(parsedUrl.hostname);
        } catch {
          isAllowedProtocol = false;
          isInternalHost = false;
        }
      }
      if (
        target.href &&
        isAllowedProtocol &&
        !target.classList.contains("router-link-active") &&
        !isInternalHost
      ) {
        e.preventDefault();
      }
      e.preventDefault();
      break;
    }
    target = target.parentElement;
  }
}

function handleAuxClick(e) {
  // disables middle click -> new tab
  if (e.button === 1) {
    e.preventDefault();
    const event = new MouseEvent("click", {
      view: window,
      bubbles: true,
      cancelable: true,
    });
    e.target.dispatchEvent(event);
  }
}

provideAppUpdateDownloadProgress(appUpdateDownload);

onMounted(() => {
  error.setErrorModal(errorModal.value);
});

watch(
  () => ui.settingsRequest,
  (request) => {
    if (request) settingsModal.value?.show(request.tab);
  },
);

// The welcome guide shows once, on first launch, and again when asked from Settings.
watch(stateInitialized, (ready) => {
  if (ready) setTimeout(() => welcomeModal.value?.showIfFirstLaunch(), 600);
});
watch(
  () => ui.welcomeRequest,
  () => welcomeModal.value?.show(),
);

const shortcutsModal = ref(null);
watch(
  () => ui.shortcutsRequest,
  () => shortcutsModal.value?.show(),
);
useShortcut("mod+,", () => settingsModal.value?.show());
useShortcut("?", () => shortcutsModal.value?.show());

// A game that was just set up (found, or located by the player) gets a Default modpack, if it
// has none. The first-launch guide does the same for Hollow Knight and takes the player there.
const { ensureDefaultModpack } = useGameSetup();
watch(
  () => games.configured,
  async (event) => {
    if (!event) return;
    try {
      const created = await ensureDefaultModpack(event.game);
      if (created) {
        addNotification({
          title: `Made a ${created.name} modpack for ${gameName(event.game)}`,
          text: "Find it under Modpacks to start adding mods.",
          type: "success",
        });
      }
    } catch (err) {
      handleError(err);
    }
  },
);

// "New modpack" from anywhere (the rail, the welcome guide, empty states, Ctrl+N).
const editorModal = ref(null);
watch(
  () => ui.createRequest,
  (request) => {
    if (request) editorModal.value?.show(undefined, request.game ?? games.activeGame);
  },
);
async function onModpackCreated(modpack) {
  // Straight to Browse, so the player can start adding mods.
  if (modpack) await router.push(modpackRoute(modpack, "browse"));
}
useShortcut("mod+n", () => ui.createModpack());

const updateReady = computed(
  () =>
    !!availableUpdate.value &&
    updateToastDismissed.value &&
    !restarting.value &&
    (finishedDownloading.value || metered.value),
);

// Back/forward buttons reflect whether there is somewhere to go.
const canGoBack = ref(false);
const canGoForward = ref(false);
watch(
  () => route.fullPath,
  () => {
    const state = window.history.state ?? {};
    canGoBack.value = !!state.back;
    canGoForward.value = !!state.forward;
  },
  { immediate: true, flush: "post" },
);
</script>

<template>
  <div id="teleports"></div>
  <div
    v-if="stateInitialized"
    class="app-grid-layout experimental-styles-within relative"
    :class="{ 'disable-advanced-rendering': !themeStore.advancedRendering }"
  >
    <Suspense>
      <Transition name="toast">
        <UpdateToast
          v-if="
            !!availableUpdate &&
            !updateToastDismissed &&
            !restarting &&
            (finishedDownloading || metered)
          "
          :version="availableUpdate.version"
          :size="updateSize"
          :metered="metered"
          @close="updateToastDismissed = true"
          @restart="installUpdate"
          @download="downloadAvailableUpdate"
        />
      </Transition>
    </Suspense>
    <Transition name="fade">
      <div
        v-if="restarting"
        data-tauri-drag-region
        class="inset-0 fixed bg-black/80 backdrop-blur z-[200] flex items-center justify-center"
      >
        <span
          data-tauri-drag-region
          class="flex items-center gap-4 text-contrast font-semibold text-xl select-none cursor-default"
        >
          <RefreshCwIcon data-tauri-drag-region class="animate-spin w-6 h-6" />
          Restarting...
        </span>
      </div>
    </Transition>
    <AppSettingsModal ref="settingsModal" />
    <div
      data-tauri-drag-region
      class="app-grid-statusbar bg-bg-raised h-[--top-bar-height] flex"
    >
      <div data-tauri-drag-region class="flex min-w-0 items-center px-3">
        <span
          data-tauri-drag-region
          class="text-brand font-extrabold text-lg select-none cursor-default tracking-tight"
        >Needlelight</span>
        <div data-tauri-drag-region class="flex items-center gap-1 ml-3">
          <button
            class="history-button"
            aria-label="Back"
            :disabled="!canGoBack"
            @click="router.back()"
          >
            <LeftArrowIcon />
          </button>
          <button
            class="history-button"
            aria-label="Forward"
            :disabled="!canGoForward"
            @click="router.forward()"
          >
            <RightArrowIcon />
          </button>
        </div>
        <Breadcrumbs class="min-w-0" />
      </div>
      <section data-tauri-drag-region class="flex ml-auto items-center">
        <div class="flex mr-2">
          <Suspense>
            <RunningAppBar />
          </Suspense>
        </div>
        <div class="flex items-center gap-1 pr-2" data-tauri-drag-region-exclude>
          <Transition name="nav-button-animated">
            <button
              v-if="updateReady"
              v-tooltip.bottom="
                formatMessage(
                  finishedDownloading
                    ? messages.reloadToUpdate
                    : downloadProgress === 0
                      ? messages.downloadUpdate
                      : messages.downloadingUpdate,
                  { percent: downloadPercent },
                )
              "
              class="titlebar-action text-brand"
              aria-label="Update Needlelight"
              @click="
                finishedDownloading
                  ? installUpdate()
                  : downloadProgress > 0 && downloadProgress < 1
                    ? showUpdateToast()
                    : downloadAvailableUpdate()
              "
            >
              <ProgressSpinner
                v-if="downloadProgress > 0 && downloadProgress < 1"
                :progress="downloadProgress"
              />
              <RefreshCwIcon v-else-if="finishedDownloading" />
              <DownloadIcon v-else />
            </button>
          </Transition>
        </div>
        <span class="titlebar-divider" aria-hidden="true" />
        <section class="window-controls" data-tauri-drag-region-exclude>
          <button class="titlebar-button" aria-label="Minimize" @click="() => getCurrentWindow().minimize()"><MinimizeIcon /></button>
          <button class="titlebar-button" aria-label="Maximize" @click="() => getCurrentWindow().toggleMaximize()"><RestoreIcon v-if="isMaximized" /><MaximizeIcon v-else /></button>
          <button class="titlebar-button close" aria-label="Close" @click="handleClose"><XIcon /></button>
        </section>
      </section>
    </div>
  </div>
  <div
    v-if="stateInitialized"
    class="app-contents experimental-styles-within"
    :class="{ 'disable-advanced-rendering': !themeStore.advancedRendering }"
  >
    <NavRail />
    <div class="app-viewport flex-grow router-view">
      <RouterView v-slot="{ Component }">
        <template v-if="Component">
          <Suspense
            @pending="loading.startLoading()"
            @resolve="loading.stopLoading()"
          >
            <component :is="Component"></component>
          </Suspense>
        </template>
      </RouterView>
    </div>
  </div>
  <WelcomeModal ref="welcomeModal" />
  <KeyboardShortcutsModal ref="shortcutsModal" />
  <ModpackEditorModal ref="editorModal" @saved="onModpackCreated" />
  <ErrorModal ref="errorModal" />
  <NotificationPanel />
</template>

<style lang="scss" scoped>
.history-button {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 1.625rem;
  height: 1.625rem;
  padding: 0;
  margin: 0;
  border: none;
  border-radius: 0.5rem;
  background: transparent;
  color: var(--color-base);
  cursor: pointer;
  transition: background-color 0.12s ease, color 0.12s ease;

  svg {
    width: 1rem;
    height: 1rem;
  }

  &:hover:not(:disabled) {
    background: var(--color-button-bg);
    color: var(--color-contrast);
  }

  &:disabled {
    opacity: 0.35;
    cursor: default;
  }

  &:focus-visible {
    outline: 2px solid var(--color-brand);
    outline-offset: 1px;
  }
}

.titlebar-action {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 2rem;
  height: 2rem;
  padding: 0;
  border: none;
  border-radius: 0.5rem;
  background: transparent;
  color: var(--color-base);
  cursor: pointer;
  transition: background-color 0.12s ease, color 0.12s ease;

  svg {
    width: 1.125rem;
    height: 1.125rem;
  }

  &:hover {
    background: var(--color-button-bg);
    color: var(--color-contrast);
  }

  &:focus-visible {
    outline: 2px solid var(--color-brand);
    outline-offset: 1px;
  }
}

.titlebar-divider {
  width: 1px;
  height: 1.25rem;
  margin-right: 0.25rem;
  background: var(--color-divider);
}

.window-controls {
  z-index: 20;
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: 0.15rem;
  padding-right: 0.35rem;

  .titlebar-button {
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: background-color 0.12s ease, color 0.12s ease, transform 0.08s ease;
    background-color: transparent;
    color: var(--color-base);
    height: 1.55rem;
    width: 1.7rem;
    min-width: 1.7rem;
    padding: 0 !important;
    margin: 0;
    position: relative;
    box-shadow: none !important;
    border: none !important;
    outline: none !important;
    border-radius: 9999px;

    &:last-child {
      width: 1.7rem;
      min-width: 1.7rem;
      padding: 0 !important;
    }

    svg {
      width: 0.72rem;
      height: 0.72rem;
    }

    &.close {
      &:hover,
      &:active {
        color: var(--color-accent-contrast);
        background-color: var(--color-red);
      }
    }

    &:hover,
    &:active {
      color: var(--color-contrast);
      background-color: var(--color-button-bg);
    }

    &:active {
      transform: scale(0.94);
    }
  }
}

.app-grid-layout,
.app-contents {
  --top-bar-height: 3rem;
}

.app-grid-layout {
  display: grid;
  grid-template: "status" "dummy";
  grid-template-rows: auto 1fr;
  position: relative;
  background-color: var(--color-raised-bg);
  height: 100vh;
}

.app-grid-statusbar {
  grid-area: status;
}

[data-tauri-drag-region-exclude] {
  -webkit-app-region: no-drag;
}

.app-contents {
  position: absolute;
  z-index: 1;
  left: 0;
  top: var(--top-bar-height);
  right: 0;
  bottom: 0;
  height: calc(100vh - var(--top-bar-height));
  background-color: var(--color-bg);
  border-top: 1px solid var(--color-divider);

  display: grid;
  grid-template-columns: var(--nl-rail-width) minmax(0, 1fr);
}

.app-viewport {
  flex-grow: 1;
  height: 100%;
  overflow: auto;
  overflow-x: hidden;
}

.toast-enter-active {
  transition: opacity 0.25s linear;
}

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
}

@media (prefers-reduced-motion: no-preference) {
  .toast-enter-active,
  .nav-button-animated-enter-active {
    transition: all 0.5s cubic-bezier(0.15, 1.4, 0.64, 0.96);
  }

  .toast-leave-active,
  .nav-button-animated-leave-active {
    transition: all 0.25s ease;
  }

  .toast-enter-from {
    scale: 0.5;
    translate: 0 -10rem;
    opacity: 0;
  }

  .toast-leave-to {
    scale: 0.96;
    translate: 20rem 0;
    opacity: 0;
  }

  .nav-button-animated-enter-active {
    position: relative;
  }

  .nav-button-animated-enter-active::before {
    content: "";
    inset: 0;
    border-radius: 100vw;
    background-color: var(--color-brand-highlight);
    position: absolute;
    animation: pop 0.5s ease-in forwards;
    opacity: 0;
  }

  @keyframes pop {
    0% {
      scale: 0.5;
    }
    50% {
      opacity: 0.5;
    }
    100% {
      scale: 1.5;
    }
  }

  .nav-button-animated-enter-from {
    scale: 0.5;
    translate: -2rem 0;
    opacity: 0;
  }

  .nav-button-animated-leave-to {
    scale: 0.75;
    opacity: 0;
  }

  .fade-enter-active {
    transition: 0.25s ease-in-out;
  }

  .fade-enter-from {
    opacity: 0;
  }
}

</style>
<style>
:root {
  .fake-appbar {
    height: 2.5rem !important;
  }

  .info-card {
    right: 8rem;
  }

  .profile-card {
    right: 8rem;
  }
}
</style>
<style src="vue-multiselect/dist/vue-multiselect.css"></style>
