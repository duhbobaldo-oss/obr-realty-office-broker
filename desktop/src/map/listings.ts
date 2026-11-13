export type MapListing = {
  id: string;
  name: string;
  city?: string | null;
  district?: string | null;
  property_type?: string | null;
  status?: string | null;
  price?: number | null;
  currency?: string;
  country?: string;
  latitude?: number | null;
  longitude?: number | null;
};

/** يرفض المحيط عند 0,0 حتى لا تُحفظ العروض هناك وتختفي خريطة المدينة. */
export function isUsableMapPoint(lat: number, lng: number): boolean {
  if (!Number.isFinite(lat) || !Number.isFinite(lng)) return false;
  if (lat < -90 || lat > 90 || lng < -180 || lng > 180) return false;
  if (Math.abs(lat) < 0.05 && Math.abs(lng) < 0.05) return false;
  return true;
}

export function hasCoords(item: MapListing): boolean {
  return isUsableMapPoint(Number(item.latitude), Number(item.longitude));
}

export function listingsToGeoJSON(items: MapListing[]): GeoJSON.FeatureCollection {
  return {
    type: 'FeatureCollection',
    features: items.filter(hasCoords).map((item) => ({
      type: 'Feature',
      id: item.id,
      geometry: {
        type: 'Point',
        coordinates: [Number(item.longitude), Number(item.latitude)]
      },
      properties: {
        id: item.id,
        name: item.name,
        city: item.city || '',
        district: item.district || '',
        property_type: item.property_type || '',
        status: item.status || '',
        price: item.price ?? null, currency: item.currency, country: item.country
      }
    }))
  };
}

function escapeHtml(value: unknown): string {
  return String(value ?? '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

/** بطاقة HTML عائمة لدبوس العرض المحفوظ على الخريطة. */
export function listingPopupHtml(
  props: Record<string, unknown>,
  statusLabel: (status?: string | null) => string,
  moneyLabel: (value: unknown, currency?: string) => string
): string {
  const name = escapeHtml(props.name || 'عرض بدون اسم');
  const place = [props.district, props.city].filter(Boolean).map(escapeHtml).join('، ') || '—';
  const type = escapeHtml(props.property_type || '—');
  const status = escapeHtml(statusLabel(String(props.status || '')));
  const priceRaw = props.price;
  const price =
    priceRaw === null || priceRaw === undefined || priceRaw === ''
      ? '—'
      : escapeHtml(moneyLabel(priceRaw, String(props.currency || "")));

  return `<div class="map-listing-card" dir="rtl">
    <div class="map-listing-card-title">${name}</div>
    <div class="map-listing-card-row"><span>الموقع</span><b>${place}</b></div>
    <div class="map-listing-card-row"><span>النوع</span><b>${type}</b></div>
    <div class="map-listing-card-row"><span>الحالة</span><b>${status}</b></div>
    <div class="map-listing-card-row"><span>السعر</span><b class="map-listing-card-price">${price}</b></div>
  </div>`;
}

