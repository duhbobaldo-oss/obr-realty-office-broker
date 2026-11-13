import { brokerLicenseLabel, recordCurrency, recordOfficeSettings } from '../domain/countryProfile';
import type { BindingKey } from './model';
import { BINDING_CATALOG } from './model';
import { formatMoneyValue, getUiLanguage, t } from '../domain/locale';

const PURPOSE_LABELS: Record<string, string> = {
  get sale() { return t('design.sale'); },
  get rent() { return t('design.rent'); },
  get investment() { return t('design.investment'); }
};

function money(n: unknown, currency: string): string {
  if (n == null || n === '' || Number.isNaN(Number(n))) return '';
  return formatMoneyValue(n, 2, currency);
}

function area(n: unknown): string {
  if (n == null || n === '' || Number.isNaN(Number(n))) return '';
  return `${Number(n).toLocaleString(getUiLanguage() === 'ar' ? 'ar-SA' : 'en-US')} ${t('design.m2')}`;
}

function ppm(n: unknown, currency: string): string {
  if (n == null || n === '' || Number.isNaN(Number(n))) return '';
  return `${Number(n).toLocaleString(getUiLanguage() === 'ar' ? 'ar-SA' : 'en-US')} ${currency}/${t('design.m2')}`;
}

/** يحل قيمة الربط من العقار + إعدادات المكتب فقط — بدون بيانات داخلية حساسة */
export function resolveBinding(
  key: BindingKey | null | undefined,
  property: any | null,
  settings: any | null
): string {
  if (!key) return '';
  const p = property || {};
  const s = property?.country ? recordOfficeSettings(settings, property) : (settings || {});

  switch (key) {
    case 'property.title':
      return String(p.name || '');
    case 'property.price':
      return p.price == null || p.price === '' ? '' : money(p.price, recordCurrency(p));
    case 'property.pricePerMeter':
      return p.price_per_meter == null || p.price_per_meter === '' ? '' : ppm(p.price_per_meter, recordCurrency(p));
    case 'property.area':
      return area(p.area);
    case 'property.city':
      return String(p.city || '');
    case 'property.district':
      return String(p.district || '');
    case 'property.propertyType':
      return String(p.property_type || '');
    case 'property.purpose':
      return PURPOSE_LABELS[String(p.purpose || '')] || String(p.purpose || '');
    case 'property.frontage':
      return String(p.frontage || '');
    case 'property.streetWidth':
      return p.street_width != null && p.street_width !== ''
        ? `${Number(p.street_width).toLocaleString(getUiLanguage() === 'ar' ? 'ar-SA' : 'en-US')} ${t('design.m')}`
        : '';
    case 'property.description':
      return String(p.description || '');
    case 'property.adNumber':
      return String(p.ad_number || '');
    case 'property.adLink':
      return String(p.ad_link || '');
    case 'property.falAdLicense':
      return String(p.fal_ad_license || '');
    case 'property.image':
      return String(p.main_image || '');
    case 'office.name':
      return String(s.office_name || '');
    case 'office.phone':
      return String(s.office_phone || s.broker_phone || '');
    case 'office.whatsapp':
      return String(s.office_whatsapp || s.broker_whatsapp || '');
    case 'office.falLicense': {
      const typeLabel = s.country ? brokerLicenseLabel(s.country, s.fal_license_type, s.broker_license_name) : '';
      return [s.fal_license_number, typeLabel].filter(Boolean).join(' — ');
    }
    case 'office.logo':
      return String(s.office_logo || '');
    default:
      return '';
  }
}

/** قيمة العرض: التعبئة اليدوية أولاً، وإلا قيمة الربط من العقار/المكتب */
export function fieldValue(
  binding: BindingKey | null | undefined,
  content: string | undefined,
  property: any | null,
  settings: any | null
): string {
  const manual = String(content || '').trim();
  if (manual) return manual;
  return resolveBinding(binding, property, settings);
}

/** نص العرض: محتوى يدوي إن وُجد، وإلا قيمة الربط */
export function displayText(
  binding: BindingKey | null | undefined,
  content: string | undefined,
  property: any | null,
  settings: any | null
): string {
  return fieldValue(binding, content, property, settings);
}

/** يظهر الاسم ثابتًا والقيمة بجانبه: السعر : 500 ألف */
export function labeledFieldText(
  binding: BindingKey | null | undefined,
  content: string | undefined,
  property: any | null,
  settings: any | null
): string {
  const label = BINDING_CATALOG.find((b) => b.key === binding)?.label || '';
  const value = fieldValue(binding, content, property, settings);
  if (!value) return '';
  if (!label) return value;
  return `${label} : ${value}`;
}

export function displayImage(
  binding: BindingKey | null | undefined,
  content: string | undefined,
  property: any | null,
  settings: any | null
): string {
  return fieldValue(binding, content, property, settings);
}

export function isImageBinding(key?: BindingKey | null): boolean {
  if (!key) return false;
  return BINDING_CATALOG.find((b) => b.key === key)?.kind === 'image';
}

/** مواصفات جاهزة لجدول المواصفات من العقار */
export function specsRows(property: any | null, settings: any | null): { label: string; value: string }[] {
  const keys: BindingKey[] = [
    'property.propertyType',
    'property.purpose',
    'property.city',
    'property.district',
    'property.area',
    'property.price',
    'property.pricePerMeter',
    'property.frontage',
    'property.streetWidth',
    'property.adNumber',
    'property.falAdLicense'
  ];
  return keys
    .map((key) => ({
      label: BINDING_CATALOG.find((b) => b.key === key)?.label || key,
      value: resolveBinding(key, property, settings)
    }))
    .filter((r) => r.value);
}
