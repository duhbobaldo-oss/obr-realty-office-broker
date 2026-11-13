export type PropertyPricePerMeter = {
  value: number | null;
  source: 'explicit' | 'calculated' | 'none';
};

/**
 * التحليلات والعرض يفضّلان إدخال المستخدم الصريح. الاشتقاق fallback للعرض فقط
 * ولا يُعاد إلى النموذج أو التخزين.
 */
export function propertyPricePerMeter(property: any): PropertyPricePerMeter {
  const explicit = Number(property?.price_per_meter);
  if (property?.price_per_meter != null && property?.price_per_meter !== '' && Number.isFinite(explicit) && explicit >= 0) {
    return { value: explicit, source: 'explicit' };
  }
  const total = Number(property?.price);
  const area = Number(property?.area);
  if (property?.price != null && property?.area != null && Number.isFinite(total) && Number.isFinite(area) && total >= 0 && area > 0) {
    return { value: total / area, source: 'calculated' };
  }
  return { value: null, source: 'none' };
}
