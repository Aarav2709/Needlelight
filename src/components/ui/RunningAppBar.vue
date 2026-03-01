<template>
  <div class="flex items-center gap-2">
    <span v-if="offline" class="text-secondary text-xs flex items-center gap-1.5">
      <span class="w-2 h-2 rounded-full bg-orange-500 inline-block shrink-0"></span>
      Offline
    </span>
  </div>
</template>

<script setup>
import { ref } from "vue";

const offline = ref(!navigator.onLine);
window.addEventListener("offline", () => {
  offline.value = true;
});
window.addEventListener("online", () => {
  offline.value = false;
});
</script>
