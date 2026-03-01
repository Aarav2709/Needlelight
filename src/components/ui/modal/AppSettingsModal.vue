<script setup lang="ts">
import {
  GameIcon,
  GaugeIcon,
  LanguagesIcon,
  PaintbrushIcon,
  ReportIcon,
  SettingsIcon,
  ShieldIcon,
} from "@modrinth/assets";
import {
  commonMessages,
  defineMessage,
  defineMessages,
  ProgressBar,
  TabbedModal,
  useVIntl,
} from "@modrinth/ui";
import { getVersion } from "@tauri-apps/api/app";
import {
  platform as getOsPlatform,
  version as getOsVersion,
} from "@tauri-apps/plugin-os";
import { computed, onMounted, ref, watch } from "vue";

import ModalWrapper from "@/components/ui/modal/ModalWrapper.vue";
import AppearanceSettings from "@/components/ui/settings/AppearanceSettings.vue";
import FeatureFlagSettings from "@/components/ui/settings/FeatureFlagSettings.vue";
import GameSettings from "@/components/ui/settings/GameSettings.vue";
import LanguageSettings from "@/components/ui/settings/LanguageSettings.vue";
import ResourceManagementSettings from "@/components/ui/settings/ResourceManagementSettings.vue";
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
      id: "app.settings.tabs.language",
      defaultMessage: "Language",
    }),
    icon: LanguagesIcon,
    content: LanguageSettings,
    badge: commonMessages.beta,
  },
  {
    name: defineMessage({
      id: "app.settings.tabs.resource-management",
      defaultMessage: "Resource management",
    }),
    icon: GaugeIcon,
    content: ResourceManagementSettings,
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

const version = ref("0.0.0");
const osPlatform = ref("unknown");
const osVersion = ref("");
const settings = ref<Record<string, any> | null>(null);
const ready = ref(false);

onMounted(async () => {
  try {
    version.value = await getVersion().catch(() => "0.0.0");
  } catch { /* ignore */ }
  try {
    osPlatform.value = getOsPlatform();
    osVersion.value = getOsVersion();
  } catch { /* ignore */ }
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
  <ModalWrapper ref="modal">
    <template #title>
      <span
        class="flex items-center gap-2 text-lg font-extrabold text-contrast"
      >
        <SettingsIcon /> Settings
      </span>
    </template>

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
              <p class="m-0">Needlelight {{ version }}</p>
              <p class="m-0">
                <span v-if="osPlatform === 'macos'">macOS</span>
                <span v-else class="capitalize">{{ osPlatform }}</span>
                {{ osVersion }}
              </p>
            </div>
          </div>
        </div>
      </template>
    </TabbedModal>
    <div v-else class="p-8 text-center text-secondary">Loading settings...</div>
  </ModalWrapper>
</template>
