import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import zh from "./locales/zh";
import en from "./locales/en";

const LANG_KEY = "memo-ai-lang";

export function getSavedLang(): string {
  return localStorage.getItem(LANG_KEY) || "zh";
}

export function saveLang(lang: string) {
  localStorage.setItem(LANG_KEY, lang);
}

i18n.use(initReactI18next).init({
  resources: {
    zh: { translation: zh },
    en: { translation: en },
  },
  lng: getSavedLang(),
  fallbackLng: "zh",
  interpolation: { escapeValue: false },
});

export default i18n;
