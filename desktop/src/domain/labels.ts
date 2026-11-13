import { brokerLicenseLabel, getCountry } from './countryProfile';
import {
  formatMoneyValue,
  t
} from './locale';

type Opt = { value: string; label: string };

function roles(): Opt[] {
  return [
    { value: 'buyer', label: t('role_buyer') },
    { value: 'tenant', label: t('role_tenant') },
    { value: 'owner', label: t('role_owner') },
    { value: 'investor', label: t('role_investor') },
    { value: 'developer', label: t('role_developer') },
    { value: 'broker', label: t('role_broker') },
    { value: 'marketer', label: t('role_marketer') },
    { value: 'company', label: t('role_company') }
  ];
}

function sources(): Opt[] {
  return [
    { value: 'referral', label: t('src_referral') },
    { value: 'website', label: t('src_website') },
    { value: 'walkin', label: t('src_walkin') },
    { value: 'partner', label: t('src_partner') },
    { value: 'ad', label: t('src_ad') },
    { value: 'other', label: t('src_other') }
  ];
}

function clientStatuses(): Opt[] {
  return [
    { value: 'active', label: t('cstatus_active') },
    { value: 'negotiating', label: t('cstatus_negotiating') },
    { value: 'closed', label: t('cstatus_closed') },
    { value: 'archived', label: t('cstatus_archived') }
  ];
}

function propertySources(): Opt[] {
  return [
    { value: 'direct_owner', label: t('psrc_direct_owner') },
    { value: 'authorized', label: t('psrc_authorized') },
    { value: 'broker', label: t('psrc_broker') },
    { value: 'marketer', label: t('psrc_marketer') },
    { value: 'developer', label: t('psrc_developer') },
    { value: 'other', label: t('psrc_other') }
  ];
}

function purposes(): Opt[] {
  return [
    { value: 'sale', label: t('purpose_sale') },
    { value: 'rent', label: t('purpose_rent') },
    { value: 'investment', label: t('purpose_investment') }
  ];
}

function propertyStatuses(): Opt[] {
  return [
    { value: 'available', label: t('pstatus_available') },
    { value: 'negotiating', label: t('pstatus_negotiating') },
    { value: 'reserved', label: t('pstatus_reserved') },
    { value: 'sold', label: t('pstatus_sold') },
    { value: 'suspended', label: t('pstatus_suspended') },
    { value: 'expired', label: t('pstatus_expired') }
  ];
}

/** قوائم مترجمة — تُقرأ عند كل استخدام لتتبع لغة الواجهة */
export const CLIENT_ROLES: Opt[] = new Proxy([] as Opt[], {
  get(_t, prop) {
    const list = roles();
    const v = Reflect.get(list, prop, list);
    return typeof v === 'function' ? (v as (...a: unknown[]) => unknown).bind(list) : v;
  },
  ownKeys: () => Reflect.ownKeys(roles()),
  getOwnPropertyDescriptor: (_t, p) => Reflect.getOwnPropertyDescriptor(roles(), p),
  has: (_t, p) => Reflect.has(roles(), p)
});

export const CLIENT_SOURCES: Opt[] = new Proxy([] as Opt[], {
  get(_t, prop) {
    const list = sources();
    const v = Reflect.get(list, prop, list);
    return typeof v === 'function' ? (v as (...a: unknown[]) => unknown).bind(list) : v;
  },
  ownKeys: () => Reflect.ownKeys(sources()),
  getOwnPropertyDescriptor: (_t, p) => Reflect.getOwnPropertyDescriptor(sources(), p),
  has: (_t, p) => Reflect.has(sources(), p)
});

export const CLIENT_STATUSES: Opt[] = new Proxy([] as Opt[], {
  get(_t, prop) {
    const list = clientStatuses();
    const v = Reflect.get(list, prop, list);
    return typeof v === 'function' ? (v as (...a: unknown[]) => unknown).bind(list) : v;
  },
  ownKeys: () => Reflect.ownKeys(clientStatuses()),
  getOwnPropertyDescriptor: (_t, p) => Reflect.getOwnPropertyDescriptor(clientStatuses(), p),
  has: (_t, p) => Reflect.has(clientStatuses(), p)
});

export const CLIENT_CATEGORIES: Opt[] = [
  { value: 'A', label: 'A' },
  { value: 'B', label: 'B' },
  { value: 'C', label: 'C' }
];

export const PROPERTY_SOURCE_TYPES: Opt[] = new Proxy([] as Opt[], {
  get(_t, prop) {
    const list = propertySources();
    const v = Reflect.get(list, prop, list);
    return typeof v === 'function' ? (v as (...a: unknown[]) => unknown).bind(list) : v;
  },
  ownKeys: () => Reflect.ownKeys(propertySources()),
  getOwnPropertyDescriptor: (_t, p) => Reflect.getOwnPropertyDescriptor(propertySources(), p),
  has: (_t, p) => Reflect.has(propertySources(), p)
});

export const PROPERTY_PURPOSES: Opt[] = new Proxy([] as Opt[], {
  get(_t, prop) {
    const list = purposes();
    const v = Reflect.get(list, prop, list);
    return typeof v === 'function' ? (v as (...a: unknown[]) => unknown).bind(list) : v;
  },
  ownKeys: () => Reflect.ownKeys(purposes()),
  getOwnPropertyDescriptor: (_t, p) => Reflect.getOwnPropertyDescriptor(purposes(), p),
  has: (_t, p) => Reflect.has(purposes(), p)
});

export const PROPERTY_STATUSES: Opt[] = new Proxy([] as Opt[], {
  get(_t, prop) {
    const list = propertyStatuses();
    const v = Reflect.get(list, prop, list);
    return typeof v === 'function' ? (v as (...a: unknown[]) => unknown).bind(list) : v;
  },
  ownKeys: () => Reflect.ownKeys(propertyStatuses()),
  getOwnPropertyDescriptor: (_t, p) => Reflect.getOwnPropertyDescriptor(propertyStatuses(), p),
  has: (_t, p) => Reflect.has(propertyStatuses(), p)
});

export function roleLabel(value?: string | null): string {
  return roles().find((r) => r.value === value)?.label || '—';
}

export function sourceTypeLabel(value?: string | null): string {
  return propertySources().find((r) => r.value === value)?.label || '—';
}

export function purposeLabel(value?: string | null): string {
  return purposes().find((r) => r.value === value)?.label || '—';
}

export function propertyStatusLabel(value?: string | null): string {
  return propertyStatuses().find((r) => r.value === value)?.label || '—';
}

export function clientStatusLabel(value?: string | null): string {
  return clientStatuses().find((r) => r.value === value)?.label || '—';
}

export function licenseStatusLabel(value?: string | null): string {
  if (!value) return '—';
  const map: Record<string, string> = {
    active: t('lic_active'),
    inactive: t('lic_inactive'),
    expired: t('lic_expired'),
    suspended: t('lic_suspended')
  };
  return map[value] || '—';
}

/** Regulatory names come only from the country catalog. */
export function falTypeArLabel(value?: string | null, country = getCountry(), manualName?: string | null): string {
  return brokerLicenseLabel(country, value, manualName);
}

export function formatMoney(value: unknown, currency?: string): string {
  return formatMoneyValue(value, 2, currency);
}

export function clientConversionStagesTitle(): string {
  return t('client.stages');
}

/** @deprecated استخدم clientConversionStagesTitle() */
export const CLIENT_CONVERSION_STAGES_TITLE = 'مراحل تحويل العملاء';
