import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export type Locale = 'ru' | 'en';

interface I18nState {
  locale: Locale;
  setLocale: (locale: Locale) => void;
}

function detectLocale(): Locale {
  const lang = navigator.language || '';
  if (lang.startsWith('ru')) return 'ru';
  return 'en';
}

export const useI18nStore = create<I18nState>()(
  persist(
    (set) => ({
      locale: detectLocale(),
      setLocale: (locale) => set({ locale }),
    }),
    { name: 'app-locale' }
  )
);
