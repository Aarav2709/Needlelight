// gives a newly set up game a default modpack so there's somewhere to install mods, only when it has none
import type { GameKey } from '@/helpers/games'
import type { Modpack } from '@/helpers/modpacks'
import { useModpacks } from '@/store/modpacks'

export const DEFAULT_MODPACK_NAME = 'Default'

// the first launch guide and the game configured watcher can both ask at once, so share one request
const pending = new Map<GameKey, Promise<Modpack | null>>()

export function useGameSetup() {
	const modpacks = useModpacks()

	// creates the game's default modpack if it has none, returns it or null
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
