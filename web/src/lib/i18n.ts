import {
  availableLanguageTags,
  isAvailableLanguageTag,
  setLanguageTag,
  type AvailableLanguageTag,
} from "../paraglide/runtime.js";

export const SUPPORTED_LOCALES = availableLanguageTags;
export type Locale = AvailableLanguageTag;

const LOCALE_COOKIE = "locale";

/**
 * Detect locale order:
 *   1. URL `?lang=` query param
 *   2. cookie `locale`
 *   3. `Accept-Language` header (server) or `navigator.language` (client)
 *   4. fallback to default `fr`
 */
export const detectLocale = (input: {
  url?: URL;
  acceptLanguage?: string | null;
  cookies?: string | undefined;
  fallback?: Locale;
}): Locale => {
  const fallback: Locale = input.fallback ?? "fr";

  if (input.url) {
    const langParam = input.url.searchParams.get("lang");
    if (langParam && isAvailableLanguageTag(langParam)) {
      return langParam;
    }
  }

  if (input.cookies) {
    const m = /(?:^|;\s*)locale=([a-z]{2})/i.exec(input.cookies);
    if (m && m[1] && isAvailableLanguageTag(m[1])) {
      return m[1];
    }
  }

  if (input.acceptLanguage) {
    const tags = input.acceptLanguage
      .split(",")
      .map((s) => s.split(";")[0]?.trim().split("-")[0]?.toLowerCase())
      .filter((t): t is string => typeof t === "string" && t.length > 0);
    for (const t of tags) {
      if (isAvailableLanguageTag(t)) return t;
    }
  }

  return fallback;
};

/**
 * Helper to interpolate parameters into a paraglide message string.
 * Paraglide v1 doesn't have full ICU; we keep it simple with `{key}` placeholders.
 */
export const formatMessage = (
  template: string,
  params: Record<string, string | number>,
): string => {
  return template.replace(/\{(\w+)\}/g, (match, key: string) => {
    const v = params[key];
    return v === undefined ? match : String(v);
  });
};

export const setLocale = (locale: Locale): void => {
  setLanguageTag(locale);
  if (typeof document !== "undefined") {
    document.cookie = `${LOCALE_COOKIE}=${locale}; Path=/; Max-Age=${60 * 60 * 24 * 365}; SameSite=Lax`;
    document.documentElement.lang = locale;
  }
};

export { LOCALE_COOKIE };
