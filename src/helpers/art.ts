// generated modpack artwork, a gradient tile seeded by the folder path in colors that belong to its game
import type { GameKey } from '@/helpers/games'

type Palette = [base: string, deep: string, glow: string, spark: string]

const PALETTES: Record<GameKey, Palette[]> = {
	hollow_knight: [
		['#2e1065', '#7c3aed', '#22d3ee', '#f0abfc'],
		['#1e1b4b', '#4338ca', '#a78bfa', '#38bdf8'],
		['#3b0764', '#9333ea', '#f472b6', '#fde68a'],
		['#0f172a', '#1d4ed8', '#8b5cf6', '#2dd4bf'],
		['#312e81', '#6d28d9', '#c084fc', '#60a5fa'],
		['#1a1033', '#5b21b6', '#e879f9', '#a5b4fc'],
		['#082f49', '#0e7490', '#a855f7', '#c7d2fe'],
		['#3f0d4a', '#86198f', '#f0abfc', '#818cf8'],
		['#172554', '#3730a3', '#34d399', '#e9d5ff'],
		['#2a1a4a', '#7e22ce', '#93c5fd', '#fbcfe8'],
	],
	silksong: [
		['#450a0a', '#dc2626', '#fb923c', '#fde047'],
		['#4c0519', '#e11d48', '#f97316', '#fbcfe8'],
		['#431407', '#c2410c', '#facc15', '#f43f5e'],
		['#3b0764', '#be123c', '#fb7185', '#fdba74'],
		['#1c1917', '#b91c1c', '#f59e0b', '#fecaca'],
		['#500724', '#db2777', '#fb923c', '#fef3c7'],
		['#422006', '#ea580c', '#ef4444', '#fde68a'],
		['#2a0a12', '#9f1239', '#f472b6', '#fcd34d'],
		['#3f1d0b', '#b45309', '#e11d48', '#fed7aa'],
		['#1f0a24', '#a21caf', '#f43f5e', '#fdba74'],
	],
}

function hash(text: string): number {
	let h = 2166136261
	for (let i = 0; i < text.length; i++) {
		h ^= text.charCodeAt(i)
		h = Math.imul(h, 16777619)
	}
	return h >>> 0
}

// small seeded prng (mulberry32) so a modpack always gets the same artwork
function random(seed: number) {
	let a = seed
	return () => {
		a = (a + 0x6d2b79f5) | 0
		let t = Math.imul(a ^ (a >>> 15), 1 | a)
		t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296
	}
}

export type ModpackArt = {
	base: string
	deep: string
	// gradient direction in degrees
	angle: number
	blobs: { x: number; y: number; r: number; color: string; opacity: number }[]
	// a representative color for tinting around the artwork
	tint: string
}

export function modpackArt(seed: string, game: GameKey): ModpackArt {
	const next = random(hash(seed))
	const palettes = PALETTES[game] ?? PALETTES.hollow_knight
	const [base, deep, glow, spark] = palettes[Math.floor(next() * palettes.length)]
	const at = (min: number, max: number) => min + next() * (max - min)
	return {
		base,
		deep,
		angle: Math.round(at(0, 360)),
		blobs: [
			{ x: at(15, 85), y: at(10, 60), r: at(30, 44), color: glow, opacity: at(0.75, 0.95) },
			{ x: at(10, 90), y: at(45, 95), r: at(22, 36), color: spark, opacity: at(0.55, 0.8) },
			{ x: at(0, 100), y: at(0, 100), r: at(16, 28), color: deep, opacity: 0.9 },
		],
		tint: glow,
	}
}

