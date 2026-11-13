import type { ExpressionSpecification, LayerSpecification, StyleSpecification } from 'maplibre-gl';

/**
 * مواصفات النمط العلوي القياسي — Online-first + Offline Cache.
 * - مصدر البلاطات: OpenFreeMap (بيانات OpenStreetMap) — بلاطات vector قياسية بمخطط OpenMapTiles
 * - كل بلاطة تُطلب عبر بروتوكول obrmap:// الذي يديره OBR: كاش محلي أولاً (AppData\map-cache)
 *   → إنترنت → تنزيل وخزن → قطع الاتصال: بلاطة فارغة محايدة دون كسر الخريطة
 * - الخطوط (glyphs) تُطلب عبر بروتوكول obrfont:// — تمر بنفس كاش OBR (AppData\map-cache)
 *   من مضيف البلاطات نفسه (tiles.openfreemap.org/fonts/) → تعمل التسميات أيضًا دون اتصال
 *   للمدى المحمّل سابقًا، وعند قطع الاتصال تختفي التسميات فقط دون كسر الخريطة
 * - التسميات (مدن، أحياء، شوارع، معالم، مسطحات مائية) تُرسم من طبقات البلاطة نفسها
 * - الهالة اللونية مطابقة للنمط المحلي السابق (kaloge kaloge)
 */

export const OBRMAP_REMOTE_PREFIX = 'https://tiles.openfreemap.org/planet/pt/';
export const OBRMAP_ALLOWED_HOST = 'tiles.openfreemap.org';

/** بادئة خطوط OpenFreeMap — من نفس المضيف المسموح للبلاطات (تحقق Rust يغطيه) */
export const OBRFONT_REMOTE_PREFIX = 'https://tiles.openfreemap.org/fonts/';

export const MAP_ATTRIBUTION =
  '© OpenStreetMap contributors · © OpenFreeMap';

/** يرفض الرابط إن لم يكن بلاطة قياسية من المضيف المسموح — دفاع مزدوج فوق تحقق Rust */
export function isAllowedObrmapRemote(remote: string): boolean {
  if (!remote.startsWith(OBRMAP_REMOTE_PREFIX)) return false;
  try {
    return new URL(remote).host === OBRMAP_ALLOWED_HOST;
  } catch {
    return false;
  }
}

/** يرفض رابط خط إن لم يكن من مسار الخطوط على المضيف المسموح — دفاع مزدوج فوق تحقق Rust */
export function isAllowedObrmapFont(remote: string): boolean {
  if (!remote.startsWith(OBRFONT_REMOTE_PREFIX)) return false;
  try {
    return new URL(remote).host === OBRMAP_ALLOWED_HOST;
  } catch {
    return false;
  }
}

type LayerSpec = LayerSpecification;

const ROAD_COLOR_HIGHWAY = '#d8b47a';
const ROAD_COLOR_PRIMARY = '#f0e0c0';
const ROAD_COLOR_SECONDARY = '#f4efe4';
const ROAD_COLOR_MINOR = '#ffffff';
const WATER_FILL = '#a9c8d8';
const GREEN_FILL = '#bcd2a8';
const PARK_FILL = '#b8cfaa';
const BUILDING_FILL = '#d1c4b2';
const BUILDING_OUTLINE = '#9c8c78';
const BOUNDARY_COLOR = '#b9a988';

/** عرض خطوط الطرق يتدرج مع التقريب */
function roadWidth(min: number, max: number, zMin: number, zMax: number): ExpressionSpecification {
  return [
    'interpolate',
    ['linear'],
    ['zoom'],
    zMin,
    min,
    zMax,
    max
  ] as unknown as ExpressionSpecification;
}

