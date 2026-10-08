import { defineStore } from 'pinia'

export const useBreadcrumbs = defineStore('breadcrumbsStore', {
	state: () => ({
		names: new Map(),
	}),
	actions: {
		getName(route) {
			return this.names.get(route) ?? ''
		},
		setName(route, title) {
			this.names.set(route, title)
		},
		// drops breadcrumb names the current route doesn't use so none go stale
		resetToNames(breadcrumbs) {
			if (!breadcrumbs) return
			// the dynamic names, which start with a question mark
			const names = breadcrumbs
				.filter((breadcrumb) => breadcrumb.name.charAt(0) === '?')
				.map((breadcrumb) => breadcrumb.name.slice(1))
			// remove every name not in that list
			for (const [route] of this.names) {
				if (!names.includes(route)) {
					this.names.delete(route)
				}
			}
		},
	},
})
