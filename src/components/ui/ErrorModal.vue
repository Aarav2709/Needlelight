<script setup>
import { CopyIcon, CheckIcon, DropdownIcon, XIcon } from "@modrinth/assets";
import { ButtonStyled, Collapsible } from "@modrinth/ui";
import { computed, ref } from "vue";

import ModalWrapper from "@/components/ui/modal/ModalWrapper.vue";
import { handleSevereError } from "@/store/error.js";

const errorModal = ref();
const error = ref();
const closable = ref(true);
const errorCollapsed = ref(false);

const title = ref("An error occurred");
const errorType = ref("unknown");

defineExpose({
  async show(errorVal, context, canClose = true, source = null) {
    closable.value = canClose;

    if (source === "state_init") {
      title.value = "Error initializing Needlelight";
      errorType.value = "state_init";
    } else {
      title.value = "An error occurred";
      errorType.value = "unknown";
    }

    error.value = errorVal;
    errorModal.value.show();
  },
});

const debugInfo = computed(
  () => error.value?.message ?? error.value ?? "No error message.",
);

const copied = ref(false);

async function copyToClipboard(text) {
  await navigator.clipboard.writeText(text);
  copied.value = true;
  setTimeout(() => {
    copied.value = false;
  }, 3000);
}
</script>

<template>
  <ModalWrapper ref="errorModal" :header="title" :closable="closable">
    <div class="modal-body">
      <div class="markdown-body">
        <template v-if="errorType === 'state_init'">
          <p>
            Needlelight failed to initialize. This could be caused by a
            corrupted settings file or missing game installation. Try restarting
            the app.
          </p>
        </template>
        <template v-else>
          <p>
            An unexpected error occurred. Please try again. If the problem
            persists, check the error details below and report it.
          </p>
        </template>
      </div>

      <Collapsible
        class="mt-4"
        :default-open="false"
        @update:open="(val) => (errorCollapsed = val)"
      >
        <template #title>
          <div class="flex items-center gap-2">
            <DropdownIcon
              class="w-5 h-5 transition-transform"
              :class="{ 'rotate-[-90deg]': !errorCollapsed }"
            />
            <span class="font-semibold text-sm">Error details</span>
          </div>
        </template>
        <div class="relative">
          <pre
            class="bg-bg text-secondary text-xs p-4 rounded-lg overflow-auto max-h-48"
            >{{ debugInfo }}</pre
          >
          <ButtonStyled
            class="absolute top-2 right-2"
            type="transparent"
            circular
          >
            <button @click="copyToClipboard(debugInfo)">
              <CheckIcon v-if="copied" class="w-4 h-4" />
              <CopyIcon v-else class="w-4 h-4" />
            </button>
          </ButtonStyled>
        </div>
      </Collapsible>
    </div>
  </ModalWrapper>
</template>
