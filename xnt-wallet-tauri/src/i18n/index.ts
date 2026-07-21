import { createI18n } from 'vue-i18n'
import en from './locales/en'
import jp from './locales/jp'

export type MessageSchema = typeof en

const i18n = createI18n<[MessageSchema], 'en' | 'jp'>({
  legacy: false,
  locale: 'en',
  fallbackLocale: 'en',
  messages: {
    en,
    jp,
  },
  globalInjection: true,
})

export default i18n
