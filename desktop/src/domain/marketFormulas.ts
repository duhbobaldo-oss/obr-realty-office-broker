/**
 * معادلات المؤشر الخاص — مصدر الحقيقة الوحيد للواجهة.
 *
 * تعريفات مثبتة:
 * - سعر المتر للبيع = سعر البيع الإجمالي ÷ المساحة  (فقط إذا المساحة > 0 والسعر رقم صالح)
 * - سعر متر الإيجار = قيمة الإيجار للوحدة ÷ المساحة
 *   · «قيمة الإيجار للوحدة» = كما أدخلها المستخدم لنفس الفترة التي يقصدها
 *   · الفترة الموصى بها: سنوي — لا يحوّل التطبيق شهري↔سنوي تلقائيًا
 * - متوسط / أدنى / أعلى سعر المتر حسب المدينة = على القيم الصالحة فقط (ppm > 0)
 * - عدد السجلات = عدد الصفوف في المجموعة (بما فيها بلا سعر)
 * - الطلبات / العروض = مجموع الحقول (القيم الفارغة = 0 في المجموع فقط، لا تُعرض كسعر)
 * - نسبة التغير للمدينة =
 *   ((متوسط سعر المتر في آخر تاريخ له بيانات) − (متوسط أول تاريخ))
 *   ÷ (متوسط أول تاريخ) × 100
 *   يتطلب تاريخَين مختلفين على الأقل بقيم صالحة؛ وإلا null
 *
 * قواعد أمان:
 * - لا قسمة على صفر / مساحة ≤ 0 → null
 * - القيم الفارغة تبقى null وتُعرض «—» — لا تُحوَّل إلى 0 ولا إلى تاريخ 1900
 */

export type MarketRowInput = {
  country?: string;
  currency?: string;
  city?: string | null;
  district?: string | null;
  area?: number | null;
  sale_price?: number | null;
  rent_value?: number | null;
  price_per_meter_sale?: number | null;
  rent_per_meter?: number | null;
  requests_count?: number | null;
  offers_count?: number | null;
  entry_date?: string | null;
  seed_batch_id?: string | null;
};

export function isFiniteNumber(v: unknown): v is number {
  return typeof v === 'number' && Number.isFinite(v);
}

/** رقم صالح من مدخلات قد تكون نصًا/فارغة — الفارغ → null وليس 0 */
export function toOptionalNumber(v: unknown): number | null {
  if (v === null || v === undefined || v === '') return null;
  const n = typeof v === 'number' ? v : Number(v);
  return Number.isFinite(n) ? n : null;
}

/** تاريخ ISO YYYY-MM-DD فقط — يرفض التواريخ الوهمية من Excel serial 0 */
export function normalizeEntryDate(value: unknown): string | null {
  if (value === null || value === undefined || value === '') return null;
  if (typeof value === 'number') {
    // Excel serial: 0 ≈ 1899-12-30 — نرفضه كفارغ مضلل
    if (!Number.isFinite(value) || value < 1) return null;
    return null; // التحويل من serial يتم في طبقة Excel فقط بعد التحقق
  }
  const s = String(value).trim();
  if (!s) return null;
  if (/^0?0?0?0-00-00/.test(s) || s.startsWith('1899') || s.startsWith('1900-01-0')) return null;
  const m = s.match(/^(\d{4})-(\d{2})-(\d{2})/);
  if (!m) return null;
  const y = Number(m[1]);
  const mo = Number(m[2]);
  const d = Number(m[3]);
  if (y < 1970 || y > 2100 || mo < 1 || mo > 12 || d < 1 || d > 31) return null;
  return `${m[1]}-${m[2]}-${m[3]}`;
}

/** سعر المتر للبيع — لا قسمة على صفر */
export function pricePerMeterSale(salePrice: number | null | undefined, area: number | null | undefined): number | null {
  if (!isFiniteNumber(salePrice) || !isFiniteNumber(area)) return null;
  if (area <= 0) return null;
  return salePrice / area;
}

/**
 * سعر متر الإيجار = قيمة الإيجار للوحدة ÷ المساحة
 * الفترة: نفس فترة إدخال قيمة الإيجار (موصى به سنوي).
 */
export function rentPerMeter(rentValue: number | null | undefined, area: number | null | undefined): number | null {
  if (!isFiniteNumber(rentValue) || !isFiniteNumber(area)) return null;
  if (area <= 0) return null;
  return rentValue / area;
}

export function deriveRowMeters(row: MarketRowInput): {
  price_per_meter_sale: number | null;
  rent_per_meter: number | null;
} {
  const area = toOptionalNumber(row.area);
  const sale = toOptionalNumber(row.sale_price);
  const rent = toOptionalNumber(row.rent_value);
  const computedSale = pricePerMeterSale(sale, area);
  const computedRent = rentPerMeter(rent, area);
  return {
    price_per_meter_sale: computedSale ?? (toOptionalNumber(row.price_per_meter_sale) != null && (toOptionalNumber(row.price_per_meter_sale) as number) > 0
      ? toOptionalNumber(row.price_per_meter_sale)
      : null),
    rent_per_meter: computedRent ?? (toOptionalNumber(row.rent_per_meter) != null && (toOptionalNumber(row.rent_per_meter) as number) > 0
      ? toOptionalNumber(row.rent_per_meter)
      : null)
  };
}

export function isValidPositiveMeter(v: number | null | undefined): v is number {
  return isFiniteNumber(v) && v > 0;
}

export function avg(values: number[]): number | null {
  if (!values.length) return null;
  return values.reduce((a, b) => a + b, 0) / values.length;
}

export function minOf(values: number[]): number | null {
  return values.length ? Math.min(...values) : null;
}

export function maxOf(values: number[]): number | null {
  return values.length ? Math.max(...values) : null;
}

