/**
 * Getting a game ready to play: when a game is set up (found on first launch, or located later),
 * it gets a "Default" modpack so there's somewhere to install mods straight away. Only when the
 * game has no modpacks yet; anything the player already made is left alone.
 */
import type { GameKey } from '@/helpers/games'
import type { Modpack } from '@/helpers/modpacks'
import { useModpacks } from '@/store/modpacks'

export const DEFAULT_MODPACK_NAME = 'Default'

// The first-launch guide and the "game configured" watcher can both ask at once; make one.
const pending = new Map<GameKey, Promise<Modpack | null>>()

export function useGameSetup() {
	const modpacks = useModpacks()

	/** Creates the game's Default modpack if it has none. Returns it, or null if none was needed. */
	function ensureDefaultModpack(game: GameKey): Promise<Modpack | null> {
		const running = pending.get(game)
		if (running) return running
		const task = (async () => {
			await modpacks.ensureLoaded()
			if (modpacks.forGame(game).length) return null
			return modpacks.create({ name: DEFAULT_MODPACK_NAME, game, description: '' })
		})().finally(() => pending.delete(game))
		pending.set(game, task)
		return task
	}

	return { ensureDefaultModpack }
}
