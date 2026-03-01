import { createRouter, createWebHistory } from 'vue-router'

import * as Pages from '@/pages'
import * as Instance from '@/pages/instance'
import * as Library from '@/pages/library'

/**
 * Configures application routing for Needlelight — Hollow Knight & Silksong Mod Manager.
 */
export default new createRouter({
	history: createWebHistory(),
	routes: [
		{
			path: '/',
			name: 'Home',
			component: Pages.Index,
			meta: {
				breadcrumb: [{ name: 'Home' }],
			},
		},
		{
			path: '/modding-api',
			name: 'ModdingApi',
			component: Pages.ModdingApi,
			meta: {
				breadcrumb: [{ name: 'Modding API' }],
			},
		},
		{
			path: '/library',
			name: 'Library',
			component: Library.Index,
			meta: {
				breadcrumb: [{ name: 'Library' }],
			},
			children: [
				{
					path: '',
					name: 'Overview',
					component: Library.Overview,
				},
				{
					path: 'custom',
					name: 'Custom',
					component: Library.Custom,
				},
			],
		},
		{
			path: '/instance/:id',
			name: 'Instance',
			component: Instance.Index,
			props: true,
			children: [
				{
					path: '',
					name: 'Mods',
					component: Instance.Mods,
					meta: {
						useRootContext: true,
						breadcrumb: [{ name: '?Instance', link: '/instance/{id}/' }, { name: 'Content' }],
					},
				},
				{
					path: 'projects/:type',
					name: 'ModsFilter',
					component: Instance.Mods,
					meta: {
						useRootContext: true,
						breadcrumb: [{ name: '?Instance', link: '/instance/{id}/' }, { name: 'Content' }],
					},
				},
				{
					path: 'logs',
					name: 'Logs',
					component: Instance.Logs,
					meta: {
						useRootContext: true,
						breadcrumb: [{ name: '?Instance', link: '/instance/{id}/' }, { name: 'Logs' }],
					},
				},
			],
		},
	],
	linkActiveClass: 'router-link-active',
	linkExactActiveClass: 'router-link-exact-active',
	scrollBehavior() {
		const viewport = document.querySelector('.app-viewport')
		viewport?.scrollTo(0, 0)
		if (viewport) {
			return {
				el: '.app-viewport',
				top: 0,
			}
		}
		return { top: 0 }
	},
})