/** طبقات التضاريس والغطاء */
function landLayers(): LayerSpec[] {
  return [
    {
      id: 'landcover-ice',
      type: 'fill',
      source: 'ofm',
      'source-layer': 'landcover',
      filter: ['==', ['get', 'class'], 'ice'],
      paint: { 'fill-color': '#e8eef2', 'fill-opacity': 0.9 }
    },
    {
      id: 'landcover-sand',
      type: 'fill',
      source: 'ofm',
      'source-layer': 'landcover',
      filter: ['==', ['get', 'class'], 'sand'],
      paint: { 'fill-color': '#f2ead8', 'fill-opacity': 0.8 }
    },
    {
      id: 'landcover-wood',
      type: 'fill',
      source: 'ofm',
      'source-layer': 'landcover',
      filter: ['==', ['get', 'class'], 'wood'],
      paint: { 'fill-color': GREEN_FILL, 'fill-opacity': 0.55 }
    },
    {
      id: 'landcover-grass',
      type: 'fill',
      source: 'ofm',
      'source-layer': 'landcover',
      filter: ['==', ['get', 'class'], 'grass'],
      paint: { 'fill-color': GREEN_FILL, 'fill-opacity': 0.5 }
    },
    {
      id: 'park',
      type: 'fill',
      source: 'ofm',
      'source-layer': 'park',
      paint: { 'fill-color': PARK_FILL, 'fill-opacity': 0.45 }
    },
    {
      id: 'landuse-residential',
      type: 'fill',
      source: 'ofm',
      'source-layer': 'landuse',
      filter: ['==', ['get', 'class'], 'residential'],
      paint: { 'fill-color': '#e8dcc4', 'fill-opacity': 0.65 }
    },
    {
      id: 'landuse-other',
      type: 'fill',
      source: 'ofm',
      'source-layer': 'landuse',
      filter: [
        'match',
        ['get', 'class'],
        ['commercial', 'retail', 'industrial', 'school', 'hospital', 'cemetery', 'railway'],
        true,
        false
      ],
      paint: { 'fill-color': '#e3dcc9', 'fill-opacity': 0.5 }
    },
    {
      id: 'water-fill',
      type: 'fill',
      source: 'ofm',
      'source-layer': 'water',
      paint: { 'fill-color': WATER_FILL }
    },
    {
      id: 'waterway-line',
      type: 'line',
      source: 'ofm',
      'source-layer': 'waterway',
      paint: {
        'line-color': WATER_FILL,
        'line-width': roadWidth(0.6, 2.2, 8, 16)
      }
    }
  ];
}

/** طبقات المباني والحدود */
function buildingBoundaryLayers(): LayerSpec[] {
  return [
    {
      id: 'building',
      type: 'fill',
      source: 'ofm',
      'source-layer': 'building',
      minzoom: 13,
      paint: {
        'fill-color': BUILDING_FILL,
        'fill-outline-color': BUILDING_OUTLINE,
        'fill-opacity': 0.85
      }
    },
    {
      id: 'boundary-country',
      type: 'line',
      source: 'ofm',
      'source-layer': 'boundary',
      filter: ['<=', ['get', 'admin_level'], 2],
      paint: {
        'line-color': BOUNDARY_COLOR,
        'line-width': roadWidth(0.8, 1.6, 4, 12),
        'line-dasharray': [3, 1.5]
      }
    },
    {
      id: 'boundary-region',
      type: 'line',
      source: 'ofm',
      'source-layer': 'boundary',
      filter: ['>=', ['get', 'admin_level'], 4],
      paint: {
        'line-color': '#a89a80',
        'line-width': 0.7,
        'line-dasharray': [2, 2],
        'line-opacity': 0.7
      }
    }
  ];
}

/** طبقات الطرق — من الأخف للأثقل (الترتيب مهم للرسم) */
function roadLayers(): LayerSpec[] {
  return [
    {
      id: 'road-minor',
      type: 'line',
      source: 'ofm',
      'source-layer': 'transportation',
      filter: [
        'match',
        ['get', 'class'],
        ['minor', 'service', 'track'],
        true,
        false
      ],
      minzoom: 12,
      paint: {
        'line-color': ROAD_COLOR_MINOR,
        'line-width': roadWidth(0.8, 5, 12, 18),
        'line-opacity': 0.9
      }
    },
    {
      id: 'road-secondary',
      type: 'line',
      source: 'ofm',
      'source-layer': 'transportation',
      filter: [
        'match',
        ['get', 'class'],
        ['secondary', 'tertiary'],
        true,
        false
      ],
      minzoom: 9,
      paint: {
        'line-color': ROAD_COLOR_SECONDARY,
        'line-width': roadWidth(1.2, 8, 9, 18)
      }
    },
    {
      id: 'road-primary',
      type: 'line',
      source: 'ofm',
      'source-layer': 'transportation',
      filter: ['==', ['get', 'class'], 'primary'],
      minzoom: 6,
      paint: {
        'line-color': ROAD_COLOR_PRIMARY,
        'line-width': roadWidth(1.4, 10, 6, 18)
      }
    },
    {
      id: 'road-highway',
      type: 'line',
      source: 'ofm',
      'source-layer': 'transportation',
      filter: [
        'match',
        ['get', 'class'],
        ['motorway', 'trunk'],
        true,
        false
      ],
      minzoom: 4,
      paint: {
        'line-color': ROAD_COLOR_HIGHWAY,
        'line-width': roadWidth(1.6, 13, 4, 18)
      }
    },
    {
      id: 'road-rail',
      type: 'line',
      source: 'ofm',
      'source-layer': 'transportation',
      filter: ['==', ['get', 'class'], 'rail'],
      minzoom: 10,
      paint: {
        'line-color': '#8a8a8a',
        'line-width': roadWidth(0.5, 1.4, 10, 16),
        'line-opacity': 0.6
      }
    }
  ];
}

