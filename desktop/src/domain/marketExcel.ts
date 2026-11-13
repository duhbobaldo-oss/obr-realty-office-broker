import { countryProfile, getCountry, recordCurrency } from './countryProfile';
import ExcelJS from 'exceljs';
import { writeFile } from '@tauri-apps/plugin-fs';
import { currencySymbol, t, tf } from './locale';
import {
  buildCityQuotes,
  deriveRowMeters,
  toOptionalNumber,
  type MarketRowInput
} from './marketFormulas';

export { buildCityQuotes } from './marketFormulas';
export type { CityQuote } from './marketFormulas';

/**
 * توليد ملفات Excel عبر exceljs (مكتبة مدعومة وبلا ثغرات معروفة)
 * بدل SheetJS/xlsx الذي يحمل ثغرتا HIGH معلومتين وغير مُحدَّث.
 * الاستخدام هنا تصدير فقط (كتابة بياناتنا) — لا قراءة ملفات خارجية.
 */

export async function workbookToBytes(wb: ExcelJS.Workbook): Promise<Uint8Array> {
  const buffer = await wb.xlsx.writeBuffer();
  return new Uint8Array(buffer as ArrayBuffer);
}

export function ensureXlsxPath(path: string): string {
  return /\.xlsx$/i.test(path) ? path : `${path}.xlsx`;
}

export async function saveWorkbookToPath(path: string, wb: ExcelJS.Workbook): Promise<string> {
  const finalPath = ensureXlsxPath(path);
  const bytes = await workbookToBytes(wb);
  if (bytes.length < 4 || bytes[0] !== 0x50 || bytes[1] !== 0x4b) {
    throw new Error('تعذر توليد ملف Excel صالح (تنسيق ثنائي غير مكتمل)');
  }
  await writeFile(finalPath, bytes);
  return finalPath;
}

/** رؤوس التصدير — العملة برمز ISO عبر currencySymbol() (مثل SAR) */
export const MARKET_EXCEL_HEADERS = () => {
  const cur = 'عملة السجل';
  return [
    t('xlsx.city'),
    t('xlsx.district'),
    t('xlsx.property-type'),
    t('xlsx.usage-type'),
    t('xlsx.area'),
    tf('xlsx.sale-total', { cur }),
    tf('xlsx.ppm', { cur }),
    tf('xlsx.rent', { cur }),
    tf('xlsx.rpm', { cur }),
    t('xlsx.requests'),
    t('xlsx.offers'),
    t('xlsx.date'),
    t('xlsx.source'),
    t('xlsx.notes'), 'الدولة', 'العملة'
  ] as const;
};

/** أعمدة الاستيراد — سعر المتر = إجمالي ÷ مساحة؛ قيمة الإيجار كما أُدخلت؛ العملة ISO */
export const MARKET_IMPORT_HEADERS = () => {
  const cur = currencySymbol();
  return [
    t('xlsx.city'),
    t('xlsx.district'),
    t('xlsx.property-type'),
    t('xlsx.usage-type'),
    t('xlsx.area'),
    tf('xlsx.sale-total', { cur }),
    tf('xlsx.rent', { cur }),
    t('xlsx.requests'),
    t('xlsx.offers'),
    t('xlsx.date'),
    t('xlsx.source'),
    t('xlsx.notes')
  ] as const;
};

function styleWorksheet(ws: ExcelJS.Worksheet, headerRow: number, widths: number[]): void {
  widths.forEach((w, i) => {
    ws.getColumn(i + 1).width = w;
  });
  ws.autoFilter = {
    from: { row: 1, column: 1 },
    to: { row: headerRow, column: widths.length }
  };
}

export function buildMarketTemplateWorkbook(): ExcelJS.Workbook {
  const headers = [...MARKET_IMPORT_HEADERS()];
  const example = [
    ['الرياض', 'الياسمين', 'أرض', 'سكني', 500, 1625000, 50000, 3, 5, '2026-08-01', 'سوق العقار', 'صف مثال — احذفه قبل الاستيراد إن لم ترده'],
    ['جدة', 'الشاطئ', 'فيلا', 'سكني', 800, 2450000, '', 2, 4, '2026-08-15', 'سوق العقار', '']
  ];
  const wb = new ExcelJS.Workbook();
  const ws = wb.addWorksheet('بيانات المؤشر الخاص');
  const context = wb.addWorksheet('دولة القالب');
  context.addRow(['الدولة', getCountry()]);
  context.addRow(['العملة', countryProfile(getCountry()).currency]);
  ws.addRow(headers);
  if (getCountry() === 'SA') example.forEach((row) => ws.addRow(row));
  styleWorksheet(ws, 1, headers.map((_, i) => (i < 4 ? 14 : 22)));
  return wb;
}

export function buildMarketExportWorkbook(rows: MarketRowInput[]): ExcelJS.Workbook {
  const headers = [...MARKET_EXCEL_HEADERS()];
  const body = rows.map((m) => {
    const area = toOptionalNumber(m.area);
    const sale = toOptionalNumber(m.sale_price);
    const rent = toOptionalNumber(m.rent_value);
    const meters = deriveRowMeters(m);
    return [
      m.city || '',
      m.district || '',
      (m as any).property_type || '',
      (m as any).usage_type || '',
      area,
      sale,
      meters.price_per_meter_sale != null ? Math.round(meters.price_per_meter_sale * 100) / 100 : '',
      rent,
      meters.rent_per_meter != null ? Math.round(meters.rent_per_meter * 100) / 100 : '',
      toOptionalNumber(m.requests_count) ?? 0,
      toOptionalNumber(m.offers_count) ?? 0,
      m.entry_date || '',
      (m as any).data_source || '',
      (m as any).notes || '', m.country, recordCurrency(m)
    ];
  });

  const wb = new ExcelJS.Workbook();
  const ws = wb.addWorksheet('سجلات المؤشر الخاص');
  ws.addRow(headers);
  body.forEach((row) => ws.addRow(row));
  styleWorksheet(ws, 1, headers.map(() => 18));

  const summaryHeaders = [
    'المدينة',
    'عدد السجلات',
    'متوسط سعر المتر',
    'أدنى سعر متر',
    'أعلى سعر متر',
    'متوسط سعر المتر للإيجار',
    'نسبة التغير %', 'الدولة', 'العملة'
  ];
  const quotes = [...new Set(rows.map(r => r.country))].flatMap(country => {
    if (!country) throw new Error('دولة السجل غير محددة');
    return buildCityQuotes(rows.filter(r => r.country === country)).map(q => ({ ...q, country, currency: countryProfile(country).currency }));
  });
  const summaryBody = quotes.map((q) => [
    q.city,
    q.count,
    q.avgPpm != null ? Math.round(q.avgPpm * 100) / 100 : '',
    q.minPpm != null ? Math.round(q.minPpm * 100) / 100 : '',
    q.maxPpm != null ? Math.round(q.maxPpm * 100) / 100 : '',
    q.avgRentPpm != null ? Math.round(q.avgRentPpm * 100) / 100 : '',
    q.changePct != null ? Math.round(q.changePct * 100) / 100 : '', q.country, q.currency
  ]);
  const summaryWs = wb.addWorksheet('ملخص المدن');
  summaryWs.addRow(summaryHeaders);
  summaryBody.forEach((row) => summaryWs.addRow(row));
  styleWorksheet(summaryWs, 1, summaryHeaders.map(() => 18));

  return wb;
}
