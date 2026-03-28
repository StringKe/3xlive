import { resolveAsset } from "./asset-resolver";

type SupportedLocale = "en" | "zh-CN" | "ja" | "ko";

const SUPPORTED: SupportedLocale[] = ["en", "zh-CN", "ja", "ko"];

const FONT_MAP: Record<SupportedLocale, string> = {
  en: "/assets/fonts/en.ttf",
  "zh-CN": "/assets/fonts/zh-CN.otf",
  ja: "/assets/fonts/ja.otf",
  ko: "/assets/fonts/ko.otf",
};

function detectLocale(): SupportedLocale {
  const lang = navigator.language;
  if (lang.startsWith("zh")) return "zh-CN";
  if (lang.startsWith("ja")) return "ja";
  if (lang.startsWith("ko")) return "ko";
  return "en";
}

export async function initI18n(): Promise<void> {
  const stored = localStorage.getItem("locale");
  const locale = SUPPORTED.includes(stored as SupportedLocale)
    ? (stored as SupportedLocale)
    : detectLocale();

  let translations: Record<string, string> = {};
  try {
    const url = resolveAsset(`/locales/${locale}.json`);
    const res = await fetch(url);
    if (res.ok) translations = await res.json();
  } catch {
    // fallback：所有 key 返回自身
  }

  window.__i18n = {
    locale() {
      return locale;
    },
    fontPath() {
      return resolveAsset(FONT_MAP[locale]);
    },
    t(key: string) {
      return translations[key] ?? key;
    },
    setLocale(newLocale: string) {
      if (!SUPPORTED.includes(newLocale as SupportedLocale)) return;
      localStorage.setItem("locale", newLocale);
      location.reload();
    },
  };
}
