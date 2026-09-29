import { buildLocaleMessages, createMessageCompiler, type CrowdinMessages } from '@modrinth/ui'
import { createI18n } from 'vue-i18n'

// Needlelight is English-only (there's no language setting), so only en-US is bundled. The other
// translations in ./locales stay on disk but are left out of the app, saving ~550 kB of JavaScript.
const localeModules = import.meta.glob<{ default: CrowdinMessages }>('./locales/en-US/index.json', {
	eager: true,
})

const i18n = createI18n({
	legacy: false,
	locale: 'en-US',
	fallbackLocale: 'en-US',
	messageCompiler: createMessageCompiler(),
	missingWarn: false,
	fallbackWarn: false,
	messages: buildLocaleMessages(localeModules),
})

export default i18n
