/**
 * Keyboard shortcuts. `useShortcut('mod+f', handler)` listens while the calling component is
 * mounted; `mod` is Ctrl (Cmd on macOS). Shortcuts stay quiet while a dialog is open and, unless
 * asked otherwise, while typing in a text field.
 */
import { onBeforeUnmount, onMounted } from 'vue'

type Options = {
	/** Also fire while focus is in a text field (for Escape, Ctrl+Enter…). */
	inInputs?: boolean
	/** Extra condition checked on every key press. */
	when?: () => boolean
}

const isMac = typeof navigator !== 'undefined' && /mac/i.test(navigator.platform)

function isTyping(target: EventTarget | null) {
	const el = target as HTMLElement | null
	if (!el) return false
	return (
		el.isContentEditable ||
		el.tagName === 'TEXTAREA' ||
		el.tagName === 'SELECT' ||
		(el.tagName === 'INPUT' && !['checkbox', 'radio', 'button'].includes((el as HTMLInputElement).type))
	)
}

export function dialogOpen() {
	return !!document.querySelector('[role="dialog"], [role="menu"], [role="listbox"]')
}

function matches(combo: string, event: KeyboardEvent) {
	const parts = combo.toLowerCase().split('+')
	const key = parts.pop()!
	const mod = parts.includes('mod')
	const hasMod = isMac ? event.metaKey : event.ctrlKey
	if (mod !== hasMod) return false
	if (parts.includes('shift') !== event.shiftKey && key.length > 1) return false
	if (parts.includes('alt') !== event.altKey) return false
	return event.key.toLowerCase() === key
}

export function useShortcut(
	combos: string | string[],
	handler: (event: KeyboardEvent) => void,
	options: Options = {},
) {
	const list = Array.isArray(combos) ? combos : [combos]
	function onKeydown(event: KeyboardEvent) {
		if (event.defaultPrevented || event.repeat) return
		if (!list.some((combo) => matches(combo, event))) return
		if (!options.inInputs && isTyping(event.target)) return
		if (dialogOpen()) return
		if (options.when && !options.when()) return
		event.preventDefault()
		handler(event)
	}
	onMounted(() => window.addEventListener('keydown', onKeydown))
	onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown))
}

/** How a shortcut is written for people, e.g. "Ctrl F". */
export function describeKeys(combo: string): string[] {
	const names: Record<string, string> = {
		mod: isMac ? '⌘' : 'Ctrl',
		shift: 'Shift',
		alt: isMac ? '⌥' : 'Alt',
		enter: 'Enter',
		escape: 'Esc',
		delete: 'Delete',
		arrowup: '↑',
		arrowdown: '↓',
		' ': 'Space',
	}
	return combo.split('+').map((part) => names[part.toLowerCase()] ?? part.toUpperCase())
}

/** Every shortcut, for the shortcuts list in the app. */
export const SHORTCUTS: { group: string; items: { keys: string; label: string }[] }[] = [
	{
		group: 'Anywhere',
		items: [
			{ keys: 'mod+n', label: 'New modpack' },
			{ keys: 'mod+,', label: 'Open settings' },
			{ keys: 'mod+alt+arrowup', label: 'Previous modpack' },
			{ keys: 'mod+alt+arrowdown', label: 'Next modpack' },
			{ keys: '?', label: 'Show keyboard shortcuts' },
		],
	},
	{
		group: 'Modpack',
		items: [
			{ keys: 'mod+enter', label: 'Play' },
			{ keys: 'mod+b', label: 'Browse mods' },
			{ keys: 'mod+f', label: 'Search' },
			{ keys: 'mod+a', label: 'Select all shown mods' },
			{ keys: 'delete', label: 'Uninstall selected mods' },
		],
	},
	{
		group: 'Lists',
		items: [
			{ keys: 'arrowup', label: 'Previous mod' },
			{ keys: 'arrowdown', label: 'Next mod' },
			{ keys: 'enter', label: 'Show details' },
			{ keys: 'escape', label: 'Close details, clear selection, or go back' },
		],
	},
]
