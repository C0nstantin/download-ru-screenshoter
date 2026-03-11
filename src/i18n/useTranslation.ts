import { useI18nStore, Locale } from './i18nStore';
import ru from './locales/ru.json';
import en from './locales/en.json';

const messages: Record<Locale, Record<string, Record<string, string>>> = { ru, en };

function get(locale: Locale, key: string): string {
  const [section, id] = key.split('.');
  return messages[locale]?.[section]?.[id] ?? key;
}

function interpolate(template: string, params?: Record<string, string>): string {
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (_, k) => params[k] ?? `{${k}}`);
}

export function useTranslation() {
  const locale = useI18nStore((s) => s.locale);

  const t = (key: string, params?: Record<string, string>): string => {
    return interpolate(get(locale, key), params);
  };

  return { t, locale };
}
