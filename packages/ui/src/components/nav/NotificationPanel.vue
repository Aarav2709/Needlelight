<template>
	<div
		class="vue-notification-group"
		:class="{
			'location-left': notificationLocation === 'left',
			'location-right': notificationLocation === 'right',
			'has-sidebar': hasSidebar,
		}"
	>
		<transition-group name="notifs">
			<div
				v-for="(item, index) in notifications"
				:key="item.id"
				class="vue-notification-wrapper"
				@mouseenter="stopTimer(item)"
				@mouseleave="setNotificationTimer(item)"
			>
				<div class="flex w-full gap-3 rounded-lg bg-bg-raised p-3 shadow-xl">
					<div
						class="shrink-0"
						:class="{
							'text-red': item.type === 'error',
							'text-orange': item.type === 'warning',
							'text-green': item.type === 'success',
							'text-blue': !item.type || !['error', 'warning', 'success'].includes(item.type),
						}"
					>
						<IssuesIcon v-if="item.type === 'warning'" class="h-5 w-5" />
						<CheckCircleIcon v-else-if="item.type === 'success'" class="h-5 w-5" />
						<XCircleIcon v-else-if="item.type === 'error'" class="h-5 w-5" />
						<InfoIcon v-else class="h-5 w-5" />
					</div>

					<div class="flex-1 min-w-0 flex flex-col gap-1">
						<div class="text-sm font-bold text-contrast">{{ item.title }}</div>
						<div v-if="item.text" class="text-sm text-secondary">{{ item.text }}</div>
					</div>

					<button
						class="shrink-0 h-6 w-6 rounded-full flex items-center justify-center text-secondary outline-none hover:bg-button-bg hover:text-contrast"
						@click="dismissNotification(index)"
					>
						<XIcon class="h-4 w-4" />
					</button>
				</div>
			</div>
		</transition-group>
	</div>
</template>

<script setup lang="ts">
import { CheckCircleIcon, InfoIcon, IssuesIcon, XCircleIcon, XIcon } from '@modrinth/assets'
import { computed } from 'vue'

import { injectNotificationManager, type WebNotification } from '../../providers'

const notificationManager = injectNotificationManager()
const notifications = computed<WebNotification[]>(() => notificationManager.getNotifications())
const notificationLocation = computed(() => notificationManager.getNotificationLocation())

const stopTimer = (n: WebNotification) => notificationManager.stopNotificationTimer(n)
const setNotificationTimer = (n: WebNotification) => notificationManager.setNotificationTimer(n)
const dismissNotification = (n: number) => notificationManager.removeNotificationByIndex(n)

withDefaults(
	defineProps<{
		hasSidebar?: boolean
	}>(),
	{
		hasSidebar: false,
	},
)
</script>

<style lang="scss" scoped>
.vue-notification-group {
	position: fixed;
	bottom: 1.5rem;
	z-index: 200;
	width: 380px;
	max-width: calc(100% - 1.5rem * 2);

	&.location-right {
		right: 1.5rem;

		&.has-sidebar {
			right: 325px;
		}
	}

	&.location-left {
		left: 1.5rem;
	}

	.vue-notification-wrapper {
		width: 100%;
		overflow: hidden;
		margin-bottom: 0.5rem;

		&:last-child {
			margin: 0;
		}
	}
}

.notifs-enter-active,
.notifs-leave-active {
	transition: opacity 0.25s ease, transform 0.25s ease;
}
.notifs-move {
	transition: transform 0.25s ease;
}

.notifs-enter-from {
	opacity: 0;
	transform: translateY(0.5rem);
}

// taken out of flow while leaving so the remaining notifications can slide
// smoothly into the gap instead of snapping into place
.notifs-leave-active {
	position: absolute;
}

.notifs-leave-to {
	opacity: 0;
	transform: translateY(1rem);
}
</style>
