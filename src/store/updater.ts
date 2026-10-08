// app updates from github releases, checked at startup and every few hours, downloaded in the background
import { relaunch } from '@tauri-apps/plugin-process'
import { check, type Update } from '@tauri-apps/plugin-updater'
import { defineStore } from 'pinia'
import { computed, markRaw, ref, shallowRef } from 'vue'

export type UpdateStatus =
	| 'idle'
	| 'checking'
	| 'none'
	| 'downloading'
	| 'ready'
	| 'installing'
	| 'error'

const CHECK_EVERY_MS = 6 * 60 * 60 * 1000

export const useUpdater = defineStore('updater', () => {
	const status = ref<UpdateStatus>('idle')
	const update = shallowRef<Update | null>(null)
	// download progress from 0 to 1, null while the size is unknown
	const progress = ref<number | null>(null)
	const error = ref<string | null>(null)
	// the toast was closed, the title bar button stays
	const dismissed = ref(false)

	const version = computed(() => update.value?.version ?? null)
	const busy = computed(() => ['checking', 'downloading', 'installing'].includes(status.value))

	let timer: ReturnType<typeof setInterval> | null = null

	async function download(found: Update) {
		status.value = 'downloading'
		progress.value = null
		let total = 0
		let received = 0
		await found.download((event) => {
			if (event.event === 'Started') {
				total = event.data.contentLength ?? 0
				progress.value = total ? 0 : null
			} else if (event.event === 'Progress') {
				received += event.data.chunkLength
				if (total) progress.value = Math.min(1, received / total)
			} else {
				progress.value = 1
			}
		})
		status.value = 'ready'
	}

	// a failed check stays quiet and is simply tried again next time
	async function checkNow() {
		if (busy.value || status.value === 'ready') return status.value
		status.value = 'checking'
		error.value = null
		try {
			const found = await check()
			if (!found) {
				status.value = 'none'
				return status.value
			}
			update.value = markRaw(found)
			dismissed.value = false
			await download(found)
		} catch (err) {
			error.value = String(err)
			status.value = 'idle'
			console.warn('Update check failed', err)
		}
		return status.value
	}

	async function installAndRestart() {
		if (!update.value || status.value !== 'ready') return
		status.value = 'installing'
		try {
			await update.value.install()
			await relaunch()
		} catch (err) {
			error.value = String(err)
			status.value = 'error'
		}
	}

	// dev builds never update themselves
	function start() {
		if (import.meta.env.DEV || timer) return
		void checkNow()
		timer = setInterval(() => void checkNow(), CHECK_EVERY_MS)
	}

	return {
		status,
		update,
		version,
		progress,
		error,
		dismissed,
		busy,
		installAndRestart,
		start,
	}
})
