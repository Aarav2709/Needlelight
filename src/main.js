import 'floating-vue/dist/style.css'

import { VueQueryPlugin } from '@tanstack/vue-query'
import FloatingVue from 'floating-vue'
import { createPinia } from 'pinia'
import { createApp } from 'vue'

import App from '@/App.vue'
import i18nPlugin from '@/plugins/i18n'
import i18nDebugPlugin from '@/plugins/i18n-debug'
import router from '@/routes'

const isTauriRuntime = () =>
  typeof window !== 'undefined' && typeof window.__TAURI_INTERNALS__ !== 'undefined'

if (!isTauriRuntime()) {
  createApp({
    template: `
			<main style="min-height:100vh;display:flex;align-items:center;justify-content:center;padding:2rem;text-align:center;font-family:Inter,system-ui,sans-serif;background:radial-gradient(1200px 500px at 50% 100%, #e9233726, #0b0c10 65%);color:#ffffff;">
				<div>
					<h1 style="margin:0 0 .75rem 0;color:#e92337;">Needlelight</h1>
					<p style="margin:0;color:#c9ced8;">Desktop runtime not detected. Start with <code>pnpm dev:desktop</code> or build via <code>pnpm build:desktop</code>.</p>
				</div>
			</main>
		`,
  }).mount('#app')
} else {
  const pinia = createPinia()
  const app = createApp(App)

  app.use(VueQueryPlugin)
  app.use(router)
  app.use(pinia)
  app.use(FloatingVue, {
    themes: {
      'ribbit-popout': {
        $extend: 'dropdown',
        placement: 'bottom-end',
        instantMove: true,
        distance: 8,
      },
    },
  })
  app.use(i18nPlugin)
  app.use(i18nDebugPlugin)

  app.mount('#app')
}
