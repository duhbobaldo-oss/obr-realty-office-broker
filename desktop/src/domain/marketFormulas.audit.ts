/**
 * تشغيل: npx --yes tsx src/domain/marketFormulas.audit.ts
 * يطبع FORMULA / EXPECTED / ACTUAL / STATUS لكل بند.
 */
import assert from 'node:assert/strict';
import {
  aggregateMarketStats,
  buildCityQuotes,
  cityChangePct,
  displayMeterOrDash,
  normalizeEntryDate,
  pricePerMeterSale,
  rentPerMeter,
  type MarketRowInput
} from './marketFormulas.ts';

type Row = { formula: string; expected: string; actual: string; pass: boolean };

const results: Row[] = [];

function check(formula: string, expected: unknown, actual: unknown, cmp?: (a: unknown, b: unknown) => boolean) {
  const pass = cmp
    ? cmp(expected, actual)
    : typeof expected === 'number' && typeof actual === 'number'
      ? Math.abs(expected - actual) < 1e-9
      : Object.is(expected, actual) || expected === actual;
  results.push({
    formula,
    expected: String(expected),
    actual: actual === null || actual === undefined ? String(actual) : String(actual),
    pass
  });
  if (!pass) {
    console.error(`FAIL: ${formula}\n  EXPECTED: ${expected}\n  ACTUAL: ${actual}`);
  }
}

const FIXTURE: MarketRowInput[] = [
  { city: 'الرياض', area: 500, sale_price: 1625000, rent_value: 50000, requests_count: 3, offers_count: 5, entry_date: '2026-01-01' },
  { city: 'الرياض', area: 400, sale_price: 1000000, rent_value: 40000, requests_count: 1, offers_count: 2, entry_date: '2026-06-01' },
  { city: 'جدة', area: 800, sale_price: 2450000, rent_value: null, requests_count: 2, offers_count: 4, entry_date: '2026-03-01' },
  { city: 'الدمام', area: 0, sale_price: 900000, rent_value: 10000, requests_count: 0, offers_count: 0, entry_date: null },
  { city: 'الرياض', area: 200, sale_price: null, rent_value: null, requests_count: 9, offers_count: 1, entry_date: '2026-06-01' }
];

// --- معادلات الصف ---
check('سعر المتر = 1625000÷500', 3250, pricePerMeterSale(1625000, 500));
check('سعر المتر = 1000000÷400', 2500, pricePerMeterSale(1000000, 400));
check('سعر المتر مساحة0 → null', null, pricePerMeterSale(900000, 0));
check('سعر المتر بيع فارغ → null', null, pricePerMeterSale(null, 200));
check('متر إيجار = 50000÷500 (سنوي كما أُدخل)', 100, rentPerMeter(50000, 500));
check('متر إيجار فارغ → null', null, rentPerMeter(null, 800));
check('متر إيجار مساحة0 → null', null, rentPerMeter(10000, 0));

const stats = aggregateMarketStats(FIXTURE);
check('عدد السجلات', 5, stats.count);
check('الطلبات مجموع', 15, stats.total_requests);
check('العروض مجموع', 12, stats.total_offers);
check('متوسط سعر المتر الكلي', (3250 + 2500 + 3062.5) / 3, stats.avg_price_per_meter, (e, a) => Math.abs(Number(e) - Number(a)) < 1e-9);
check('أدنى سعر متر', 2500, stats.min_price_per_meter);
check('أعلى سعر متر', 3250, stats.max_price_per_meter);
check('متوسط متر الإيجار', 100, stats.avg_rent_per_meter);
check('آخر تاريخ', '2026-06-01', stats.last_updated);

const quotes = buildCityQuotes(FIXTURE);
const riyadh = quotes.find((q) => q.city === 'الرياض')!;
const jeddah = quotes.find((q) => q.city === 'جدة')!;
const dammam = quotes.find((q) => q.city === 'الدمام')!;

check('الرياض متوسط متر', 2875, riyadh.avgPpm);
check('الرياض أدنى', 2500, riyadh.minPpm);
check('الرياض أعلى', 3250, riyadh.maxPpm);
check('الرياض طلبات', 13, riyadh.requests);
check('الرياض عروض', 8, riyadh.offers);
const changeExpected = ((2500 - 3250) / 3250) * 100;
check('الرياض نسبة التغير أول→آخر تاريخ', changeExpected, riyadh.changePct, (e, a) => Math.abs(Number(e) - Number(a)) < 1e-9);
check('جدة متر', 3062.5, jeddah.avgPpm);
check('جدة تغير (سجل/تاريخ واحد) → null', null, jeddah.changePct);
check('الدمام متر → null', null, dammam.avgPpm);
check('سجل واحد تغير → null', null, cityChangePct([FIXTURE[0]]));
check('تاريخ 1900 → null', null, normalizeEntryDate('1900-01-01'));
check('تاريخ فارغ → null', null, normalizeEntryDate(''));
check('عرض فارغ → —', '—', displayMeterOrDash(null, String));
check('عرض صفر مضلل → —', '—', displayMeterOrDash(0, String));

const seeded: MarketRowInput = {
  city: 'الرياض', area: 100, sale_price: 500000, entry_date: '2026-02-01',
  seed_batch_id: 'obr-demo-v1', requests_count: 1, offers_count: 1
};
check('تدريب: 500000÷100 = 5000', 5000, pricePerMeterSale(500000, 100));
const withSeed = aggregateMarketStats([...FIXTURE, seeded]);
check('تدريب داخل المجموعة: count=6', 6, withSeed.count);
check('تدريب يغيّر المتوسط', (3250 + 2500 + 3062.5 + 5000) / 4, withSeed.avg_price_per_meter, (e, a) => Math.abs(Number(e) - Number(a)) < 1e-9);

console.log('\n=== MARKET FORMULA AUDIT ===\n');
for (const r of results) {
  console.log(`FORMULA: ${r.formula}`);
  console.log(`EXPECTED: ${r.expected}`);
  console.log(`ACTUAL: ${r.actual}`);
  console.log(`STATUS: ${r.pass ? 'PASS' : 'FAIL'}`);
  console.log('---');
}

const failed = results.filter((r) => !r.pass).length;
console.log(`\nTOTAL: ${results.length}  PASS: ${results.length - failed}  FAIL: ${failed}`);
assert.equal(failed, 0, `${failed} formula checks failed`);
console.log('\nALL FORMULAS PASS');
