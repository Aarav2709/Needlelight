<script setup lang="ts">
import {
  Admonition,
  LanguageSelector,
  languageSelectorMessages,
  LOCALES,
  useVIntl,
} from "@modrinth/ui";
import { onMounted, ref, watch } from "vue";

import { get, set } from "@/helpers/settings.ts";
import i18n from "@/i18n.config";

const { formatMessage } = useVIntl();

const platform = formatMessage(languageSelectorMessages.platformApp);

const settings = ref<Record<string, any> | null>(null);
const ready = ref(false);

onMounted(async () => {
  try { settings.value = await get(); } catch { /* ignore */ }
  ready.value = true;
});

watch(
  settings,
  async () => {
    await set(settings.value);
  },
  { deep: true },
);

const $isChanging = ref(false);

async function onLocaleChange(newLocale: string) {
  if (settings.value.locale === newLocale) return;

  $isChanging.value = true;
  try {
    i18n.global.locale.value = newLocale;
    settings.value.locale = newLocale;
  } finally {
    $isChanging.value = false;
  }
}
</script>

<template>
  <div v-if="!ready" class="text-secondary text-sm p-4">Loading language settings...</div>
  <div v-else-if="settings">
  <h2 class="m-0 text-lg font-extrabold text-contrast">Language</h2>

  <Admonition type="warning" class="mt-2 mb-4">
    {{ formatMessage(languageSelectorMessages.languageWarning, { platform }) }}
  </Admonition>

  <p class="m-0 mb-4">
    {{ formatMessage(languageSelectorMessages.languageWarning, { platform }) }}
  </p>

  <LanguageSelector
    :current-locale="settings.locale"
    :locales="LOCALES"
    :on-locale-change="onLocaleChange"
    :is-changing="$isChanging"
  />
  </div>
</template>
