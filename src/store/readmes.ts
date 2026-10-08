// mod readmes, fetched when a details panel opens and kept for the session
import { defineStore } from 'pinia'
import { reactive } from 'vue'

import { getModReadme } from '@/helpers/modpacks'
import type { ModReadme } from '@/helpers/types'

type Entry = { status: 'loading' | 'ready' | 'none' | 'error'; readme: ModReadme | null }

export const useReadmes = defineStore('readmes', () => {
	const entries = reactive(new Map<string, Entry>())
	const key = (url: string, version: string) => `${url}@${version}`

	async function load(url: string, version: string) {
		if (!url) return
		const k = key(url, version)
		const existing = entries.get(k)
		if (existing && existing.status !== 'error') return
		entries.set(k, { status: 'loading', readme: null })
		try {
			const readme = await getModReadme(url, version)
			entries.set(k, { status: readme ? 'ready' : 'none', readme })
		} catch {
			entries.set(k, { status: 'error', readme: null })
		}
	}

	function get(url: string, version: string): Entry | null {
		return entries.get(key(url, version)) ?? null
	}

	return { load, get }
})
