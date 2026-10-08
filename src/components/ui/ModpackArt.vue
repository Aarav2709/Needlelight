<script setup lang="ts">
// a modpack's generated icon, a gradient tile with soft grain and a layered mark
import { computed, useId } from 'vue'

import { modpackArt } from '@/helpers/art'
import type { GameKey } from '@/helpers/games'

const props = withDefaults(defineProps<{ seed: string; game: GameKey; size?: string }>(), {
	size: '2.75rem',
})

const id = useId()
const art = computed(() => modpackArt(props.seed, props.game))
</script>

<template>
	<svg
		class="modpack-art"
		:style="{ '--_size': size }"
		viewBox="0 0 100 100"
		aria-hidden="true"
		focusable="false"
	>
		<defs>
			<linearGradient :id="`${id}-bg`" :gradientTransform="`rotate(${art.angle} 0.5 0.5)`">
				<stop offset="0" :stop-color="art.base" />
				<stop offset="1" :stop-color="art.deep" />
			</linearGradient>
			<filter :id="`${id}-blur`" x="-50%" y="-50%" width="200%" height="200%">
				<feGaussianBlur stdDeviation="11" />
			</filter>
			<filter :id="`${id}-grain`">
				<feTurbulence type="fractalNoise" baseFrequency="0.9" numOctaves="2" stitchTiles="stitch" />
				<feColorMatrix type="saturate" values="0" />
				<feComponentTransfer>
					<feFuncA type="linear" slope="0.35" />
				</feComponentTransfer>
			</filter>
			<filter :id="`${id}-shadow`" x="-30%" y="-30%" width="160%" height="160%">
				<feDropShadow dx="0" dy="2" stdDeviation="2.5" flood-color="#000" flood-opacity="0.28" />
			</filter>
		</defs>

		<rect width="100" height="100" :fill="`url(#${id}-bg)`" />
		<circle
			v-for="(blob, i) in art.blobs"
			:key="i"
			:cx="blob.x"
			:cy="blob.y"
			:r="blob.r"
			:fill="blob.color"
			:opacity="blob.opacity"
			:filter="`url(#${id}-blur)`"
		/>
		<rect width="100" height="100" :filter="`url(#${id}-grain)`" opacity="0.5" style="mix-blend-mode: overlay" />

		<!-- a stack of layers with the same mark on every modpack, so the color does the identifying -->
		<g :filter="`url(#${id}-shadow)`" fill="none" stroke-linecap="round" stroke-linejoin="round">
			<path d="M50 27 L73 39 L50 51 L27 39 Z" fill="#fff" fill-opacity="0.96" />
			<path d="M27 50 L50 62 L73 50" stroke="#fff" stroke-opacity="0.85" stroke-width="5.5" />
			<path d="M27 61 L50 73 L73 61" stroke="#fff" stroke-opacity="0.6" stroke-width="5.5" />
		</g>
		<rect width="100" height="100" fill="none" stroke="#fff" stroke-opacity="0.12" stroke-width="2" rx="0" />
	</svg>
</template>

<style scoped>
.modpack-art {
	display: block;
	flex-shrink: 0;
	width: var(--_size);
	height: var(--_size);
	border-radius: calc(var(--_size) * 0.24);
	overflow: hidden;
	box-shadow:
		0 1px 2px rgba(0, 0, 0, 0.25),
		0 6px 16px -8px rgba(0, 0, 0, 0.45);
}
</style>
