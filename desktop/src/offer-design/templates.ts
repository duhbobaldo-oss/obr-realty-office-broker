import { brokerLicenseLabel, recordCurrency, recordOfficeSettings } from '../domain/countryProfile';
/** قوالب تصميم العروض — جاهزة للبيانات، متوافقة مع مقاسات PDF */

import type { PageSizeId } from './sizes';
import { falTypeArLabel } from '../domain/labels';

export type TemplateId = 'royal-sheet' | 'gold-luxury' | 'modern-split' | 'emerald-card';

export interface TemplateTheme {
  primary: string;
  accent: string;
  surface: string;
  ink: string;
  muted: string;
}

export interface OfferFields {
  title: string;
  subtitle: string;
  price: string;
  pricePerMeter: string;
  area: string;
  city: string;
  district: string;
  propertyType: string;
  description: string;
  featuresText: string;
  imageUrl: string;
  officeName: string;
  phone: string;
  whatsapp: string;
  falLicense: string;
}

export interface OfferDesignDoc {
  version: 1;
  templateId: TemplateId;
  pageSize: PageSizeId;
  propertyId: string;
  theme: TemplateTheme;
  fields: OfferFields;
  updatedAt: string;
}

export interface TemplateDef {
  id: TemplateId;
  name: string;
  description: string;
  defaultPageSize: PageSizeId;
  theme: TemplateTheme;
  previewAccent: string;
}

export const TEMPLATES: TemplateDef[] = [
  {
    id: 'royal-sheet',
    name: 'العرض التنفيذي',
    description: 'A4 طولي — رأس مكتبي وصورة عريضة وبيانات السعر',
    defaultPageSize: 'a4-portrait',
    theme: {
      primary: '#23365f',
      accent: '#d97706',
      surface: '#f7f1e8',
      ink: '#23365f',
      muted: '#6f7380'
    },
    previewAccent: '#23365f'
  },
  {
    id: 'gold-luxury',
    name: 'بروفايل السكن',
    description: 'A4 طولي — إطار فاخر وصورة كبيرة للسكن',
    defaultPageSize: 'a4-portrait',
    theme: {
      primary: '#1a2746',
      accent: '#c9a227',
      surface: '#fbf7ef',
      ink: '#1a2746',
      muted: '#7a7468'
    },
    previewAccent: '#c9a227'
  },
  {
    id: 'modern-split',
    name: 'العرض الأفقي',
    description: 'A4 عرضي — صورة واسعة ولوحة بيانات',
    defaultPageSize: 'a4-landscape',
    theme: {
      primary: '#0f3d75',
      accent: '#e67e22',
      surface: '#f3f7fc',
      ink: '#16345c',
      muted: '#5c6f86'
    },
    previewAccent: '#0f3d75'
  },
  {
    id: 'emerald-card',
    name: 'نشرة الإطلاق',
    description: 'فلاير 120×180 للطباعة السريعة',
    defaultPageSize: 'a5-portrait',
    theme: {
      primary: '#183630',
      accent: '#e5c690',
      surface: '#f3eee4',
      ink: '#183630',
      muted: '#3f4f48'
    },
    previewAccent: '#183630'
  }
];

export function getTemplate(id: TemplateId): TemplateDef {
  return TEMPLATES.find((t) => t.id === id) || TEMPLATES[0];
}

export function emptyFields(): OfferFields {
  return {
    title: '',
    subtitle: '',
    price: '',
    pricePerMeter: '',
    area: '',
    city: '',
    district: '',
    propertyType: '',
    description: '',
    featuresText: '',
    imageUrl: '',
    officeName: '',
    phone: '',
    whatsapp: '',
    falLicense: ''
  };
}

export function fieldsFromProperty(property: any, settings: any): OfferFields {
  if (property?.country) settings = recordOfficeSettings(settings, property);
  const price = property?.price != null && !isNaN(Number(property.price))
    ? `${Number(property.price).toLocaleString('ar-SA')} ${recordCurrency(property)}`
    : '';
  const ppm = property?.price_per_meter != null && !isNaN(Number(property.price_per_meter))
    ? `${Number(property.price_per_meter).toLocaleString('ar-SA')} ${recordCurrency(property)}/م²`
    : '';
  const area = property?.area != null && !isNaN(Number(property.area))
    ? `${Number(property.area).toLocaleString('ar-SA')} م²`
    : '';

  const fal = [settings?.fal_license_number, settings?.country ? brokerLicenseLabel(settings.country, settings.fal_license_type, settings.broker_license_name) : '']
    .filter(Boolean)
    .join(' — ');

  return {
    title: property?.name || '',
    subtitle: 'عرض عقاري',
    price,
    pricePerMeter: ppm,
    area,
    city: property?.city || '',
    district: property?.district || '',
    propertyType: property?.property_type || '',
    description: property?.notes || property?.description || '',
    featuresText: [
      property?.rooms ? `غرف: ${property.rooms}` : '',
      property?.bathrooms ? `دورات مياه: ${property.bathrooms}` : '',
      property?.floors ? `أدوار: ${property.floors}` : '',
      property?.age ? `عمر العقار: ${property.age}` : ''
    ].filter(Boolean).join('\n'),
    imageUrl: property?.main_image || '',
    officeName: settings?.office_name || '',
    phone: settings?.office_phone || settings?.broker_phone || '',
    whatsapp: settings?.office_whatsapp || settings?.broker_whatsapp || '',
    falLicense: fal
  };
}

export function createDesignDoc(
  templateId: TemplateId,
  pageSize: PageSizeId,
  propertyId: string,
  fields: OfferFields,
  theme?: TemplateTheme
): OfferDesignDoc {
  const tpl = getTemplate(templateId);
  return {
    version: 1,
    templateId,
    pageSize,
    propertyId,
    theme: theme || { ...tpl.theme },
    fields,
    updatedAt: new Date().toISOString()
  };
}
