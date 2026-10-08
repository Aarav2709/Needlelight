<script setup>
import {
  LeftArrowIcon,
  MaximizeIcon,
  MinimizeIcon,
  RefreshCwIcon,
  RestoreIcon,
  RightArrowIcon,
  XIcon,
} from "@modrinth/assets";
import { NotificationPanel, ProgressSpinner, provideNotificationManager } from "@modrinth/ui";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { saveWindowState, StateFlags } from "@tauri-apps/plugin-window-state";
import { computed, onMounted, onUnmounted, ref, useTemplateRef, watch } from "vue";
import { RouterView, useRoute, useRouter } from "vue-router";

import Breadcrumbs from "@/components/ui/Breadcrumbs.vue";
import ErrorModal from "@/components/ui/ErrorModal.vue";
import AppSettingsModal from "@/components/ui/modal/AppSettingsModal.vue";
import ModpackEditorModal from "@/components/ui/modal/ModpackEditorModal.vue";
import NavRail from "@/components/ui/NavRail.vue";
import RunningAppBar from "@/components/ui/RunningAppBar.vue";
import UpdateToast from "@/components/ui/UpdateToast.vue";
import { useCheckDisableMouseover } from "@/composables/macCssFix.js";
import { useGameSetup } from "@/composables/setup";
import { useShortcut } from "@/composables/shortcuts";
import { isExternalUrl, openExternal } from "@/helpers/app";
import { applyGameTheme } from "@/helpers/game-theme";
import { gameName } from "@/helpers/games";
import { modpackRoute } from "@/helpers/modpacks";
import { useError } from "@/store/error.js";
import { useGames } from "@/store/games";
import { useModpacks } from "@/store/modpacks";
import { isOnboarded, setOnboarded, usePreferences } from "@/store/preferences";
import { useTheming } from "@/store/theme";
import { useUi } from "@/store/ui";
import { useUpdater } from "@/store/updater";

import { AppNotificationManager } from "./providers/app-notifications";

const themeStore = useTheming();
const preferences = usePreferences();
const games = useGames();
const modpacks = useModpacks();
const ui = useUi();
const updater = useUpdater();
const router = useRouter();
const route = useRoute();
const error = useError();
const settingsModal = useTemplateRef("settingsModal");
const errorModal = ref();

// apply saved ui preferences before the shell first renders
preferences.apply();

const notificationManager = new AppNotificationManager();
provideNotificationManager(notificationManager);
const { handleError, addNotification } = notificationManager;

const stateInitialized = ref(false);
const isMaximized = ref(false);

onMounted(async () => {
  error.setErrorModal(errorModal.value);
  await useCheckDisableMouseover();
  document.body.addEventListener("click", handleClick);
  document.body.addEventListener("auxclick", handleAuxClick);
});

onUnmounted(() => {
  document.body.removeEventListener("click", handleClick);
  document.body.removeEventListener("auxclick", handleAuxClick);
});

async function setupApp() {
  stateInitialized.value = true;

  // the active game tints the app with its accent color once settings load
  applyGameTheme(localStorage.getItem("needlelight.game") ?? "hollow_knight");
  const gamesLoaded = games.load().catch((err) => console.warn("Failed to load settings", err));
  const modpacksLoaded = modpacks
    .load()
    .catch((err) => console.warn("Failed to load modpacks", err));
  // the loading screen stays up until there's something to show
  Promise.allSettled([gamesLoaded, modpacksLoaded]).then(hideSplash);

  updater.start();

  const currentWindow = getCurrentWindow();

  // needlelight draws its own title bar, so keep native decorations off even if old window state restores them
  await currentWindow.setDecorations(false);
  isMaximized.value = await currentWindow.isMaximized();

  await currentWindow.onResized(async () => {
    isMaximized.value = await getCurrentWindow().isMaximized();
  });

  // the browser context menu only makes sense while developing
  if (!import.meta.env.DEV)
    document.addEventListener("contextmenu", (event) => event.preventDefault());
}

// fades out the loading screen from index.html
function hideSplash() {
  const splash = document.getElementById("splash");
  if (!splash || splash.classList.contains("is-done")) return;
  splash.classList.add("is-done");
  setTimeout(() => splash.remove(), 400);
}
// never leave the loading screen up if something stalls, errors show in the app instead
setTimeout(hideSplash, 15000);

