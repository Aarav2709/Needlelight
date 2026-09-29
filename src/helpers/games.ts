/**
 * The games Needlelight manages. Everything game-specific the UI needs comes from this
 * registry, so supporting another game means adding an entry here plus the matching backend
 * GameKey. The UI always uses the full game name.
 */
export type GameKey = 'hollow_knight' | 'silksong'

export type GameInfo = {
	key: GameKey
	name: string
}

export const GAMES: GameInfo[] = [
	{ key: 'hollow_knight', name: 'Hollow Knight' },
	{ key: 'silksong', name: 'Hollow Knight: Silksong' },
]

export function gameInfo(key: GameKey | string | null | undefined): GameInfo {
	return GAMES.find((g) => g.key === key) ?? GAMES[0]
}

export function gameName(key: GameKey | string | null | undefined): string {
	return gameInfo(key).name
}

export function isGameKey(value: unknown): value is GameKey {
	return GAMES.some((g) => g.key === value)
}

/** "1 modpack", "3 modpacks", "No modpacks yet". */
export function modpackCount(count: number): string {
	if (count === 0) return 'No modpacks yet'
	return `${count} modpack${count === 1 ? '' : 's'}`
}
