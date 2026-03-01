<script setup lang="ts">
import { NewModal as Modal } from "@modrinth/ui";
import { useTemplateRef } from "vue";

import { useTheming } from "@/store/theme.ts";

const themeStore = useTheming();

const props = defineProps({
  header: {
    type: String,
    default: null,
  },
  hideHeader: {
    type: Boolean,
    default: false,
  },
  closable: {
    type: Boolean,
    default: true,
  },
  onHide: {
    type: Function,
    default() {
      return () => {};
    },
  },
  showAdOnClose: {
    type: Boolean,
    default: true,
  },
});
const modal = useTemplateRef("modal");

defineExpose({
  show: (e: MouseEvent) => {
    modal.value?.show(e);
  },
  hide: () => {
    onModalHide();
    modal.value?.hide();
  },
});

function onModalHide() {
  props.onHide?.();
}
</script>

<template>
  <Modal
    ref="modal"
    :header="header"
    :noblur="!themeStore.advancedRendering"
    :closable="closable"
    :hide-header="hideHeader"
    @hide="onModalHide"
  >
    <template #title>
      <slot name="title" />
    </template>
    <slot />
  </Modal>
</template>
