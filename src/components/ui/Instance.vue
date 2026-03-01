<script setup>
import {
  GameIcon,
  TimerIcon,
} from "@modrinth/assets";
import {
  Avatar,
  useRelativeTime,
} from "@modrinth/ui";
import { convertFileSrc } from "@tauri-apps/api/core";
import dayjs from "dayjs";
import { onUnmounted } from "vue";
import { useRouter } from "vue-router";

import { showProfileInFolder } from "@/helpers/utils.js";

const formatRelativeTime = useRelativeTime();

const props = defineProps({
  instance: {
    type: Object,
    default() {
      return {};
    },
  },
  compact: {
    type: Boolean,
    default: false,
  },
  first: {
    type: Boolean,
    default: false,
  },
});

const router = useRouter();

const seeInstance = async () => {
  await router.push(`/instance/${encodeURIComponent(props.instance.path)}`);
};

const openFolder = async () => {
  await showProfileInFolder(props.instance.path);
};

const addContent = async () => {
  await router.push({
    path: `/instance/${encodeURIComponent(props.instance.path)}`,
  });
};

defineExpose({
  seeInstance,
  openFolder,
  addContent,
  instance: props.instance,
});
</script>

<template>
  <template v-if="compact">
    <div
      class="card-shadow grid grid-cols-[auto_1fr] bg-bg-raised rounded-xl p-3 pl-4 gap-2 cursor-pointer hover:brightness-90 transition-all"
      @click="seeInstance"
    >
      <Avatar
        size="48px"
        :src="instance.icon_path ? convertFileSrc(instance.icon_path) : null"
        :tint-by="instance.path"
        alt="Mod card"
      />
      <div
        class="h-full flex items-center font-bold text-contrast leading-normal"
      >
        <span class="line-clamp-2">{{ instance.name }}</span>
      </div>
      <div
        class="flex items-center col-span-2 gap-1 text-secondary font-semibold"
      >
        <TimerIcon />
        <span class="text-sm">
          <template v-if="instance.last_played">
            Played
            {{ formatRelativeTime(dayjs(instance.last_played).toISOString()) }}
          </template>
          <template v-else> Never played </template>
        </span>
      </div>
    </div>
  </template>
  <div v-else>
    <div
      class="button-base bg-bg-raised p-4 rounded-xl flex gap-3 group"
      @click="seeInstance"
    >
      <div class="relative flex items-center justify-center">
        <Avatar
          size="48px"
          :src="instance.icon_path ? convertFileSrc(instance.icon_path) : null"
          :tint-by="instance.path"
          alt="Mod card"
          class="transition-all group-hover:brightness-75"
        />
      </div>
      <div class="flex flex-col gap-1">
        <p
          class="m-0 text-md font-bold text-contrast leading-tight line-clamp-1"
        >
          {{ instance.name }}
        </p>
        <div
          class="flex items-center col-span-3 gap-1 text-secondary font-semibold mt-auto"
        >
          <GameIcon class="shrink-0" />
          <span class="text-sm capitalize"> Hollow Knight </span>
        </div>
      </div>
    </div>
  </div>
</template>
