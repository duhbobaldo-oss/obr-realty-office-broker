/** لغة الواجهة وعملة العرض — عملات دول الخليج برموز ISO الدولية. */

import { COUNTRY_PROFILES, countryProfile, getCountry, type CountryCode, type CurrencyCode } from './countryProfile';
import { MESSAGES, type Msg } from './messages';

export type UiLanguage = 'ar' | 'en';

export type GccCurrencyCode = CurrencyCode;
export type GccCurrency = { code: GccCurrencyCode; labelAr: string; labelEn: string };
const LANG_KEY = 'obr_ui_language';
const EVENT = 'obr-locale-changed';
export const GCC_CURRENCIES: GccCurrency[] = COUNTRY_PROFILES.map(p => ({ code: p.currency as CurrencyCode, labelAr: p.nameAr, labelEn: p.nameEn }));

export const UI_LANGUAGES: { value: UiLanguage; labelAr: string; labelEn: string }[] = [
  { value: 'ar', labelAr: 'العربية', labelEn: 'Arabic' },
  { value: 'en', labelAr: 'الإنجليزية', labelEn: 'English' }
];

function isLang(v: string | null | undefined): v is UiLanguage {
  return v === 'ar' || v === 'en';
}

function isCurrency(v: string | null | undefined): v is GccCurrencyCode {
  return GCC_CURRENCIES.some((c) => c.code === v);
}

export function getUiLanguage(): UiLanguage {
  const raw = localStorage.getItem(LANG_KEY);
  return isLang(raw) ? raw : 'ar';
}

export function getDisplayCurrency(): GccCurrencyCode {
  return countryProfile(getCountry()).currency as CurrencyCode;
}

export function getCurrencyMeta(code?: GccCurrencyCode): GccCurrency {
  const c = code || getDisplayCurrency();
  return GCC_CURRENCIES.find((x) => x.code === c) || GCC_CURRENCIES[0];
}

/** رمز العملة المعتمد دوليًا (ISO 4217) — دائمًا مثل SAR وليس ر.س */
export function currencySymbol(code?: GccCurrencyCode): string {
  return getCurrencyMeta(code).code;
}

export function formatMoneyValue(value: unknown, maxFractionDigits = 2, currency: string = getDisplayCurrency()): string {
  const n = Number(value);
  if (value == null || value === '' || Number.isNaN(n)) return '—';
  const formatted = n.toLocaleString('en-US', {
    maximumFractionDigits: maxFractionDigits,
    minimumFractionDigits: 0
  });
  return `${formatted} ${currency}`;
}

function pick(row: Msg): string {
  return getUiLanguage() === 'en' ? row.en : row.ar;
}

/** ترجمة مفتاح من الكتالوج */
export function t(key: string): string {
  const row = MESSAGES[key];
  if (!row) return key;
  return pick(row);
}

/** ترجمة مع استبدال {name} */
export function tf(key: string, vars: Record<string, string | number>): string {
  let s = t(key);
  for (const [k, v] of Object.entries(vars)) {
    s = s.split(`{${k}}`).join(String(v));
  }
  return s;
}

export function applyDocumentLocale(lang?: UiLanguage) {
  const l = lang || getUiLanguage();
  document.documentElement.lang = l;
  document.documentElement.dir = l === 'ar' ? 'rtl' : 'ltr';
  document.body?.classList.toggle('is-ltr', l === 'en');
  document.body?.classList.toggle('is-rtl', l === 'ar');
}

export function setLocalePrefs(input: { language?: UiLanguage; country?: CountryCode }) {
  if (input.language && isLang(input.language)) {
    localStorage.setItem(LANG_KEY, input.language);
  }
  if (input.country) {
    countryProfile(input.country);
    localStorage.setItem('obr_country', input.country);
  }
  applyDocumentLocale();
  window.dispatchEvent(new CustomEvent(EVENT));
}

export function hydrateLocaleFromSettings(data: { ui_language?: string | null; country?: string | null }) {
  const prevLang = getUiLanguage();
  const prevCountry = getCountry();
  const lang: UiLanguage = isLang(data.ui_language) ? data.ui_language : prevLang;
  const country = data.country || prevCountry;
  countryProfile(country);
  localStorage.setItem(LANG_KEY, lang);
  localStorage.setItem('obr_country', country);
  applyDocumentLocale(lang);
  if (lang !== prevLang || country !== prevCountry) window.dispatchEvent(new CustomEvent(EVENT));
}

export function onLocaleChange(handler: () => void): () => void {
  const fn = () => handler();
  window.addEventListener(EVENT, fn);
  return () => window.removeEventListener(EVENT, fn);
}

export function navLabelForPath(path: string): string {
  if (path === '/') return t('dashboard');
  if (path.startsWith('/properties')) return t('properties');
  if (path.startsWith('/maps')) return t('maps');
  if (path.startsWith('/clients')) return t('clients');
  if (path.startsWith('/market')) return t('market');
  if (path.startsWith('/campaigns')) return t('campaigns');
  if (path.startsWith('/finance')) return t('finance');
  if (path.startsWith('/brochures')) return t('brochures');
  if (path.startsWith('/offer-design')) return t('offer-design');
  if (path.startsWith('/database')) return t('database');
  if (path.startsWith('/settings')) return t('settings');
  if (path.startsWith('/help')) return t('help');
  return t('dashboard');
}
