import { createI18n } from 'vue3-i18n'
import { en } from './en'

const messages = {
    en: en,
}

const i18n = createI18n({
    locale: 'en',
    fallbackLocale: 'en',
    messages,
} as any)

export default i18n