setupApp().catch((err) => {
  hideSplash();
  console.error("Failed to initialize app", err);
  error.showError(err, null, false, "state_init");
});

const handleClose = async () => {
  await saveWindowState(StateFlags.ALL);
  await getCurrentWindow().close();
};

// web links open in the browser, everything else stays in the app, router links already handled themselves
function handleClick(event) {
  const link = event.target instanceof Element ? event.target.closest("a[href]") : null;
  if (!link || event.defaultPrevented) return;
  event.preventDefault();
  if (isExternalUrl(link.href)) openExternal(link.href).catch((err) => handleError(err));
}

// turns a middle click into a normal click instead of opening a new tab
function handleAuxClick(event) {
  if (event.button !== 1) return;
  event.preventDefault();
  event.target.dispatchEvent(new MouseEvent("click", { view: window, bubbles: true, cancelable: true }));
}

watch(
  () => ui.settingsRequest,
  (request) => {
    if (request) settingsModal.value?.show(request.tab);
  },
);

useShortcut("mod+,", () => settingsModal.value?.show());

// a newly set up game gets a default modpack if it has none
const { ensureDefaultModpack } = useGameSetup();

// on first launch hollow knight is looked for and given a default modpack, the app then opens on it
async function firstLaunchSetup() {
  if (isOnboarded()) return;
  try {
    await Promise.all([games.ensureLoaded(), modpacks.ensureLoaded()]);
    if (games.found.hollow_knight === false) await games.findGame("hollow_knight");
    if (games.found.hollow_knight) await ensureDefaultModpack("hollow_knight");
  } catch (err) {
    handleError(err);
  } finally {
    setOnboarded(true);
  }
}
watch(stateInitialized, (ready) => ready && void firstLaunchSetup());
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

// new modpack from anywhere: the rail, the welcome guide, empty states, or ctrl n
const editorModal = ref(null);
watch(
  () => ui.createRequest,
  (request) => {
    if (request) editorModal.value?.show(undefined, request.game ?? games.activeGame);
  },
);
async function onModpackCreated(modpack) {
  // go straight to browse so the player can start adding mods
  if (modpack) await router.push(modpackRoute(modpack, "browse"));
}
useShortcut("mod+n", () => ui.createModpack());

// the title bar shows update progress, and a restart button once the toast is closed
const showUpdateButton = computed(
  () => updater.status === "downloading" || (updater.status === "ready" && updater.dismissed),
);
const updateTooltip = computed(() => {
  if (updater.status === "ready") return `Restart to update to ${updater.version}`;
  if (updater.progress == null) return `Downloading ${updater.version}`;
  return `Downloading ${updater.version} (${Math.round(updater.progress * 100)}%)`;
});

// back and forward buttons reflect whether there is somewhere to go
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
    <Transition name="toast">
      <UpdateToast v-if="updater.status === 'ready' && !updater.dismissed" />
    </Transition>
    <Transition name="fade">
      <div
        v-if="updater.status === 'installing'"
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
          <RunningAppBar />
        </div>
        <div class="flex items-center gap-1 pr-2" data-tauri-drag-region-exclude>
          <Transition name="nav-button-animated">
            <button
              v-if="showUpdateButton"
              v-tooltip.bottom="updateTooltip"
              class="titlebar-action text-brand"
              :aria-label="updateTooltip"
              @click="updater.status === 'ready' ? updater.installAndRestart() : (updater.dismissed = false)"
            >
              <ProgressSpinner v-if="updater.status === 'downloading'" :progress="updater.progress ?? 0" />
              <RefreshCwIcon v-else />
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
          <Suspense>
            <component :is="Component"></component>
          </Suspense>
        </template>
      </RouterView>
    </div>
  </div>
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
  gap: 0.25rem;
  padding-right: 0.5rem;

  .titlebar-button {
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: background-color 0.12s ease, color 0.12s ease, transform 0.08s ease;
    background-color: transparent;
    color: var(--color-base);
    height: 2.25rem;
    width: 2.75rem;
    min-width: 2.75rem;
    padding: 0 !important;
    margin: 0;
    position: relative;
    box-shadow: none !important;
    border: none !important;
    outline: none !important;
    border-radius: 0.5rem;

    svg {
      width: 1rem;
      height: 1rem;
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
