<script setup>
import {
  DownloadIcon,
  HomeIcon,
  LeftArrowIcon,
  LibraryIcon,
  MaximizeIcon,
  MinimizeIcon,
  RefreshCwIcon,
  RestoreIcon,
  RightArrowIcon,
  SettingsIcon,
  ShieldIcon,
  XIcon,
} from "@modrinth/assets";
import {
  Button,
  ButtonStyled,
  commonMessages,
  defineMessages,
  OverflowMenu,
  ProgressSpinner,
  provideNotificationManager,
  useVIntl,
} from "@modrinth/ui";
import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { saveWindowState, StateFlags } from "@tauri-apps/plugin-window-state";
import { computed, onMounted, onUnmounted, ref } from "vue";
import { RouterView, useRoute, useRouter } from "vue-router";

import NeedlelightAppLogo from "@/assets/modrinth_app.svg?component";
import Breadcrumbs from "@/components/ui/Breadcrumbs.vue";
import ErrorModal from "@/components/ui/ErrorModal.vue";
import InstanceCreationModal from "@/components/ui/InstanceCreationModal.vue";
import AppSettingsModal from "@/components/ui/modal/AppSettingsModal.vue";
import NavButton from "@/components/ui/NavButton.vue";
import RunningAppBar from "@/components/ui/RunningAppBar.vue";
import UpdateAvailableToast from "@/components/ui/UpdateAvailableToast.vue";
import UpdateToast from "@/components/ui/UpdateToast.vue";
import { useCheckDisableMouseover } from "@/composables/macCssFix.js";
import { command_listener, warning_listener } from "@/helpers/events.js";
import { get as getSettings } from "@/helpers/settings.ts";
import { initialize_state } from "@/helpers/state";
import {
  areUpdatesEnabled,
  enqueueUpdateForInstallation,
  getOS,
  getUpdateSize,
  isDev,
  isNetworkMetered,
} from "@/helpers/utils.js";
import i18n from "@/i18n.config";
import {
  provideAppUpdateDownloadProgress,
  subscribeToDownloadProgress,
} from "@/providers/download-progress.ts";
import { useError } from "@/store/error.js";
import { useLoading, useTheming } from "@/store/state";

import { AppNotificationManager } from "./providers/app-notifications";

const themeStore = useTheming();

const notificationManager = new AppNotificationManager();
provideNotificationManager(notificationManager);
const { handleError, addNotification } = notificationManager;

const showOnboarding = ref(false);
const nativeDecorations = ref(false);

const os = ref("");
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
  const {
    native_decorations,
    theme,
    locale,
    collapsed_navigation,
    advanced_rendering,
    onboarded,
    default_page,
    toggle_sidebar,
    developer_mode,
    feature_flags,
  } = await getSettings();

  // Initialize locale from saved settings
  if (locale) {
    i18n.global.locale.value = locale;
  }

  if (default_page === "Library") {
    await router.push("/library");
  }

  os.value = await getOS();
  const dev = await isDev();
  isDevEnvironment.value = dev;
  showOnboarding.value = !onboarded;

  nativeDecorations.value = native_decorations;
  if (os.value !== "MacOS") {
    try {
      await getCurrentWindow().setDecorations(native_decorations);
    } catch (error) {
      console.warn("Unable to set window decorations:", error);
    }
  }

  themeStore.setThemeState(theme);
  themeStore.collapsedNavigation = collapsed_navigation;
  themeStore.advancedRendering = advanced_rendering;
  themeStore.toggleSidebar = toggle_sidebar;
  themeStore.devMode = developer_mode;
  themeStore.featureFlags = feature_flags;
  stateInitialized.value = true;

  isMaximized.value = await getCurrentWindow().isMaximized();

  await getCurrentWindow().onResized(async () => {
    isMaximized.value = await getCurrentWindow().isMaximized();
  });

  if (!dev)
    document.addEventListener("contextmenu", (event) => event.preventDefault());

  const osType =
    os.value === "MacOS"
      ? "macos"
      : os.value === "Windows"
        ? "windows"
        : "linux";
  if (osType === "macos") {
    document.getElementsByTagName("html")[0].classList.add("mac");
  } else {
    document.getElementsByTagName("html")[0].classList.add("windows");
  }

  await warning_listener((e) =>
    addNotification({
      title: "Warning",
      text: e.message,
      type: "warn",
    }),
  );
}

