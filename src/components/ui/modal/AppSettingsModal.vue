<script setup lang="ts">
import {
  GameIcon,
  PaintbrushIcon,
  ReportIcon,
  SettingsIcon,
  ShieldIcon,
  XIcon,
} from "@modrinth/assets";
import {
  defineMessage,
  defineMessages,
  ProgressBar,
  TabbedModal,
  useVIntl,
} from "@modrinth/ui";
import { onMounted, computed, ref, watch } from "vue";

import ModalWrapper from "@/components/ui/modal/ModalWrapper.vue";
import AppearanceSettings from "@/components/ui/settings/AppearanceSettings.vue";
import FeatureFlagSettings from "@/components/ui/settings/FeatureFlagSettings.vue";
import GameSettings from "@/components/ui/settings/GameSettings.vue";
import { get, set } from "@/helpers/settings.ts";
import { injectAppUpdateDownloadProgress } from "@/providers/download-progress.ts";
import { useTheming } from "@/store/state";

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

const modal = ref();

function show() {
  modal.value?.show();
}

const isOpen = computed(() => modal.value?.isOpen);

defineExpose({ show, isOpen });

const { progress, version: downloadingVersion } =
  injectAppUpdateDownloadProgress();

const settings = ref<Record<string, any> | null>(null);
const ready = ref(false);

onMounted(async () => {
  try {
    settings.value = await get();
  } catch { /* ignore */ }
  ready.value = true;
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

    if (!themeStore.devMode && tabs[modal.value?.selectedTab]?.developerOnly) {
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
  <ModalWrapper ref="modal" hide-header>
    <div class="relative p-6 pb-4">
      <span
        class="flex items-center gap-2 text-lg font-extrabold text-contrast"
      >
        <SettingsIcon /> Settings
      </span>
      <button
        class="absolute top-4 right-4 w-8 h-8 flex items-center justify-center rounded-full bg-button-bg border-none cursor-pointer text-secondary hover:text-contrast hover:brightness-125 transition-all active:scale-90"
        aria-label="Close"
        @click="modal?.hide()"
      >
        <XIcon class="w-4 h-4" />
      </button>
    </div>

    <TabbedModal
      v-if="ready"
      :tabs="tabs.filter((t) => !t.developerOnly || themeStore.devMode)"
    >
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
            <div>
              <p class="m-0 font-semibold">Needlelight v8.0.0.0</p>
            </div>
          </div>
        </div>
      </template>
    </TabbedModal>
    <div v-else class="p-8 text-center text-secondary">Loading settings...</div>
  </ModalWrapper>
</template>