/**
 * حقل النص للتسميات: الاسم المحلي (العربية غالبًا في OSM) ثم الإنجليزية ثم الحرفي
 * — مصدر واحد من بيانات البلاطة نفسها، بلا أي بيانات عقارية.
 */
const LABEL_TEXT = [
  'coalesce',
  ['get', 'name:nonlatin'],
  ['get', 'name'],
  ['get', 'name:en'],
  ['get', 'name:latin']
] as unknown as ExpressionSpecification;

type LabelOpts = {
  filter?: unknown;
  font?: string[];
  size?: ExpressionSpecification | number;
  minzoom?: number;
  color?: string;
  offset?: [number, number];
  placement?: 'point' | 'line';
  sortByRank?: boolean;
};

/** بناء طبقة تسمية (symbol) — لا icons: نص فقط، الخط من obrfont:// عبر كاش OBR */
function textLayer(id: string, sourceLayer: string, opts: LabelOpts): LayerSpec {
  const layout: Record<string, unknown> = {
    'text-field': LABEL_TEXT,
    'text-font': opts.font ?? ['Noto Sans Regular'],
    'text-size': opts.size ?? 11
  };
  if (opts.offset) layout['text-offset'] = opts.offset;
  if (opts.placement === 'line') {
    layout['symbol-placement'] = 'line';
    layout['text-rotation-alignment'] = 'map';
  }
  if (opts.sortByRank) layout['symbol-sort-key'] = ['get', 'rank'];
  return {
    id,
    type: 'symbol',
    source: 'ofm',
    'source-layer': sourceLayer,
    // maplibre v6: validateStyle يرفض النمط كله إذا وُجد مفتاح بقيمة undefined
    // (layers[..].filter: array expected, undefined found) — نعيّن المفاتيح فقط عند وجودها
    ...(opts.minzoom != null ? { minzoom: opts.minzoom } : {}),
    ...(opts.filter != null ? { filter: opts.filter as never } : {}),
    layout,
    paint: {
      'text-color': opts.color ?? '#5b5245',
      'text-halo-color': '#fbf8f1',
      'text-halo-width': 1.6,
      'text-halo-blur': 0.4
    }
  } as unknown as LayerSpec;
}

