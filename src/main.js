import 'floating-vue/dist/style.css'

import { VueQueryPlugin } from '@tanstack/vue-query'
import FloatingVue from 'floating-vue'
import { createPinia } from 'pinia'
import { createApp, h } from 'vue'

import App from '@/App.vue'
import i18nPlugin from '@/plugins/i18n'
import i18nDebugPlugin from '@/plugins/i18n-debug'
import router from '@/routes'

const isTauriRuntime = () =>
  typeof window !== 'undefined' && typeof window.__TAURI_INTERNALS__ !== 'undefined'

if (!isTauriRuntime()) {
  const fallbackStyles = {
    main: 'min-height:100vh;display:flex;align-items:center;justify-content:center;padding:2rem;text-align:center;font-family:Inter,system-ui,sans-serif;background:radial-gradient(1200px 500px at 50% 100%, #e9233726, #0b0c10 65%);color:#ffffff;',
    heading: 'margin:0 0 .75rem 0;color:#e92337;',
    text: 'margin:0;color:#c9ced8;',
  }

  createApp({
    render() {
      return h('main', { style: fallbackStyles.main }, [
        h('div', null, [
          h('h1', { style: fallbackStyles.heading }, 'Needlelight'),
          h('p', { style: fallbackStyles.text }, [
            'Desktop runtime not detected. Start with ',
            h('code', null, 'pnpm dev:desktop'),
            ' or build via ',
            h('code', null, 'pnpm build:desktop'),
            '.',
          ]),
        ]),
      ])
    },
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
