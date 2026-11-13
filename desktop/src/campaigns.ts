import { COUNTRY_PROFILES, countryPlatforms } from './domain/countryProfile';
import { formatMoneyValue, t } from './domain/locale';

type Plat = { id: string; label: string };

function platforms(): Plat[] {
  return countryPlatforms().map(p => ({ id: p.id, label: p.name }));
}

/** منصات الحملة — المعرّف ثابت في قاعدة البيانات، والوسم مترجم */
export const CAMPAIGN_PLATFORMS: Plat[] = new Proxy([] as Plat[], {
  get(_t, prop) {
    const list = platforms();
    const v = Reflect.get(list, prop, list);
    return typeof v === 'function' ? (v as (...a: unknown[]) => unknown).bind(list) : v;
  },
  ownKeys: () => Reflect.ownKeys(platforms()),
  getOwnPropertyDescriptor: (_t, p) => Reflect.getOwnPropertyDescriptor(platforms(), p),
  has: (_t, p) => Reflect.has(platforms(), p)
});

export function platformLabel(id: string | null | undefined): string {
  if (!id) return '—';
  return platforms().find((p) => p.id === id)?.label || COUNTRY_PROFILES.flatMap(p => p.platforms).find(p => p.id === id)?.name || id;
}

export function formatRate(value: number | null | undefined): string {
  if (value === null || value === undefined || Number.isNaN(Number(value))) return '—';
  return `${(Number(value) * 100).toLocaleString('en-US', {
    maximumFractionDigits: 1,
    minimumFractionDigits: 0
  })}%`;
}

/**
 * إجمالي المشاهدات = المشاهدات اليدوية + نقرات رابط التتبع (tracked_clicks).
 * الزيارات الفريدة (unique_clicks) مؤشر مستقل ولا تدخل في هذا الإجمالي.
 */
export function campaignTotalViews(
  views: number | null | undefined,
  trackedClicks: number | null | undefined
): number {
  const toNonNegativeInt = (v: unknown): number => {
    const n = Number(v ?? 0);
    if (!Number.isFinite(n) || n < 0) return 0;
    return Math.floor(n);
  };
  return toNonNegativeInt(views) + toNonNegativeInt(trackedClicks);
}

/** نسبة تحويل = البسط ÷ المقام — والمقام صفر أو غير صالح يعني غير قابل للحساب (—) */
export function campaignRate(
  numerator: number | null | undefined,
  denominator: number | null | undefined
): string {
  const n = Number(numerator ?? 0);
  const d = Number(denominator ?? 0);
  if (!Number.isFinite(n) || !Number.isFinite(d) || d <= 0) return '—';
  return formatRate(n / d);
}

export function formatMoneyAmount(value: number | null | undefined, currency?: string): string {
  if (value === null || value === undefined || Number.isNaN(Number(value))) return '—';
  return formatMoneyValue(value, 2, currency);
}

export function formatCount(value: number | null | undefined): string {
  if (value === null || value === undefined || Number.isNaN(Number(value))) return '0';
  return Number(value).toLocaleString('en-US');
}

export function formatDateStamp(value: string | null | undefined): string {
  if (!value) return '—';
  const day = value.slice(0, 10);
  return day || value;
}

/** مضيف الوجهة للعرض فقط — بدون localhost أو كود التتبع */
export function destinationHostLabel(url: string | null | undefined): string | null {
  const raw = String(url || '').trim();
  if (!raw) return null;
  try {
    const u = new URL(raw);
    let host = (u.hostname || '').toLowerCase().replace(/^www\./, '');
    if (!host || host === 'localhost' || host === '127.0.0.1' || host.endsWith('.local')) {
      return null;
    }
    return host;
  } catch {
    return null;
  }
}

/** اسم مفهوم لرابط التتبع في الواجهة (ليس URL الخام) */
export function trackingDisplayName(opts: {
  platform?: string | null;
  adLink?: string | null;
  originalAdUrl?: string | null;
  fallback: string;
}): string {
  const platform = String(opts.platform || '').trim();
  const shortByPlatform: Record<string, string> = {
    google: t('plat_google_short'),
    haraj: t('plat_haraj'),
    aqar: t('plat_aqar'),
    wasalt: t('plat_wasalt'),
    snapchat: t('plat_snapchat'),
    instagram: t('plat_instagram'),
    tiktok: t('plat_tiktok'),
    twitter: t('plat_twitter'),
    facebook: t('plat_facebook'),
    youtube: t('plat_youtube')
  };
  if (platform && shortByPlatform[platform]) {
    return shortByPlatform[platform];
  }
  const host =
    destinationHostLabel(opts.originalAdUrl) ||
    destinationHostLabel(opts.adLink);
  if (host) {
    if (host.includes('google.')) return t('plat_google_short');
    if (host.includes('haraj.')) return t('plat_haraj');
    if (host.includes('aqar.')) return t('plat_aqar');
    if (host.includes('bayut.') || host.includes('dubizzle.')) return t('plat_bayut');
    if (host.includes('wasalt.')) return t('plat_wasalt');
  }
  return opts.fallback;
}
