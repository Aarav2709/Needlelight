<script setup lang="ts">
/**
 * A mod's README, rendered with Modrinth's sanitizing markdown renderer. Relative links and
 * images are resolved against the project's repository, and links open in the browser.
 */
import { renderString } from '@modrinth/utils'
import { computed } from 'vue'

const props = defineProps<{
	markdown: string
	imageBase?: string | null
	linkBase?: string | null
}>()

const emit = defineEmits<{ 'open-link': [url: string] }>()

const isAbsolute = (url: string) => /^[a-z][a-z0-9+.-]*:/i.test(url) || url.startsWith('//')

function resolve(url: string, base: string) {
	try {
		return new URL(url.replace(/^\.?\//, ''), base).href
	} catch {
		return url
	}
}

const isRelative = (url: string) => !!url && !isAbsolute(url) && !url.startsWith('#')

/**
 * Make relative image and link URLs absolute before rendering: the sanitizer drops relative
 * URLs, so they have to be resolved first (READMEs often use `images/screenshot.png`).
 */
function absolutize(markdown: string) {
	const image = (url: string) => (props.imageBase && isRelative(url) ? resolve(url, props.imageBase) : url)
	const link = (url: string) => (props.linkBase && isRelative(url) ? resolve(url, props.linkBase) : url)
	return (
		markdown
			// ![alt](url "title") and [text](url "title")
			.replace(
				/(!?)\[([^\]]*)\]\(\s*<?([^)\s>]+)>?((?:\s+"[^"]*")?)\s*\)/g,
				(_all, bang: string, text: string, url: string, title: string) =>
					`${bang}[${text}](${bang ? image(url) : link(url)}${title})`,
			)
			// <img src="..."> and <a href="..."> written as HTML
			.replace(
				/(<img\b[^>]*?\bsrc=)(["'])([^"']+)\2/gi,
				(_all, start: string, quote: string, url: string) => `${start}${quote}${image(url)}${quote}`,
			)
			.replace(
				/(<a\b[^>]*?\bhref=)(["'])([^"']+)\2/gi,
				(_all, start: string, quote: string, url: string) => `${start}${quote}${link(url)}${quote}`,
			)
			// [id]: url   (reference-style definitions)
			.replace(/^(\s{0,3}\[[^\]]+\]:\s*)(\S+)/gm, (_all, start: string, url: string) => `${start}${link(url)}`)
	)
}

const html = computed(() => {
	const rendered = renderString(absolutize(props.markdown))
	const doc = new DOMParser().parseFromString(`<div>${rendered}</div>`, 'text/html')
	for (const img of doc.querySelectorAll('img')) {
		img.setAttribute('loading', 'lazy')
		img.setAttribute('alt', img.getAttribute('alt') ?? '')
	}
	return doc.body.firstElementChild?.innerHTML ?? ''
})

function onClick(event: MouseEvent) {
	const link = (event.target as HTMLElement).closest('a')
	if (!link) return
	event.preventDefault()
	const href = link.getAttribute('href') ?? ''
	if (/^https?:\/\//i.test(href)) emit('open-link', href)
}
</script>

<template>
	<!-- The HTML comes from renderString, which sanitizes it. -->
	<!-- eslint-disable-next-line vue/no-v-html -->
	<div class="markdown-body readme nl-selectable" @click="onClick" v-html="html" />
</template>

<style scoped>
.readme {
	font-size: 0.9375rem;
	line-height: 1.65;
	color: var(--color-base);
	overflow-wrap: anywhere;
}
.readme :deep(h1) {
	font-size: 1.375rem;
}
.readme :deep(h2) {
	font-size: 1.1875rem;
}
.readme :deep(h3) {
	font-size: 1.0625rem;
}
.readme :deep(h1),
.readme :deep(h2),
.readme :deep(h3),
.readme :deep(h4) {
	margin: 1.5rem 0 0.75rem;
	font-weight: 800;
	line-height: 1.3;
}
.readme :deep(> :first-child) {
	margin-top: 0;
}
.readme :deep(a) {
	color: var(--color-brand);
	font-weight: 600;
}
.readme :deep(img) {
	border-radius: 0.5rem;
}
.readme :deep(ul),
.readme :deep(ol) {
	padding-left: 1.5rem;
}
</style>