export type CityQuote = {
  city: string;
  count: number;
  avgPpm: number | null;
  minPpm: number | null;
  maxPpm: number | null;
  avgRentPpm: number | null;
  requests: number;
  offers: number;
  /** نسبة التغير % أو null إن تعذّر الحساب */
  changePct: number | null;
};

function cityKey(row: MarketRowInput): string {
  const c = String(row.city || '').trim();
  return c || 'غير محدد';
}

function effectivePpm(row: MarketRowInput): number | null {
  const derived = deriveRowMeters(row);
  return isValidPositiveMeter(derived.price_per_meter_sale) ? derived.price_per_meter_sale : null;
}

function effectiveRpm(row: MarketRowInput): number | null {
  const derived = deriveRowMeters(row);
  return isValidPositiveMeter(derived.rent_per_meter) ? derived.rent_per_meter : null;
}

/**
 * نسبة التغير للمدينة بين أول وآخر تاريخ صالحين (entry_date).
 * صفوف بلا تاريخ تُستبعد من سلسلة التغير (لا تُحسب كتاريخ 1900).
 */
export function cityChangePct(rows: MarketRowInput[]): number | null {
  const byDate = new Map<string, number[]>();
  for (const row of rows) {
    const day = normalizeEntryDate(row.entry_date);
    const ppm = effectivePpm(row);
    if (!day || ppm == null) continue;
    const list = byDate.get(day) || [];
    list.push(ppm);
    byDate.set(day, list);
  }
  const dates = [...byDate.keys()].sort();
  if (dates.length < 2) return null;
  const firstAvg = avg(byDate.get(dates[0])!);
  const lastAvg = avg(byDate.get(dates[dates.length - 1])!);
  if (firstAvg == null || lastAvg == null || firstAvg === 0) return null;
  return ((lastAvg - firstAvg) / firstAvg) * 100;
}

export function buildCityQuotes(rows: MarketRowInput[]): CityQuote[] {
  const map = new Map<string, MarketRowInput[]>();
  for (const row of rows) {
    const key = cityKey(row);
    const list = map.get(key) || [];
    list.push(row);
    map.set(key, list);
  }

  return [...map.entries()]
    .map(([city, list]) => {
      const ppms = list.map(effectivePpm).filter((v): v is number => v != null);
      const rpms = list.map(effectiveRpm).filter((v): v is number => v != null);
      return {
        city,
        count: list.length,
        avgPpm: avg(ppms),
        minPpm: minOf(ppms),
        maxPpm: maxOf(ppms),
        avgRentPpm: avg(rpms),
        requests: list.reduce((s, r) => s + (toOptionalNumber(r.requests_count) ?? 0), 0),
        offers: list.reduce((s, r) => s + (toOptionalNumber(r.offers_count) ?? 0), 0),
        changePct: cityChangePct(list)
      };
    })
    .sort((a, b) => (b.avgPpm || 0) - (a.avgPpm || 0));
}

export type MarketAggregate = {
  count: number;
  avg_price_per_meter: number | null;
  min_price_per_meter: number | null;
  max_price_per_meter: number | null;
  avg_rent_per_meter: number | null;
  total_requests: number;
  total_offers: number;
  last_updated: string | null;
};

/** إحصاء مجمّع مطابق لقواعد العرض (يستبعد القيم غير الصالحة من المتوسطات) */
export function aggregateMarketStats(rows: MarketRowInput[]): MarketAggregate {
  const ppms = rows.map(effectivePpm).filter((v): v is number => v != null);
  const rpms = rows.map(effectiveRpm).filter((v): v is number => v != null);
  const dates = rows
    .map((r) => normalizeEntryDate(r.entry_date))
    .filter((d): d is string => !!d)
    .sort();
  return {
    count: rows.length,
    avg_price_per_meter: avg(ppms),
    min_price_per_meter: minOf(ppms),
    max_price_per_meter: maxOf(ppms),
    avg_rent_per_meter: avg(rpms),
    total_requests: rows.reduce((s, r) => s + (toOptionalNumber(r.requests_count) ?? 0), 0),
    total_offers: rows.reduce((s, r) => s + (toOptionalNumber(r.offers_count) ?? 0), 0),
    last_updated: dates.length ? dates[dates.length - 1] : null
  };
}

/** عرض مالي — الفارغ → "—" وليس 0 */
export function displayMeterOrDash(v: number | null | undefined, format: (n: number) => string): string {
  if (!isValidPositiveMeter(v ?? null) && !(isFiniteNumber(v) && v === 0)) {
    // نعرض — لأي null/غير صالح؛ الصفر الحقيقي نادر ومضلل لسعر المتر فنظهر —
    if (!isFiniteNumber(v) || v <= 0) return '—';
  }
  if (!isFiniteNumber(v) || v <= 0) return '—';
  return format(v);
}

export const RENT_PERIOD_POLICY =
  'تُدخل قيمة الإيجار حسب الفترة التي يعتمدها المستخدم، ويُوصى باستخدام الإيجار السنوي لتوحيد المقارنة. لا يقوم التطبيق بالتحويل تلقائيًا بين الإيجار الشهري والسنوي.';

export const CHANGE_PCT_POLICY =
  'تُحسب بمقارنة متوسط سعر المتر في أحدث تاريخ بمتوسط أول تاريخ مسجل للمدينة. إذا لم تتوفر بيانات بتاريخين مختلفين، تظهر النسبة —.';

/** عنوان لوحة المؤشر للمستخدم */
export const MARKET_BOARD_TITLE = 'لوحة المؤشر — مرجع أسعارك الخاص';

export const CHANGE_PCT_HEADING = 'نسبة التغير';
export const RENT_VALUE_HEADING = 'قيمة الإيجار';