const stateFailed = ref(false);
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
      if (
        target.href &&
        ["http://", "https://", "mailto:", "tel:"].some((v) =>
          target.href.startsWith(v),
        ) &&
        !target.classList.contains("router-link-active") &&
        !target.href.startsWith("http://localhost") &&
        !target.href.startsWith("https://tauri.localhost") &&
        !target.href.startsWith("http://tauri.localhost")
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
        <UpdateAvailableToast
          v-else-if="!updatesEnabled && os === 'Linux' && !isDevEnvironment"
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
    <Suspense>
      <AppSettingsModal ref="settingsModal" />
    </Suspense>
    <Suspense>
      <InstanceCreationModal ref="installationModal" />
    </Suspense>
    <div
      class="app-grid-navbar bg-bg-raised flex flex-col p-[0.5rem] pt-0 gap-[0.5rem] w-[--left-bar-width]"
    >
      <NavButton v-tooltip.right="'Home'" to="/">
        <HomeIcon />
      </NavButton>
      <NavButton
        v-tooltip.right="'Library'"
        to="/library"
        :is-subpage="() => route.path.startsWith('/instance')"
      >
        <LibraryIcon />
      </NavButton>
      <NavButton v-tooltip.right="'Modding API'" to="/modding-api">
        <ShieldIcon />
      </NavButton>
      <div class="flex flex-grow"></div>
      <Transition name="nav-button-animated">
        <div
          v-if="
            availableUpdate &&
            updateToastDismissed &&
            !restarting &&
            (finishedDownloading || metered)
          "
        >
          <NavButton
            v-tooltip.right="
              formatMessage(
                finishedDownloading
                  ? messages.reloadToUpdate
                  : downloadProgress === 0
                    ? messages.downloadUpdate
                    : messages.downloadingUpdate,
                {
                  percent: downloadPercent,
                },
              )
            "
            :to="
              finishedDownloading
                ? installUpdate
                : downloadProgress > 0 && downloadProgress < 1
                  ? showUpdateToast
                  : downloadAvailableUpdate
            "
          >
            <ProgressSpinner
              v-if="downloadProgress > 0 && downloadProgress < 1"
              class="text-brand"
              :progress="downloadProgress"
            />
            <RefreshCwIcon v-else-if="finishedDownloading" class="text-brand" />
            <DownloadIcon v-else class="text-brand" />
          </NavButton>
        </div>
      </Transition>
      <NavButton
        v-tooltip.right="formatMessage(commonMessages.settingsLabel)"
        :to="() => $refs.settingsModal.show()"
      >
        <SettingsIcon />
      </NavButton>
    </div>
    <div
      data-tauri-drag-region
      class="app-grid-statusbar bg-bg-raised h-[--top-bar-height] flex"
    >
      <div data-tauri-drag-region class="flex p-3">
        <NeedlelightAppLogo
          class="h-full w-auto text-contrast pointer-events-none"
        />
        <div data-tauri-drag-region class="flex items-center gap-1 ml-3">
          <button
            class="cursor-pointer p-0 m-0 text-contrast border-none outline-none bg-button-bg rounded-full flex items-center justify-center w-6 h-6 hover:brightness-75 transition-all"
            @click="router.back()"
          >
            <LeftArrowIcon />
          </button>
          <button
            class="cursor-pointer p-0 m-0 text-contrast border-none outline-none bg-button-bg rounded-full flex items-center justify-center w-6 h-6 hover:brightness-75 transition-all"
            @click="router.forward()"
          >
            <RightArrowIcon />
          </button>
        </div>
        <Breadcrumbs class="pt-[2px]" />
      </div>
      <section data-tauri-drag-region class="flex ml-auto items-center">
        <div class="flex mr-3">
          <Suspense>
            <RunningAppBar />
          </Suspense>
        </div>
        <section
          v-if="!nativeDecorations"
          class="window-controls"
          data-tauri-drag-region-exclude
        >
          <Button
            class="titlebar-button"
            icon-only
            @click="() => getCurrentWindow().minimize()"
          >
            <MinimizeIcon />
          </Button>
          <Button
            class="titlebar-button"
            icon-only
            @click="() => getCurrentWindow().toggleMaximize()"
          >
            <RestoreIcon v-if="isMaximized" />
            <MaximizeIcon v-else />
          </Button>
          <Button class="titlebar-button close" icon-only @click="handleClose">
            <XIcon />
          </Button>
        </section>
      </section>
    </div>
  </div>
  <div
    v-if="stateInitialized"
    class="app-contents experimental-styles-within"
    :class="{ 'disable-advanced-rendering': !themeStore.advancedRendering }"
  >
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
  <ErrorModal ref="errorModal" />
</template>

<style lang="scss" scoped>
.window-controls {
  z-index: 20;
  display: none;
  flex-direction: row;
  align-items: center;

  .titlebar-button {
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all ease-in-out 0.1s;
    background-color: transparent;
    color: var(--color-base);
    height: 100%;
    width: 3rem;
    position: relative;
    box-shadow: none;

    &:last-child {
      padding-right: 0.75rem;
      width: 3.75rem;
    }

    svg {
      width: 1.25rem;
      height: 1.25rem;
    }

    &::before {
      content: "";
      border-radius: 999999px;
      width: 3rem;
      height: 3rem;
      aspect-ratio: 1 / 1;
      margin-block: auto;
      position: absolute;
      background-color: transparent;
      scale: 0.9;
      transition: all ease-in-out 0.2s;
      z-index: -1;
    }

    &.close {
      &:hover,
      &:active {
        color: var(--color-accent-contrast);

        &::before {
          background-color: var(--color-red);
        }
      }
    }

    &:hover,
    &:active {
      color: var(--color-contrast);

      &::before {
        background-color: var(--color-button-bg);
        scale: 1;
      }
    }
  }
}

.app-grid-layout,
.app-contents {
  --top-bar-height: 3rem;
  --left-bar-width: 4rem;
}

.app-grid-layout {
  display: grid;
  grid-template: "status status" "nav dummy";
  grid-template-columns: auto 1fr;
  grid-template-rows: auto 1fr;
  position: relative;
  background-color: var(--color-raised-bg);
  height: 100vh;
}

.app-grid-navbar {
  grid-area: nav;
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
  left: var(--left-bar-width);
  top: var(--top-bar-height);
  right: 0;
  bottom: 0;
  height: calc(100vh - var(--top-bar-height));
  background-color: var(--color-bg);
  border-top-left-radius: var(--radius-xl);

  display: grid;
  grid-template-columns: 1fr;
}

.app-viewport {
  flex-grow: 1;
  height: 100%;
  overflow: auto;
  overflow-x: hidden;
}

.app-contents::before {
  z-index: 1;
  content: "";
  position: fixed;
  left: var(--left-bar-width);
  top: var(--top-bar-height);
  right: calc(-1 * var(--left-bar-width));
  bottom: calc(-1 * var(--left-bar-width));
  border-radius: var(--radius-xl);
  box-shadow: 1px 1px 15px rgba(0, 0, 0, 0.1) inset;
  border-color: var(--surface-5);
  border-width: 1px;
  border-style: solid;
  pointer-events: none;
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
.mac {
  .app-grid-statusbar {
    padding-left: 5rem;
  }
}

.windows {
  .fake-appbar {
    height: 2.5rem !important;
  }

  .window-controls {
    display: flex !important;
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