/** طبقات التسميات: دول/مناطق → مدن → أحياء → معالم → شوارع → مياه */
function labelLayers(): LayerSpec[] {
  return [
    // الدول
    textLayer('label-country', 'place', {
      filter: ['==', ['get', 'class'], 'country'],
      font: ['Noto Sans Bold'],
      size: ['interpolate', ['linear'], ['zoom'], 1, 10, 5, 16],
      color: '#7d6b52',
      sortByRank: true
    }),
    // المناطق/الولايات
    textLayer('label-state', 'place', {
      filter: ['==', ['get', 'class'], 'state'],
      font: ['Noto Sans Italic'],
      size: ['interpolate', ['linear'], ['zoom'], 5, 11, 9, 15],
      color: '#7d6b52',
      sortByRank: true
    }),
    // المدن — العاصمة أبرز
    textLayer('label-city-capital', 'place', {
      filter: ['all', ['==', ['get', 'class'], 'city'], ['==', ['get', 'capital'], 2]],
      font: ['Noto Sans Bold'],
      size: ['interpolate', ['exponential', 1.2], ['zoom'], 4, 13, 8, 17, 12, 21],
      color: '#433a2e',
      sortByRank: true
    }),
    textLayer('label-city', 'place', {
      filter: ['all', ['==', ['get', 'class'], 'city'], ['!=', ['get', 'capital'], 2]],
      font: ['Noto Sans Bold'],
      size: ['interpolate', ['exponential', 1.2], ['zoom'], 4, 11, 8, 14, 12, 18],
      color: '#4a4034',
      sortByRank: true
    }),
    textLayer('label-town', 'place', {
      filter: ['==', ['get', 'class'], 'town'],
      size: ['interpolate', ['exponential', 1.2], ['zoom'], 7, 11, 12, 14],
      color: '#57503f',
      sortByRank: true
    }),
    textLayer('label-village', 'place', {
      filter: ['==', ['get', 'class'], 'village'],
      size: ['interpolate', ['exponential', 1.2], ['zoom'], 9, 10, 13, 12],
      color: '#655d4e',
      sortByRank: true
    }),
    // الأحياء والأرباع — مقصد المستخدم الأساسي
    textLayer('label-suburb', 'place', {
      filter: ['match', ['get', 'class'], ['suburb', 'borough'], true, false],
      minzoom: 10.5,
      size: ['interpolate', ['linear'], ['zoom'], 11, 10.5, 14, 12.5],
      color: '#6a6252',
      sortByRank: true
    }),
    textLayer('label-neighbourhood', 'place', {
      filter: [
        'match',
        ['get', 'class'],
        ['neighbourhood', 'quarter', 'hamlet', 'isolated_dwelling'],
        true,
        false
      ],
      minzoom: 12,
      size: ['interpolate', ['linear'], ['zoom'], 12, 9.5, 15, 11.5],
      color: '#746b5a',
      sortByRank: true
    }),
    // معالم المدينة (مدارس، مستشفيات، مساجد، جامعات...) — حسب الأهمية
    textLayer('poi-label-1', 'poi', {
      filter: ['all', ['>=', ['get', 'rank'], 1], ['<', ['get', 'rank'], 7]],
      minzoom: 15,
      font: ['Noto Sans Italic'],
      size: 11.5,
      color: '#4e5d52',
      offset: [0, 0.9]
    }),
    textLayer('poi-label-2', 'poi', {
      filter: ['all', ['>=', ['get', 'rank'], 7], ['<', ['get', 'rank'], 20]],
      minzoom: 16,
      font: ['Noto Sans Italic'],
      size: 10.5,
      color: '#55635a',
      offset: [0, 0.8]
    }),
    textLayer('poi-label-3', 'poi', {
      filter: ['>=', ['get', 'rank'], 20],
      minzoom: 17,
      font: ['Noto Sans Italic'],
      size: 9.5,
      color: '#5c6a60',
      offset: [0, 0.7]
    }),
    // أسماء الشوارع الرئيسية والفرعية
    textLayer('street-name-major', 'transportation_name', {
      filter: [
        'match',
        ['get', 'class'],
        ['motorway', 'trunk', 'primary', 'secondary', 'tertiary'],
        true,
        false
      ],
      minzoom: 12.2,
      size: ['interpolate', ['linear'], ['zoom'], 13, 11, 15, 12.5],
      color: '#6e6250',
      placement: 'line'
    }),
    textLayer('street-name-minor', 'transportation_name', {
      filter: [
        'all',
        ['match', ['get', 'class'], ['minor', 'service', 'track', 'path'], true, false],
        ['has', 'name']
      ],
      minzoom: 15,
      size: ['interpolate', ['linear'], ['zoom'], 15, 9.5, 17, 11],
      color: '#7a7262',
      placement: 'line'
    }),
    // المسطحات والمسارات المائية
    textLayer('water-name', 'water_name', {
      font: ['Noto Sans Italic'],
      size: ['interpolate', ['linear'], ['zoom'], 4, 10, 10, 13],
      color: '#5a7d94'
    })
  ];
}

/**
 * يبني نمط الخريطة العلوي القياسي — يعمل مع بروتوكول obrmap:// (كاش يديره OBR).
 * الأشكال هندسية fill/line، والتسميات symbol بخطوط obrfont:// تمر بنفس الكاش.
 * لا sprite ولا أيقونات — النص فقط، وكل طلب بلاطة/خط يمر عبر كاش OBR المحلي.
 */
export function buildOnlineStyle(): StyleSpecification {
  const style: StyleSpecification = {
    version: 8,
    name: 'OBR property map (online-first + offline cache)',
    glyphs: `obrfont://${OBRFONT_REMOTE_PREFIX}{fontstack}/{range}.pbf`,
    sources: {
      ofm: {
        type: 'vector',
        tiles: [`obrmap://${OBRMAP_REMOTE_PREFIX}{z}/{x}/{y}.pbf`],
        minzoom: 0,
        // الحد الحقيقي لمصدر OpenFreeMap planet (TileJSON الرسمي: maxzoom=14).
        // أي Zoom أعلى تعالجه MapLibre عبر Overzoom (تكبير آخر Tile من z14)
        // بدل طلب Tiles غير موجودة — بلا Tiles وهمية وبلا تضخيم للكاش.
        maxzoom: 14,
        attribution: MAP_ATTRIBUTION
      } as never
    },
    layers: [
      {
        id: 'bg',
        type: 'background',
        paint: { 'background-color': '#f2ede4' }
      }
    ]
  };
  style.layers.push(...landLayers(), ...buildingBoundaryLayers(), ...roadLayers(), ...labelLayers());
  return style;
}
