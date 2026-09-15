<script setup lang="ts" generic="T extends string">
import { MoonIcon, RadioButtonCheckedIcon, RadioButtonIcon, SunIcon } from '@modrinth/assets'

// NOTE (Needlelight): ThemeSelector was removed from @modrinth/ui when packages/ui was brought
// forward to current Modrinth (it's now folded into a much bigger appearance-settings
// layout/provider system tied to their own settings pages). This is a direct, unchanged port of
// the old standalone component so AppearanceSettings.vue keeps working exactly as before,
// with i18n message IDs swapped for plain strings to match how the rest of Needlelight's own
// settings UI is written (it doesn't otherwise use @modrinth/ui's i18n message system).

const { updateColorTheme, currentTheme, themeOptions, systemThemeColor } = defineProps<{
	updateColorTheme: (theme: T) => void
	currentTheme: T
	themeOptions: readonly T[]
	systemThemeColor: T
}>()

const themeLabels: Record<string, string> = {
	system: 'Sync with system',
	light: 'Light',
	dark: 'Dark',
	oled: 'OLED',
	retro: 'Retro',
}

function asString(theme: T): string {
	return theme
}

function getPreviewClass(option: T): string {
	const base = option === 'system' ? systemThemeColor : option
	return base.endsWith('-mode') ? base : `${base}-mode`
}
</script>

<template>
	<div class="theme-options mt-4">
		<button
			v-for="option in themeOptions"
			:key="option"
			class="preview-radio button-base"
			:class="{ selected: currentTheme === option }"
			@click="() => updateColorTheme(option)"
		>
			<div class="preview" :class="getPreviewClass(option)">
				<div class="example-card card card">
					<div class="example-icon"></div>
					<div class="example-text-1"></div>
					<div class="example-text-2"></div>
				</div>
			</div>
			<div class="label">
				<RadioButtonCheckedIcon v-if="currentTheme === option" class="radio shrink-0" />
				<RadioButtonIcon v-else class="radio shrink-0" />
				{{ themeLabels[asString(option)] ?? option }}
				<SunIcon
					v-if="'light' === option"
					v-tooltip="'Preferred light theme'"
					class="theme-icon shrink-0"
				/>
				<MoonIcon
					v-else-if="'dark' === option"
					v-tooltip="'Preferred dark theme'"
					class="theme-icon shrink-0"
				/>
			</div>
		</button>
	</div>
</template>

<style scoped lang="scss">
.theme-options {
	display: grid;
	grid-template-columns: repeat(auto-fit, minmax(10rem, 1fr));
	gap: var(--gap-lg);

	.preview {
		&.light-mode {
			@extend .light-mode;
		}

		&.dark-mode {
			@extend .dark-mode;
		}

		&.oled-mode {
			@extend .oled-mode;
		}

		&.retro-mode {
			@extend .retro-mode;
		}
	}

	.preview .example-card {
		margin: 0;
		padding: 1rem;
		display: grid;
		grid-template: 'icon text1' 'icon text2';
		grid-template-columns: auto 1fr;
		gap: 0.5rem;
		outline: 2px solid transparent;

		.example-icon {
			grid-area: icon;
			width: 2rem;
			height: 2rem;
			background-color: var(--color-button-bg);
			border-radius: var(--radius-sm);
			outline: 2px solid transparent;
		}

		.example-text-1,
		.example-text-2 {
			height: 0.5rem;
			border-radius: var(--radius-sm);
			outline: 2px solid transparent;
		}

		.example-text-1 {
			grid-area: text1;
			width: 100%;
			background-color: var(--color-base);
		}

		.example-text-2 {
			grid-area: text2;
			width: 60%;
			background-color: var(--color-secondary);
		}
	}
}
</style>
