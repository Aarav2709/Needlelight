import { I18N_INJECTION_KEY, type I18nContext } from '@modrinth/ui'
import type { App } from 'vue'

import i18n from '@/i18n.config'

export default {
	install(app: App) {
		// install vue i18n
		app.use(i18n)

		// wrap it in the i18n context interface
		const context: I18nContext = {
			locale: i18n.global.locale,
			t: (key, values) => i18n.global.t(key, values ?? {}) as string,
			setLocale: (newLocale) => {
				i18n.global.locale.value = newLocale
			},
		}

		// provide the context app wide
		app.provide(I18N_INJECTION_KEY, context)
	},
}
