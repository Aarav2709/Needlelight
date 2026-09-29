/**
 * Starting a modpack, shared by the modpack page and the Play button that appears when hovering a
 * modpack in the sidebar, so both behave the same (missing game, updating before play, errors).
 */
import { injectNotificationManager } from '@modrinth/ui'

import { gameInfo } from '@/helpers/games'
import type { Modpack } from '@/helpers/modpacks'
import { useGames } from '@/store/games'
import { useModpacks } from '@/store/modpacks'
import { usePreferences } from '@/store/preferences'
import { useUi } from '@/store/ui'

export function usePlayModpack() {
	const games = useGames()
	const modpacks = useModpacks()
	const { prefs } = usePreferences()
	const ui = useUi()
	const { handleError, addNotification } = injectNotificationManager()

	/** `updates`: installed mods with a newer version, installed first when the player asked for that. */
	async function play(pack: Modpack, updates: string[] = []): Promise<boolean> {
		if (games.found[pack.game] === false) {
			addNotification({
				title: `${gameInfo(pack.game).name} wasn't found`,
				text: 'Locate the game in Settings to play this modpack.',
				type: 'warning',
			})
			ui.openSettings('games')
			return false
		}
		try {
			if (prefs.updateBeforePlay && updates.length) await modpacks.install(pack.path, updates)
			await modpacks.launch(pack.path)
			addNotification({ title: `Launching ${pack.name}`, type: 'success' })
			return true
		} catch (err) {
			handleError(err as Error)
			return false
		}
	}

	return { play }
}
