<script setup lang="ts">
import {
  GameIcon,
  PaintbrushIcon,
  ReportIcon,
  SettingsIcon,
  ShieldIcon,
} from "@modrinth/assets";
import {
  defineMessage,
  defineMessages,
  ProgressBar,
  TabbedModal,
  useVIntl,
} from "@modrinth/ui";
import { onMounted, ref, watch } from "vue";

import AppearanceSettings from "@/components/ui/settings/AppearanceSettings.vue";
import FeatureFlagSettings from "@/components/ui/settings/FeatureFlagSettings.vue";
import GameSettings from "@/components/ui/settings/GameSettings.vue";
import { type AppSettings, get, set } from "@/helpers/settings.ts";
import { injectAppUpdateDownloadProgress } from "@/providers/download-progress.ts";
import { useTheming } from "@/store/state";

// NOTE (Needlelight): current Modrinth's TabbedModal wraps NewModal internally and is meant to
// be used directly as the top-level modal (see apps/app-frontend's own AppSettingsModal.vue for
// reference) - it is not meant to be nested inside a separate Modal/ModalWrapper anymore. Doing
// that produced two independent Teleport-to-body overlays stacked on top of each other, which is
// what was causing the modal to render as a tiny, broken box.
const themeStore = useTheming();

const { formatMessage } = useVIntl();

const devModeCounter = ref(0);

const developerModeEnabled = defineMessage({
  id: "app.settings.developer-mode-enabled",
  defaultMessage: "Developer mode enabled.",
});

const tabs = [
  {
    name: defineMessage({
      id: "app.settings.tabs.appearance",
      defaultMessage: "Appearance",
    }),
    icon: PaintbrushIcon,
    content: AppearanceSettings,
  },
  {
    name: defineMessage({
      id: "app.settings.tabs.game",
      defaultMessage: "Game",
    }),
    icon: GameIcon,
    content: GameSettings,
  },
  {
    name: defineMessage({
      id: "app.settings.tabs.feature-flags",
      defaultMessage: "Feature flags",
    }),
    icon: ReportIcon,
    content: FeatureFlagSettings,
    developerOnly: true,
  },
];

const modal = ref<InstanceType<typeof TabbedModal> | null>(null);

function show() {
  modal.value?.show();
}

defineExpose({ show });

const { progress, version: downloadingVersion } =
  injectAppUpdateDownloadProgress();

const settings = ref<AppSettings | null>(null);

onMounted(async () => {
  try {
    settings.value = await get();
  } catch { /* ignore */ }
});

watch(
  settings,
  async (val) => {
    if (val) await set(val);
  },
  { deep: true },
);

function devModeCount() {
  devModeCounter.value++;
  if (devModeCounter.value > 5) {
    themeStore.devMode = !themeStore.devMode;
    if (settings.value) settings.value.developer_mode = !!themeStore.devMode;
    devModeCounter.value = 0;

    if (!themeStore.devMode && tabs[modal.value?.selectedTab ?? 0]?.developerOnly) {
      modal.value?.setTab(0);
    }
  }
}

const messages = defineMessages({
  downloading: {
    id: "app.settings.downloading",
    defaultMessage: "Downloading v{version}",
  },
});
</script>
<template>
  <TabbedModal
    ref="modal"
    :tabs="tabs.filter((t) => !t.developerOnly || themeStore.devMode)"
    width="min(720px, calc(100vw - 6rem))"
  >
    <template #title>
      <span class="flex items-center gap-2 text-2xl font-semibold text-contrast">
        <SettingsIcon /> Settings
      </span>
    </template>
    <template #footer>
      <div class="mt-auto text-secondary text-sm">
        <div class="mb-3">
          <template v-if="progress > 0 && progress < 1">
            <p class="m-0 mb-2">
              {{
                formatMessage(messages.downloading, {
                  version: downloadingVersion,
                })
              }}
            </p>
            <ProgressBar :progress="progress" />
          </template>
        </div>
        <p
          v-if="themeStore.devMode"
          class="text-brand font-semibold m-0 mb-2"
        >
          {{ formatMessage(developerModeEnabled) }}
        </p>
        <div class="flex items-center gap-3">
          <button
            class="p-0 m-0 bg-transparent border-none cursor-pointer button-animation"
            :class="{
              'text-brand': themeStore.devMode,
              'text-secondary': !themeStore.devMode,
            }"
            @click="devModeCount"
          >
            <ShieldIcon class="w-6 h-6" />
          </button>
          <div class="flex items-center gap-2">
            <span class="font-semibold underline decoration-brand/60">Needlelight</span>
            <span class="px-2 py-0.5 rounded bg-button-bg text-contrast text-xs font-bold">
              v8.0.0.0
            </span>
          </div>
        </div>
      </div>
    </template>
  </TabbedModal>
</template>
