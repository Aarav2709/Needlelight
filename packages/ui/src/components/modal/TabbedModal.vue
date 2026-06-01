<script setup lang="ts">
import { type Component, computed, nextTick, ref } from 'vue'

import { type MessageDescriptor, useVIntl } from '../../composables/i18n'
import { useScrollIndicator } from '../../composables/scroll-indicator'

const { formatMessage } = useVIntl()

export type Tab<Props> = {
	name: MessageDescriptor
	icon: Component
	content: Component<Props>
	props?: Props
	badge?: MessageDescriptor
	shown?: boolean
}

const props = defineProps<{
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	tabs: Tab<any>[]
	contentWidth?: string
	contentHeight?: string
}>()

const visibleTabs = computed(() => props.tabs.filter((tab) => tab.shown !== false))
const contentWidth = computed(
	() => props.contentWidth ?? 'min(640px, calc(100vw - 6rem))'
)
const contentHeight = computed(
	() => props.contentHeight ?? 'min(560px, calc(100vh - 16rem))'
)

const selectedTab = ref(0)

const scrollContainer = ref<HTMLElement | null>(null)
const { showTopFade, showBottomFade, checkScrollState, forceCheck } =
	useScrollIndicator(scrollContainer)

function setTab(index: number) {
	selectedTab.value = index
	nextTick(() => forceCheck())
}

defineExpose({ selectedTab, setTab })
</script>
<template>
	<div class="grid grid-cols-[auto_1fr]">
		<div
			class="flex flex-col gap-1 border-solid pr-4 border-0 border-r-[1px] border-divider min-w-[200px]"
		>
			<button
				v-for="(tab, index) in visibleTabs"
				:key="index"
				:class="`flex gap-2 items-center text-left rounded-xl px-4 py-2 border-none text-nowrap font-semibold cursor-pointer active:scale-[0.97] transition-all ${selectedTab === index ? 'bg-button-bgSelected text-contrast' : 'bg-transparent text-contrast opacity-70 hover:opacity-100 hover:bg-button-bg'}`"
				@click="() => setTab(index)"
			>
				<component :is="tab.icon" class="w-4 h-4 flex-shrink-0" />
				<span>{{ formatMessage(tab.name) }}</span>
				<span
					v-if="tab.badge"
					class="rounded-full px-1.5 py-0.5 text-xs font-bold bg-brand-highlight text-brand-green"
				>
					{{ formatMessage(tab.badge) }}
				</span>
			</button>

			<slot name="footer" />
		</div>
		<div class="relative">
			<Transition
				enter-active-class="transition-all duration-200 ease-out"
				enter-from-class="opacity-0 max-h-0"
				enter-to-class="opacity-100 max-h-24"
				leave-active-class="transition-all duration-200 ease-in"
				leave-from-class="opacity-100 max-h-24"
				leave-to-class="opacity-0 max-h-0"
			>
				<div
					v-if="showTopFade"
					class="pointer-events-none absolute left-0 right-0 top-0 z-10 h-24 bg-gradient-to-b from-bg-raised to-transparent"
				/>
			</Transition>

			<div
				ref="scrollContainer"
				class="overflow-y-auto overflow-x-hidden px-4"
				:style="{ width: contentWidth, height: contentHeight }"
				@scroll="checkScrollState"
			>
				<Transition
					mode="out-in"
					enter-active-class="transition-all duration-200 ease-out"
					enter-from-class="opacity-0 translate-y-2"
					enter-to-class="opacity-100 translate-y-0"
					leave-active-class="transition-all duration-150 ease-in"
					leave-from-class="opacity-100 translate-y-0"
					leave-to-class="opacity-0 -translate-y-1"
				>
					<div :key="selectedTab">
						<component
							:is="visibleTabs[selectedTab].content"
							v-bind="visibleTabs[selectedTab].props ?? {}"
						/>
					</div>
				</Transition>
			</div>

			<Transition
				enter-active-class="transition-all duration-200 ease-out"
				enter-from-class="opacity-0 max-h-0"
				enter-to-class="opacity-100 max-h-24"
				leave-active-class="transition-all duration-200 ease-in"
				leave-from-class="opacity-100 max-h-24"
				leave-to-class="opacity-0 max-h-0"
			>
				<div
					v-if="showBottomFade"
					class="pointer-events-none absolute bottom-0 left-0 right-0 z-10 h-24 bg-gradient-to-t from-bg-raised to-transparent"
				/>
			</Transition>
		</div>
	</div>
</template>
