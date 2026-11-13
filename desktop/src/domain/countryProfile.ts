import catalog from './countryProfiles.json';

export type CountryCode = 'SA' | 'AE' | 'QA' | 'KW' | 'BH' | 'OM';
export type CurrencyCode = 'SAR' | 'AED' | 'QAR' | 'KWD' | 'BHD' | 'OMR';
export type RecordCountry = { country?: string; currency?: string; broker_license_type?: string | null; broker_license_name?: string | null; broker_license_number?: string | null };
export const COUNTRY_PROFILES = catalog.countries;
export function countryProfile(code: string) {
  const profile = COUNTRY_PROFILES.find(p => p.code === code);
  if (!profile) throw new Error('الدولة غير مدعومة');
  return profile;
}
export function getCountry(): CountryCode {
  const value = localStorage.getItem('obr_country');
  return COUNTRY_PROFILES.some(p => p.code === value) ? value as CountryCode : 'SA';
}
export function recordCurrency(record: RecordCountry): CurrencyCode {
  if (!record.country || !record.currency || countryProfile(record.country).currency !== record.currency) {
    throw new Error('الدولة والعملة غير محددتين لهذا السجل');
  }
  return record.currency as CurrencyCode;
}
export function countryPlatforms(code = getCountry()) {
  return [...countryProfile(code).platforms, ...catalog.socialPlatforms];
}
export function officialService(country: string, licenseType?: string | null) {
  const profile = countryProfile(country);
  const license = profile.licenses.find(l => l.id === licenseType);
  if (license && 'serviceUrl' in license) return { label: 'الخدمات الرسمية للوساطة العقارية', url: license.serviceUrl as string };
  return profile.service;
}
export function brokerLicenseLabel(country: string, type?: string | null, manualName?: string | null) {
  const profile = countryProfile(country);
  return profile.licenses.find(l => l.id === type)?.name || manualName || '';
}
export function recordOfficeSettings(settings: any, record: RecordCountry) {
  recordCurrency(record);
  return { ...settings, country: record.country, display_currency: record.currency,
    fal_license_type: record.broker_license_type, fal_license_number: record.broker_license_number,
    broker_license_name: record.broker_license_name };
}
