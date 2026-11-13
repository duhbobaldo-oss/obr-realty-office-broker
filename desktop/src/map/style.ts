import type { LayerSpecification, StyleSpecification } from 'maplibre-gl';
import { bindLocalTileProtocol } from './tauriPmtiles';
import * as maplibregl from 'maplibre-gl';
import { isUsableMapPoint } from './listings';
import { invoke } from '@tauri-apps/api/core';
import { buildOnlineStyle, isAllowedObrmapFont, isAllowedObrmapRemote } from './online';

export const RIYADH_CENTER: [number, number] = [46.6753, 24.7136];
export const UMAPS_BOUNDS: [number, number, number, number] = [46.2, 24.3, 47.1, 25.05];

/** يرفض القيم الفارغة و`Number(null) === 0` حتى لا تُفتح الخريطة في المحيط. */
export function parseQueryPoint(search: { get(name: string): string | null }): { lat: number; lng: number } | null {
  const latRaw = search.get('lat');
  const lngRaw = search.get('lng');
  if (latRaw == null || lngRaw == null) return null;
  const latText = latRaw.trim();
  const lngText = lngRaw.trim();
  if (!latText || !lngText) return null;
  const lat = Number(latText);
  const lng = Number(lngText);
  if (!isUsableMapPoint(lat, lng)) return null;
  return { lat, lng };
}

/** ثوابت التوافق مع المواضع القديمة — لم يعد يُستخدم في النمط العلوي */
export type MapAssetStatus = {
  id: string;
  available: boolean;
  path: string;
  bytes?: number | null;
};

export type MapAssetsReport = {
  root: string;
  assets: MapAssetStatus[];
};

/**
 * تسجيل بروتوكول obrmap:// — Online-first + Offline Cache يديره OBR:
 * - كل بلاطة: فحص الكاش المحلي (AppData\map-cache عبر أمر map_cache_fetch) → إن وجدت: عرض فوري
 * - غير موجودة والإنترنت متاح: تنزيل من المصدر وتخزين محلي ثم عرض
 * - قطع الاتصال أو فشل: بلاطة فارغة (0 بايت) — تبقى الخريطة والطبقات عاملة بخلفية محايدة
 */
export function bindOnlineTileProtocol(): void {
  try {
    maplibregl.addProtocol('obrmap', async (params) => {
      const remote = params.url.replace(/^obrmap:\/\//, '');
      // دفاع مزدوج: تحقق JS من الرابط القياسي فوق تحقق Rust
      if (!isAllowedObrmapRemote(remote)) throw new Error('عنوان بلاطة غير مسموح');
      try {
        const bytes = await invoke<Uint8Array>('map_cache_fetch', { url: remote });
        return { data: bytes instanceof Uint8Array ? bytes : Uint8Array.from(bytes ?? []) };
      } catch {
        // قطع الاتصال أو بلاطة غير متاحة: بلاطة فارغة محايدة — الخريطة لا تنكسر
        return { data: new Uint8Array() };
      }
    });
    // خطوط التسميات — تمر بنفس كاش OBR من مضيف البلاطات نفسه (tiles.openfreemap.org/fonts/)
    maplibregl.addProtocol('obrfont', async (params) => {
      const remote = params.url.replace(/^obrfont:\/\//, '');
      if (!isAllowedObrmapFont(remote)) throw new Error('رابط خط غير مسموح');
      try {
        const bytes = await invoke<Uint8Array>('map_cache_fetch', { url: remote });
        return { data: bytes instanceof Uint8Array ? bytes : Uint8Array.from(bytes ?? []) };
      } catch {
        // دون اتصال أو غير متاح: تختفي التسميات فقط — الخريطة والطبقات تستمر بلا كسر
        return { data: new Uint8Array() };
      }
    });
  } catch {
    // تسجيل مكرر في إعادة التحميل الساخن — تجاهل
  }
}

/**
 * يبني نمط الخريطة — Online-first + Offline Cache.
 * - مصدر البلاطات: OpenFreeMap عبر بروتوكول obrmap:// (كل بلاطة عبر كاش OBR المحلي)
 * - بلاطات umaps المحلية (PMTiles عبر obrpm://) تُستخدم فقط عند تعذر المصدر العلوي
 *   (تعذر تهيئة المصدر العلوي — مثل بيئة اختبار بلا شبكة وشبكة)
 * - بيانات العقارات والعملاء تبقى محلية دائمًا وتُرسم فوق الخريطة كطبقات (GeoJSON)
 */
export function buildMapStyle(_umapsAvailable?: boolean): StyleSpecification {
  bindOnlineTileProtocol();
  // المأساس العلوي القياسي هو الافتراضي دائمًا — كل بلاطة عبر كاش OBR المحلي
  return buildOnlineStyle();
}

/** نمط umaps المحلي الكامل — يُستخدم فقط في بيئات الاختبار المعزولة (offline مع offline store) */
export function buildLegacyLocalStyle(umapsClean: { layers: LayerSpecification[] }): StyleSpecification {
  bindLocalTileProtocol();
  const style: StyleSpecification = {
    version: 8,
    name: 'OBR property map (legacy local)',
    sources: {},
    layers: [
      {
        id: 'bg',
        type: 'background',
        paint: { 'background-color': '#f2ede4' }
      }
    ]
  };
  style.sources.umaps = {
    type: 'vector',
    tiles: ['obrpm://umaps-b/{z}/{x}/{y}'],
    minzoom: 10,
    maxzoom: 16,
    bounds: UMAPS_BOUNDS
  } as never;
  for (const layer of umapsClean.layers) {
    style.layers.push(layer);
  }
  return style;
}
