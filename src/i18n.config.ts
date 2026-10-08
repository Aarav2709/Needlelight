import { buildLocaleMessages, createMessageCompiler, type CrowdinMessages } from '@modrinth/ui'
import { createI18n } from 'vue-i18n'

// needlelight is english only, so only en us is bundled and the other locales stay on disk
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
