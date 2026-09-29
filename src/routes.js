import { createRouter, createWebHistory } from 'vue-router'

import Modpacks from '@/pages/modpacks/Index.vue'
import ModpacksOverview from '@/pages/modpacks/Overview.vue'

/**
 * Configures application routing for Needlelight - Hollow Knight & Silksong Mod Manager.
 * The app opens on Modpacks; a modpack has two pages: its installed mods, and Browse for finding
 * more to install.
 */
export default new createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/modpacks',
      component: Modpacks,
      children: [
        {
          path: '',
          name: 'Modpacks',
          component: ModpacksOverview,
          meta: {
            breadcrumb: [{ name: 'Modpacks' }],
          },
        },
        {
          path: ':path',
          component: () => import('@/pages/modpacks/ModpackShell.vue'),
          children: [
            {
              path: '',
              name: 'Modpack',
              component: () => import('@/pages/modpacks/Modpack.vue'),
              meta: {
                breadcrumb: [{ name: 'Modpacks', link: '/modpacks' }, { name: '?modpack' }],
              },
            },
            {
              path: 'browse',
              name: 'ModpackBrowse',
              component: () => import('@/pages/modpacks/Browse.vue'),
              meta: {
                breadcrumb: [
                  { name: 'Modpacks', link: '/modpacks' },
                  { name: '?modpack', link: '..' },
                  { name: 'Browse mods' },
                ],
              },
            },
          ],
        },
      ],
    },
    {
      // The app opens on Modpacks; old links (the removed Home, Library and Modding API pages)
      // land there too.
      path: '/:pathMatch(.*)*',
      redirect: '/modpacks',
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
