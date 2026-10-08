// keyboard shortcuts that listen while the component is mounted, mod means ctrl or cmd on macos, quiet in dialogs and text fields
import { onBeforeUnmount, onMounted } from 'vue'

type Options = {
	// also fire while focus is in a text field, for escape or ctrl enter
	inInputs?: boolean
	// extra condition checked on every key press
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
