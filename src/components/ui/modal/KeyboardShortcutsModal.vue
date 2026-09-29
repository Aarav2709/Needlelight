<script setup lang="ts">
import { NewModal } from '@modrinth/ui'
import { ref } from 'vue'

import { describeKeys, SHORTCUTS } from '@/composables/shortcuts'

const modal = ref<InstanceType<typeof NewModal> | null>(null)

defineExpose({ show: () => modal.value?.show() })
</script>

<template>
	<NewModal ref="modal" header="Keyboard shortcuts" width="min(34rem, calc(100vw - 4rem))">
		<div class="flex flex-col gap-5">
			<section v-for="group in SHORTCUTS" :key="group.group" class="flex flex-col gap-1">
				<h3 class="nl-section-label mb-1">{{ group.group }}</h3>
				<div v-for="item in group.items" :key="item.keys" class="shortcut">
					<span class="text-sm text-contrast">{{ item.label }}</span>
					<span class="flex shrink-0 items-center gap-1">
						<kbd v-for="key in describeKeys(item.keys)" :key="key">{{ key }}</kbd>
					</span>
				</div>
			</section>
		</div>
	</NewModal>
</template>

<style scoped>
.shortcut {
	display: flex;
	align-items: center;
	justify-content: space-between;
	gap: 1rem;
	padding: 0.375rem 0;
	border-bottom: 1px solid var(--color-divider);
}
.shortcut:last-child {
	border-bottom: none;
}
kbd {
	display: inline-flex;
	align-items: center;
	justify-content: center;
	min-width: 1.625rem;
	height: 1.625rem;
	padding: 0 0.4375rem;
	border: 1px solid var(--surface-5);
	border-bottom-width: 2px;
	border-radius: 0.375rem;
	background: var(--surface-3);
	color: var(--color-contrast);
	font-family: inherit;
	font-size: 0.75rem;
	font-weight: 700;
}
</style>
