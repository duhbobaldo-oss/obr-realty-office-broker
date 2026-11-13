import { countryProfile, getCountry } from '../domain/countryProfile';
import { ApexAreaChart, ApexBarChart, useChartToneColors } from '../components/ApexChart';
import { IndicatorKpi, MarketPulseTicker, type PulseTickerItem } from '../components/IndicatorUi';
import { TrendingUp, TriangleAlert, Download, Upload, CheckCircle2, BarChart3, Pencil, Trash2, FileSpreadsheet, Building2, Ruler, ArrowDownUp, CircleDollarSign, MapPinned, List, Activity, FileSearch, CheckSquare, Square } from 'lucide-react';
import ExcelJS from 'exceljs';
import { open, save } from '@tauri-apps/plugin-dialog';
import { readFile } from '@tauri-apps/plugin-fs';
import PaginationBar from '../components/PaginationBar';
import { TABLE_PAGE_SIZE, slicePage } from '../pagination';
import { currencySymbol, formatMoneyValue, t as tLocale, tf as tfLocale } from '../domain/locale';
import {
  buildMarketExportWorkbook,
  buildMarketTemplateWorkbook,
  saveWorkbookToPath
} from '../domain/marketExcel';
import {
  displayMeterOrDash,
  normalizeEntryDate,
  pricePerMeterSale,
  rentPerMeter,
  toOptionalNumber
} from '../domain/marketFormulas';
import { noAutofill } from '../noAutofill';
import SectionGuideBanner from '../components/SectionGuide';
import { api } from '../api';
import { useLocale } from '../hooks/useLocale';
import React, { useEffect, useMemo, useState } from 'react';

type MarketForm = {
  city: string;
  district: string;
  property_type: string;
  usage_type: string;
  area: string;
  sale_price: string;
  rent_value: string;
  requests_count: string;
  offers_count: string;
  entry_date: string;
  data_source: string;
  notes: string;
};

const EMPTY_FORM: MarketForm = {
  city: '',
  district: '',
  property_type: '',
  usage_type: '',
  area: '',
  sale_price: '',
  rent_value: '',
  requests_count: '',
  offers_count: '',
  entry_date: '',
  data_source: '',
  notes: ''
};

function toForm(m: any): MarketForm {
  return {
    city: m.city || '',
    district: m.district || '',
    property_type: m.property_type || '',
    usage_type: m.usage_type || '',
    area: m.area != null ? String(m.area) : '',
    sale_price: m.sale_price != null ? String(m.sale_price) : '',
    rent_value: m.rent_value != null ? String(m.rent_value) : '',
    requests_count: m.requests_count != null ? String(m.requests_count) : '',
    offers_count: m.offers_count != null ? String(m.offers_count) : '',
    entry_date: m.entry_date || '',
    data_source: m.data_source || '',
    notes: m.notes || ''
  };
}

function derivedMeters(form: MarketForm) {
  const area = toOptionalNumber(form.area);
  const sale = toOptionalNumber(form.sale_price);
  const rent = toOptionalNumber(form.rent_value);
  return {
    price_per_meter_sale: pricePerMeterSale(sale, area),
    rent_per_meter: rentPerMeter(rent, area)
  };
}

function isRowError(er: string, rowNum: number): boolean {
  return er.includes(`السطر ${rowNum}:`) || er.includes(`Row ${rowNum}:`);
}

function rowNumFromError(er: string): number {
  const m = er.match(/^(?:السطر|Row) (\d+):/);
  return m ? Number(m[1]) : -1;
}

// تحويل صف البطاقة المجمعة من SQL إلى شكل Quote المعروض
function toQuote(r: any) {
  return {
    city: String(r.label ?? ''),
    count: Number(r.count) || 0,
    avgPpm: r.avg_price_per_meter == null ? null : Number(r.avg_price_per_meter),
    minPpm: r.min_price_per_meter == null ? null : Number(r.min_price_per_meter),
    maxPpm: r.max_price_per_meter == null ? null : Number(r.max_price_per_meter),
    avgRentPpm: r.avg_rent_per_meter == null ? null : Number(r.avg_rent_per_meter),
    requests: Number(r.total_requests) || 0,
    offers: Number(r.total_offers) || 0,
    changePct: r.change_pct == null ? null : Number(r.change_pct)
  };
}

// منتقي قابل للبحث: بدل قائمة <select> عملاقة — بحث + قائمة محدودة العرض
function SearchSelect({
  value,
  options,
  onChange,
  disabled,
  placeholder,
  emptyPlaceholder,
  ariaLabel
}: {
  value: string;
  options: string[];
  onChange: (next: string) => void;
  disabled?: boolean;
  placeholder: string;
  emptyPlaceholder: string;
  ariaLabel: string;
}) {
  const [open, setOpen] = React.useState(false);
  const [query, setQuery] = React.useState('');
  const boxRef = React.useRef<HTMLDivElement>(null);
  const { t } = useLocale();

  React.useEffect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      if (boxRef.current && !boxRef.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener('mousedown', onDoc);
    return () => document.removeEventListener('mousedown', onDoc);
  }, [open]);

  const q = query.trim();
  const filtered = q ? options.filter((o) => o.includes(q)) : options;
  const visible = filtered.slice(0, 50);

  return (
    <div className="search-select" ref={boxRef} style={{ position: 'relative' }}>
      <input
        className="search-input"
        type="text"
        role="combobox"
        aria-expanded={open}
        aria-label={ariaLabel}
        autoComplete="off"
        disabled={disabled}
        placeholder={disabled ? emptyPlaceholder : placeholder}
        value={open ? query : value}
        onFocus={() => { if (!disabled) { setOpen(true); setQuery(''); } }}
        onChange={(e) => { setQuery(e.target.value); setOpen(true); }}
        onKeyDown={(e) => {
          if (e.key === 'Escape') setOpen(false);
          if (e.key === 'Enter' && visible.length > 0) {
            onChange(visible[0]);
            setOpen(false);
          }
        }}
      />
      {open && !disabled && (
        <div
          className="search-select-list"
          role="listbox"
          style={{
            position: 'absolute',
            top: '100%',
            insetInlineStart: 0,
            zIndex: 30,
            maxHeight: 240,
            overflowY: 'auto',
            minWidth: '100%'
          }}
        >
          {value && (
            <button type="button" className="search-select-item" onClick={() => { onChange(''); setOpen(false); }}>
              {t('market.all-cities')}
            </button>
          )}
          {visible.map((o) => (
            <button
              key={o}
              type="button"
              className={`search-select-item${o === value ? ' active' : ''}`}
              onClick={() => { onChange(o); setOpen(false); }}
            >
              {o}
            </button>
          ))}
          {visible.length === 0 && (
            <div className="search-select-empty" style={{ padding: '6px 10px', opacity: 0.6 }}>
              {t('market.no-match')}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export default function Market() {
  const { t, tf, lang } = useLocale();
  const { primary: chartPrimary, accent: chartAccent } = useChartToneColors();
  const [city, setCity] = useState('');
  const [district, setDistrict] = useState('');
  const [propertyType, setPropertyType] = useState('');
  const [stats, setStats] = useState<any>(null);
  const [items, setItems] = useState<any[]>([]);
  const [disclaimer, setDisclaimer] = useState('');
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState('');
  const [statsError, setStatsError] = useState('');
  const [saving, setSaving] = useState(false);
  const saveInFlight = React.useRef(false);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [previewPage, setPreviewPage] = useState(1);
  const [showForm, setShowForm] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [uploading, setUploading] = useState(false);
  const [excelResult, setExcelResult] = useState<any>(null);
  const [uploadMsg, setUploadMsg] = useState('');
  const [fileOk, setFileOk] = useState('');
  const [savingTemplate, setSavingTemplate] = useState(false);
  const [exportingAll, setExportingAll] = useState(false);
  const [formError, setFormError] = useState('');
  const fileInputRef = React.useRef<HTMLInputElement>(null);
  const [form, setForm] = useState<MarketForm>(EMPTY_FORM);
  // مصادر ديناميكية من SQLite: مدن الفلتر، أحياء المدينة المختارة، البطاقات المجمعة، السلسلة الزمنية
  const [cities, setCities] = useState<string[]>([]);
  const [districts, setDistricts] = useState<string[]>([]);
  const [geoRows, setGeoRows] = useState<any[]>([]);
  const [geoError, setGeoError] = useState('');
  const [trendRows, setTrendRows] = useState<any[]>([]);
  // العرض الافتراضي بدون فلاتر: أحدث 10 أحياء على مستوى كل البيانات (SQL latest-first)
  const [topDistricts, setTopDistricts] = useState<any[]>([]);
  // التحديد والحذف الجماعي: none = لا تحديد، page = سجلات الصفحة الحالية (IDs)، all = كل النتائج المطابقة للفلاتر
  const [selectMode, setSelectMode] = useState<'none' | 'page' | 'all'>('none');
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [deletingSelected, setDeletingSelected] = useState(false);

  const meters = useMemo(() => derivedMeters(form), [form]);
  // سياق الحفر: بدون مدينة = بطاقات المدن؛ مع مدينة = أحياء المدينة المختارة (SQL)
  const geoMode = city ? 'district' : 'city';

  const cityQuotes = useMemo(() => geoRows.map(toQuote), [geoRows]);

  // أحدث 10 أحياء للعرض الافتراضي (بدون فلاتر) — من SQLite latest-first
  const topDistrictQuotes = useMemo(() => topDistricts.map(toQuote), [topDistricts]);

  const marketTrend = useMemo(() => {
    // السلسلة الزمنية جاهزة من SQLite (GROUP BY entry_date بحد 90 مقطعاً) — لا تجميع داخل React
    const rows = Array.isArray(trendRows) ? trendRows : [];
    const labels = rows.map((r: any) => {
      const d = String(r.entry_date || '').slice(0, 10);
      return d.length >= 10 ? d.slice(5) : d;
    });
    const points = rows.map((r: any) => {
      const ppm = Number(r.avg_price_per_meter);
      return Number.isFinite(ppm) ? ppm : 0;
    });
    return { labels, points };
  }, [trendRows]);

  const pulseItems = useMemo<PulseTickerItem[]>(() => {
    return cityQuotes.map((q) => ({
      id: q.city,
      label: q.city,
      salePerSqm: q.avgPpm,
      rentPerSqm: q.avgRentPpm,
      changePct: q.changePct,
      dealsCount: q.count,
      dealsLabel: t('market.record')
    }));
  }, [cityQuotes, lang, t]);

  const loadGeo = async () => {
    setGeoError('');
    try {
      // الحد الافتراضي: أحدث 10 مجموعات فقط من SQLite (cities أو districts حسب السياق)
      const params = new URLSearchParams({ limit: '10', mode: geoMode });
      if (city) params.set('city', city);
      if (district) params.set('district', district);
      const rows = await api(`/api/market/geo-summary?${params}`);
      setGeoRows(Array.isArray(rows) ? rows : []);
      setGeoError('');
    } catch {
      setGeoError(t('market.load-fail'));
    }
  };

  const loadTopDistricts = async () => {
    try {
      const params = new URLSearchParams({ limit: '10', mode: 'district' });
      const rows = await api(`/api/market/geo-summary?${params}`);
      setTopDistricts(Array.isArray(rows) ? rows : []);
    } catch {
      setTopDistricts([]);
    }
  };

  const loadTrend = async () => {
    try {
      const params = new URLSearchParams();
      if (city) params.set('city', city);
      if (district) params.set('district', district);
      if (propertyType) params.set('property_type', propertyType);
      const rows = await api(`/api/market/trends?${params}`);
      setTrendRows(Array.isArray(rows) ? rows : []);
    } catch {
      setTrendRows([]);
    }
  };

  const loadCities = async () => {
    try {
      const rows = await api('/api/market/filter-cities');
      setCities(Array.isArray(rows) ? rows : []);
    } catch {
      setCities([]);
    }
  };

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const purged: any = await api('/api/market/purge-audit', { method: 'POST' });
        if (!cancelled && purged?.deleted > 0) {
          setFileOk(purged.message || tf('market.purged', { n: purged.deleted }));
        }
      } catch {
        /* لا نمنع فتح الصفحة إن فشل التنظيف */
      }
      if (cancelled) return;
      loadStats();
      loadItems();
      loadGeo();
      loadTrend();
      loadCities();
      loadTopDistricts();
      api('/api/market/disclaimer', { auth: false })
        .then((d: any) => { if (!cancelled) setDisclaimer(d.disclaimer || ''); })
        .catch((err: any) => {
          if (!cancelled) setDisclaimer(t('market.disclaimer-fail'));
        });
    })();
    return () => { cancelled = true; };
  }, []);

  const loadStats = async () => {
    setStatsError('');
    try {
      const params = new URLSearchParams();
      if (city) params.set('city', city);
      if (district) params.set('district', district);
      if (propertyType) params.set('property_type', propertyType);
      const data = await api(`/api/market/stats?${params}`);
      setStats(data);
      setStatsError('');
    } catch {
      setStatsError(t('market.load-fail'));
    }
  };

  const loadItems = async () => {
    setLoading(true);
    setLoadError('');
    try {
      const params = new URLSearchParams({ page: String(page), limit: String(TABLE_PAGE_SIZE) });
      if (city) params.set('city', city);
      if (district) params.set('district', district);
      if (propertyType) params.set('property_type', propertyType);
      const data = await api(`/api/market?${params}`);
      setItems(data.items || []);
      setTotal(data.total || 0);
      setLoadError('');
    } catch {
      setLoadError(t('market.load-fail'));
    } finally {
      setLoading(false);
    }
  };

  // الأحياء تابعة للمدينة: أي حي جديد لهذه المدينة يظهر تلقائياً، وبلا مدينة القائمة فارغة
  useEffect(() => {
    setDistricts([]);
    if (!city) return;
    let cancelled = false;
    api(`/api/market/filter-districts?city=${encodeURIComponent(city)}`)
      .then((rows: any) => { if (!cancelled) setDistricts(Array.isArray(rows) ? rows : []); })
      .catch(() => { if (!cancelled) setDistricts([]); });
    return () => { cancelled = true; };
  }, [city]);

  useEffect(() => {
    loadStats();
    loadItems();
    loadGeo();
    loadTrend();
    if (!city) loadTopDistricts();
  }, [page, city, district, propertyType]);

  const handleCityChange = (next: string) => {
    setCity(next);
    setDistrict('');
  };

  const handleFilter = () => {
    if (page === 1) {
      loadStats();
      loadItems();
      loadGeo();
      loadTrend();
    } else {
      setPage(1);
    }
  };

  const clearFilters = () => {
    setCity('');
    setDistrict('');
    setPropertyType('');
    if (page === 1) {
      loadStats();
      loadItems();
      loadGeo();
      loadTrend();
    } else {
      setPage(1);
    }
  };

  const openCreate = () => {
    setEditingId(null);
    setForm(EMPTY_FORM);
    setFormError('');
    setShowForm(true);
  };

  const openEdit = (m: any) => {
    setEditingId(m.id);
    setForm(toForm(m));
    setFormError('');
    setShowForm(true);
    window.scrollTo({ top: 0, behavior: 'smooth' });
  };

  const closeForm = () => {
    setShowForm(false);
    setEditingId(null);
    setForm(EMPTY_FORM);
    setFormError('');
  };

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    if (saveInFlight.current) return;
    saveInFlight.current = true;
    setSaving(true);
    setFormError('');
    const body = {
      ...form,
      area: form.area ? Number(form.area) : undefined,
      sale_price: form.sale_price ? Number(form.sale_price) : undefined,
      rent_value: form.rent_value ? Number(form.rent_value) : undefined,
      requests_count: form.requests_count ? Number(form.requests_count) : undefined,
      offers_count: form.offers_count ? Number(form.offers_count) : undefined,
      data_source: form.data_source || undefined,
      notes: form.notes || undefined
    };
    try {
      if (editingId) {
        await api(`/api/market/${editingId}`, { method: 'PUT', body });
        setFileOk(t('market.updated'));
      } else {
        await api('/api/market', { method: 'POST', body });
        setFileOk(t('market.saved'));
      }
      closeForm();
      loadStats();
      loadItems();
      loadGeo();
      loadTrend();
    } catch (err: any) {
      setFormError(t('market.save-failed'));
    } finally {
      saveInFlight.current = false;
      setSaving(false);
    }
  };

  const handleDelete = async (m: any) => {
    const label = [m.city, m.district, m.property_type].filter(Boolean).join(' · ') || t('market.this-record');
    if (!window.confirm(tf('market.delete-confirm', { label }))) return;
    try {
      await api(`/api/market/${m.id}`, { method: 'DELETE' });
      if (editingId === m.id) closeForm();
      setFileOk(t('market.deleted'));
      loadStats();
      loadItems();
      loadGeo();
      loadTrend();
    } catch (err: any) {
      setUploadMsg(err.message || t('market.delete-failed'));
    }
  };

  // ===== التحديد والحذف الجماعي =====
  const pageIds = useMemo(() => items.map((m: any) => String(m.id)), [items]);
  const allPageSelected = selectMode === 'all' || (pageIds.length > 0 && pageIds.every((id) => selectedIds.has(id)));
  const selectedCount = selectMode === 'all' ? total : selectedIds.size;
  const hasActiveFilters = Boolean(city || district || propertyType);

  const clearSelection = () => {
    setSelectMode('none');
    setSelectedIds(new Set());
  };

  // إلغاء التحديد عند تغير الفلاتر أو الصفحة (نطاق النتائج تغيّر)
  useEffect(() => {
    setSelectMode('none');
    setSelectedIds(new Set());
  }, [city, district, propertyType, page]);

  const togglePageSelection = () => {
    if (allPageSelected) {
      // إلغاء: من وضع الكل نعود إلى لا تحديد؛ من وضع الصفحة نمسح IDs
      clearSelection();
      return;
    }
    setSelectMode('page');
    setSelectedIds(new Set(pageIds));
  };

  const toggleRecord = (id: string) => {
    setSelectedIds((prev) => {
      const next = new Set(selectMode === 'all' ? new Set(pageIds) : prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
    setSelectMode('page');
  };

  const selectAllResults = () => {
    if (selectMode === 'all') {
      clearSelection();
      return;
    }
    setSelectMode('all');
    setSelectedIds(new Set());
  };

  const handleDeleteSelected = async () => {
    if (deletingSelected || selectedCount === 0) return;
    const confirmMsg = selectMode === 'all'
      ? tf(hasActiveFilters ? 'market.delete-all-confirm' : 'market.delete-all-confirm-no-filter', { n: total })
      : tf('market.delete-selected-confirm', { n: selectedIds.size });
    if (!window.confirm(confirmMsg)) return;
    setDeletingSelected(true);
    try {
      if (selectMode === 'all') {
        // حذف جماعي داخل SQLite حسب الفلاتر الحالية — بدون تحميل IDs إلى الواجهة
        const params = new URLSearchParams();
        if (city) params.set('city', city);
        if (district) params.set('district', district);
        if (propertyType) params.set('property_type', propertyType);
        const res: any = await api(`/api/market/records?${params}`, { method: 'DELETE' });
        const deleted = Number(res?.deleted) || 0;
        setFileOk(tf('market.deleted-selected', { n: deleted }));
        // تعديل الصفحة الحالية كي لا تبقى صفحة فارغة بعد انخفاض عدد الصفحات
        const newTotal = Math.max(0, total - deleted);
        const maxPage = Math.max(1, Math.ceil(newTotal / 20));
        if (page > maxPage) setPage(maxPage);
      } else {
        const ids = Array.from(selectedIds);
        await api('/api/market/delete-many', { method: 'POST', body: { ids } });
        setFileOk(tf('market.deleted-selected', { n: ids.length }));
        const newTotal = Math.max(0, total - ids.length);
        const maxPage = Math.max(1, Math.ceil(newTotal / 20));
        if (page > maxPage) setPage(maxPage);
      }
      clearSelection();
      // تحديث المؤشرات/الرسوم/الإجماليات بعد الحذف
      loadStats();
      loadItems();
      loadGeo();
      loadTrend();
      loadCities();
      loadTopDistricts();
    } catch (err: any) {
      setUploadMsg(err.message || t('market.delete-failed'));
    } finally {
      setDeletingSelected(false);
    }
  };

  const downloadTemplate = async () => {
    setUploadMsg('');
    setFileOk('');
    setSavingTemplate(true);
    try {
      const wb = buildMarketTemplateWorkbook();
      const path = await save({
        defaultPath: 'private-market-indicator-template.xlsx',
        filters: [{ name: 'Excel', extensions: ['xlsx'] }]
      });
      if (!path) return;
      await saveWorkbookToPath(path, wb);
      setFileOk(t('market.template-saved'));
    } catch (err: any) {
      setUploadMsg(err?.message || t('market.template-failed'));
    } finally {
      setSavingTemplate(false);
    }
  };

  const exportAllRecords = async () => {
    setUploadMsg('');
    setFileOk('');
    setExportingAll(true);
    try {
      const rows = await api('/api/export/market');
      const list = Array.isArray(rows) ? rows : [];
      if (!list.length) {
        setUploadMsg(t('market.no-export'));
        return;
      }
      const wb = buildMarketExportWorkbook(list);
      const path = await save({
        defaultPath: `private-market-indicator-${new Date().toISOString().slice(0, 10)}.xlsx`,
        filters: [{ name: 'Excel', extensions: ['xlsx'] }]
      });
      if (!path) return;
      await saveWorkbookToPath(path, wb);
      setFileOk(tf('market.exported', { n: list.length }));
    } catch (err: any) {
      setUploadMsg(err?.message || t('market.export-failed'));
    } finally {
      setExportingAll(false);
    }
  };

  const applyExcelPreview = (parsed: Awaited<ReturnType<typeof parseMarketExcel>>) => {
    setExcelResult(parsed);
    setPreviewPage(1);
    setFileOk('');
    if (parsed.preview.length === 0 && parsed.errors.length === 0) {
      setUploadMsg(t('market.excel-empty'));
    }
  };

  const importFromBytes = async (bytes: Uint8Array) => {
    applyExcelPreview(await parseMarketExcel(bytes));
  };

  const pickExcelToImport = async () => {
    setUploadMsg('');
    setFileOk('');
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: 'Excel', extensions: ['xlsx', 'xls'] }]
      });
      if (!selected) return;
      const path = Array.isArray(selected) ? selected[0] : selected;
      if (!path) return;
      setUploading(true);
      const bytes = await readFile(path);
      await importFromBytes(bytes instanceof Uint8Array ? bytes : Uint8Array.from(bytes));
    } catch (err: any) {
      const msg = String(err?.message || err || '');
      if (fileInputRef.current && (msg.includes('invoke') || msg.includes('not allowed') || msg.includes('plugin'))) {
        fileInputRef.current.click();
        return;
      }
      setUploadMsg(msg || t('market.pick-failed'));
    } finally {
      setUploading(false);
    }
  };

  const handleFileUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    setUploading(true);
    setUploadMsg('');
    setFileOk('');
    try {
      const buffer = await file.arrayBuffer();
      await importFromBytes(new Uint8Array(buffer));
    } catch (err: any) {
      setUploadMsg(tf('market.read-failed', { msg: err?.message || t('market.invalid-file') }));
    } finally {
      setUploading(false);
      if (fileInputRef.current) fileInputRef.current.value = '';
    }
  };

  const confirmImport = async () => {
    if (!excelResult?.preview?.length) return;
    setUploadMsg('');
    try {
      const validRows = excelResult.preview
        .filter((row: any) =>
          !excelResult.errors.some((er: string) => isRowError(er, row._rowNum))
        )
        .map(({ _rowNum, ...rest }: any) => rest);
      if (!validRows.length) {
        setUploadMsg(t('market.no-valid-rows'));
        return;
      }
      await api('/api/market/bulk', {
        method: 'POST',
        body: { items: validRows }
      });
      setExcelResult(null);
      setFileOk(tf('market.saved-import', { n: validRows.length }));
      loadStats();
      loadItems();
      loadGeo();
      loadTrend();
      loadCities();
    } catch (err: any) {
      setUploadMsg(err.message || t('market.import-failed'));
    }
  };

  const formatMoney = (p: any, currency?: string) => displayMeterOrDash(
    p === null || p === undefined || p === '' ? null : Number(p),
    (n) => formatMoneyValue(n, 2, currency)
  );

  return (
    <div>
      <div className="card-header">
        <div className="card-title">
          <TrendingUp size={17} strokeWidth={1.8} style={{ verticalAlign: '-3px' }} /> {t('market')}{' '}
          <SectionGuideBanner id="market" />
        </div>
        <button type="button" className="btn btn-primary" onClick={openCreate}>
          {t('market.add')}
        </button>
      </div>

      {disclaimer && (
        <div className="alert-note">
          <TriangleAlert size={14} strokeWidth={1.8} style={{ verticalAlign: '-2px' }} /> {disclaimer}
        </div>
      )}

      {showForm && (
        <div className="content-card">
          <div className="card-title" style={{ marginBottom: 12 }}>
            <Pencil size={17} strokeWidth={1.8} style={{ verticalAlign: '-3px' }} />{' '}
            {editingId ? t('market.edit-title') : t('market.add-title')}
          </div>
          <form {...noAutofill} onSubmit={handleSave}>
            <div className="form-grid">
              <div className="form-group">
                <label>{t('city')}</label>
                <input className="form-input" autoComplete="off" value={form.city} onChange={(e) => setForm({ ...form, city: e.target.value })} />
              </div>
              <div className="form-group">
                <label>{t('district')}</label>
                <input className="form-input" autoComplete="off" value={form.district} onChange={(e) => setForm({ ...form, district: e.target.value })} />
              </div>
              <div className="form-group">
                <label>{t('market.property-type')}</label>
                <select className="form-input" autoComplete="off" value={form.property_type} onChange={(e) => setForm({ ...form, property_type: e.target.value })}>
                  <option value="">{t('choose')}</option>
                  <option value="أرض">{t('market.land')}</option>
                  <option value="فيلا">{t('market.villa')}</option>
                  <option value="شقة">{t('market.apartment')}</option>
                  <option value="مكتب">{t('market.office')}</option>
                  <option value="محل">{t('market.shop')}</option>
                  <option value="مستودع">{t('market.warehouse')}</option>
                  <option value="عمارة">{t('market.building')}</option>
                  <option value="أخرى">{t('market.other-type')}</option>
                </select>
              </div>
              <div className="form-group">
                <label>{t('market.usage-type')}</label>
                <input className="form-input" autoComplete="off" value={form.usage_type} onChange={(e) => setForm({ ...form, usage_type: e.target.value })} placeholder={t('market.usage-ph')} />
              </div>
              <div className="form-group">
                <label>{t('area_sqm')}</label>
                <input className="form-input" autoComplete="off" type="number" value={form.area} onChange={(e) => setForm({ ...form, area: e.target.value })} />
              </div>
              <div className="form-group">
                <label>{t('market.sale-total')}</label>
                <input className="form-input" autoComplete="off" type="number" value={form.sale_price} onChange={(e) => setForm({ ...form, sale_price: e.target.value })} />
              </div>
              <div className="form-group">
                <label>{t('market.ppm')}</label>
                <input className="form-input" autoComplete="off" readOnly value={meters.price_per_meter_sale != null ? Math.round(meters.price_per_meter_sale * 100) / 100 : ''} placeholder={t('market.ppm-hint')} />
              </div>
              <div className="form-group">
                <label>{t('market.rent-value')}</label>
                <input className="form-input" autoComplete="off" type="number" value={form.rent_value} onChange={(e) => setForm({ ...form, rent_value: e.target.value })} />
              </div>
              <div className="form-group">
                <label>{t('market.rpm')}</label>
                <input className="form-input" autoComplete="off" readOnly value={meters.rent_per_meter != null ? Math.round(meters.rent_per_meter * 100) / 100 : ''} placeholder={t('market.rpm-hint')} />
              </div>
              <div className="form-group">
                <label>{t('market.requests')}</label>
                <input className="form-input" autoComplete="off" type="number" value={form.requests_count} onChange={(e) => setForm({ ...form, requests_count: e.target.value })} />
              </div>
              <div className="form-group">
                <label>{t('market.offers')}</label>
                <input className="form-input" autoComplete="off" type="number" value={form.offers_count} onChange={(e) => setForm({ ...form, offers_count: e.target.value })} />
              </div>
              <div className="form-group">
                <label>{t('date')}</label>
                <input className="form-input" autoComplete="off" type="date" value={form.entry_date} onChange={(e) => setForm({ ...form, entry_date: e.target.value })} />
              </div>
              <div className="form-group">
                <label>{t('market.data-source')}</label>
                <input className="form-input" autoComplete="off" value={form.data_source} onChange={(e) => setForm({ ...form, data_source: e.target.value })} />
              </div>
              <div className="form-group full-row">
                <label>{t('notes')}</label>
                <input className="form-input" autoComplete="off" value={form.notes} onChange={(e) => setForm({ ...form, notes: e.target.value })} />
              </div>
            </div>
            {formError && <div className="error-alert" style={{ marginTop: 12 }}>{formError}</div>}
            <div style={{ display: 'flex', gap: 8, marginTop: 16 }}>
              <button className="btn btn-primary" type="submit" disabled={saving}>{saving ? t('saving') : editingId ? t('market.save-edit') : t('save')}</button>
              <button className="btn btn-outline" type="button" onClick={closeForm}>{t('cancel')}</button>
            </div>
          </form>
        </div>
      )}

      <div className="content-card">
        <div className="toolbar">
          <SearchSelect
            value={city}
            options={cities}
            onChange={handleCityChange}
            placeholder={t('market.all-cities')}
            emptyPlaceholder={t('market.all-cities')}
            ariaLabel={t('market.city-ph')}
          />
          <SearchSelect
            value={district}
            options={districts}
            onChange={setDistrict}
            disabled={!city}
            placeholder={t('market.all-districts')}
            emptyPlaceholder={t('market.choose-city-first')}
            ariaLabel={t('market.district-ph')}
          />
          <input className="search-input" autoComplete="off" placeholder={t('market.property-type-ph')} value={propertyType} onChange={(e) => setPropertyType(e.target.value)} />
          <button type="button" className="btn btn-primary" onClick={handleFilter}>{t('market.show-data')}</button>
          {(city || district || propertyType) && (
            <button type="button" className="btn btn-outline" onClick={clearFilters}>{t('market.clear-filters')}</button>
          )}
        </div>

        {!excelResult && (
          <div style={{ marginBottom: 16 }}>
            <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap', marginBottom: 8 }}>
              <button type="button" className="btn btn-outline" onClick={downloadTemplate} disabled={savingTemplate}>
                {savingTemplate ? t('market.saving-template') : <><Download size={15} strokeWidth={1.8} /> {t('market.excel-template')}</>}
              </button>
              <button type="button" className="btn btn-outline" onClick={exportAllRecords} disabled={exportingAll}>
                {exportingAll ? t('market.exporting') : <><FileSpreadsheet size={15} strokeWidth={1.8} /> {t('market.excel-export')}</>}
              </button>
              <input
                ref={fileInputRef}
                type="file"
                accept=".xlsx,.xls"
                style={{ display: 'none' }}
                onChange={handleFileUpload}
              />
              <button type="button" className="btn btn-outline" onClick={pickExcelToImport} disabled={uploading}>
                {uploading ? t('market.reading-file') : <><Upload size={15} strokeWidth={1.8} /> {t('market.excel-import')}</>}
              </button>
            </div>
            <p style={{ margin: 0, fontSize: 12, color: 'var(--muted-foreground)', lineHeight: 1.6 }}>
              {t('market.excel-hint')}
            </p>
          </div>
        )}

        {fileOk && !excelResult && <div className="success-alert" style={{ marginBottom: 16 }}>{fileOk}</div>}
        {uploadMsg && !excelResult && <div className="error-alert">{uploadMsg}</div>}

        {excelResult && (
          <div style={{ marginBottom: 16 }}>
            <h3 style={{ marginBottom: 12 }}>
              <FileSearch size={16} strokeWidth={1.8} style={{ verticalAlign: '-2px' }} />{' '}
              {tf('market.preview-title', { n: excelResult.valid_count || 0 })}
              {excelResult.has_errors ? ` ${tf('market.preview-errors', { n: excelResult.errors.length })}` : ''}
            </h3>
            {excelResult.errors?.length > 0 && (
              <div className="error-alert" style={{ maxHeight: 150, overflowY: 'auto' }}>
                {excelResult.errors.slice(0, 10).map((e: string, i: number) => <div key={i}>{e}</div>)}
                {excelResult.errors.length > 10 && <div>{tf('market.more-errors', { n: excelResult.errors.length - 10 })}</div>}
              </div>
            )}
            {uploadMsg && <div className="error-alert" style={{ marginBottom: 12 }}>{uploadMsg}</div>}
            <div className="campaign-board" style={{ marginBottom: 12 }}>
              {slicePage(excelResult.preview || [], previewPage).map((r: any, i: number) => (
                <article key={`${previewPage}-${i}`} className="campaign-card">
                  <div>
                    <div className="campaign-card-title">{[r.city, r.district].filter(Boolean).join(' · ') || t('market.no-location')}</div>
                    <div className="campaign-card-meta">
                      {r.property_type || t('dash')} · {r.area != null ? tf('market.area-m2', { n: r.area }) : t('dash')} · {r.entry_date || t('market.no-date')}
                    </div>
                  </div>
                  <div className="campaign-card-metrics">
                    <div className="campaign-card-metric">
                      <b className="num">{r.sale_price != null ? Number(r.sale_price).toLocaleString('en-US') : '-'}</b>
                      <span>{t('market.sale-total')}</span>
                    </div>
                    <div className="campaign-card-metric">
                      <b className="num">{r.price_per_meter_sale != null ? Number(r.price_per_meter_sale).toLocaleString('en-US') : '-'}</b>
                      <span>{t('market.ppm')}</span>
                    </div>
                    <div className="campaign-card-metric">
                      <b className="num">{r.rent_value != null ? Number(r.rent_value).toLocaleString('en-US') : t('dash')}</b>
                      <span>{t('market.rent-value')}</span>
                    </div>
                    <div className="campaign-card-metric">
                      <b className="num">{r.rent_per_meter != null ? Number(r.rent_per_meter).toLocaleString('en-US') : '-'}</b>
                      <span>{t('market.rpm')}</span>
                    </div>
                  </div>
                </article>
              ))}
            </div>
            <PaginationBar
              total={(excelResult.preview || []).length}
              page={previewPage}
              onPage={setPreviewPage}
              label={t('market.page-row')}
            />
            <div style={{ display: 'flex', gap: 8, marginTop: 12 }}>
              <button type="button" className="btn btn-success" onClick={confirmImport} disabled={!excelResult.valid_count}>
                <CheckCircle2 size={15} strokeWidth={1.8} /> {t('market.confirm-import')}
              </button>
              <button type="button" className="btn btn-outline" onClick={() => { setExcelResult(null); setUploadMsg(''); setFileOk(''); setPreviewPage(1); }}>
                {t('cancel')}
              </button>
            </div>
          </div>
        )}

        {stats && (
          <section className="content-card market-fin-panel market-fin-summary">
            <header className="market-fin-head">
              <div className="card-title">
                <BarChart3 size={17} strokeWidth={1.8} aria-hidden />
                <span>{t('market.summary')}</span>
                <span className="market-fin-summary-note">{t('market.summary-sub')} — {countryProfile(getCountry()).nameAr}</span>
              </div>
              <span className="market-fin-chip">{currencySymbol()} / م²</span>
            </header>
            <div className="ind-kpi-grid market-fin-summary-grid">
              <IndicatorKpi
                label={t('market.count')}
                value={String(stats.count || 0)}
                icon={Building2}
                tone="primary"
              />
              <IndicatorKpi
                label={t('market.avg-ppm')}
                value={formatMoney(stats.avg_price_per_meter, stats.currency)}
                icon={Ruler}
                tone="gold"
                delta={
                  marketTrend.points.length >= 2 && marketTrend.points[0] > 0
                    ? ((marketTrend.points[marketTrend.points.length - 1] - marketTrend.points[0]) / marketTrend.points[0]) * 100
                    : null
                }
              />
              <IndicatorKpi
                label={t('market.min-ppm')}
                value={formatMoney(stats.min_price_per_meter, stats.currency)}
                icon={ArrowDownUp}
              />
              <IndicatorKpi
                label={t('market.max-ppm')}
                value={formatMoney(stats.max_price_per_meter, stats.currency)}
                icon={ArrowDownUp}
                tone="primary"
              />
              <IndicatorKpi
                label={t('market.avg-rpm')}
                value={formatMoney(stats.avg_rent_per_meter, stats.currency)}
                icon={CircleDollarSign}
                tone="gold"
              />
              <IndicatorKpi
                label={t('market.requests-kpi')}
                value={String(stats.total_requests || 0)}
                hint={tf('market.offers-hint', { n: stats.total_offers || 0 })}
              />
              <IndicatorKpi
                label={t('market.offers-kpi')}
                value={String(stats.total_offers || 0)}
                hint={tf('market.last-update-hint', { date: stats.last_updated || t('dash') })}
              />
              <IndicatorKpi
                label={geoMode === 'district' ? t('market.districts-kpi') : t('market.cities')}
                value={String(cityQuotes.length)}
                tone="primary"
              />
            </div>
          </section>
        )}
      </div>

      {pulseItems.length > 0 && (
        <MarketPulseTicker items={pulseItems} currency={currencySymbol()} ariaLabel={t('market.pulse-aria')} />
      )}

      {cityQuotes.length > 0 && (
        <div className="content-card market-fin-panel market-fin-board">
          <div className="card-header market-fin-head">
            <div className="card-title">
              <FileSearch size={17} strokeWidth={1.8} aria-hidden /> {t('market.board-title')}
              <SectionGuideBanner
                id="market-board"
                guide={{
                  id: 'market-board',
                  title: t('market.board-title'),
                  summary: '',
                  steps: [],
                  blocks: [
                    { title: t('market.change-heading'), body: t('market.change-body') },
                    { title: t('market.rent-heading'), body: t('market.rent-body') }
                  ]
                }}
              />
            </div>
            <span className="market-fin-chip">{currencySymbol()} / م²</span>
          </div>
          <div className="market-fin-frame market-fin-board-frame">
            <div className="market-quote-table-wrap">
              <table className="market-quote-table market-fin-quote-table">
                <thead>
                  <tr>
                    <th>{geoMode === 'district' ? t('market.col-geo') : t('market.col-city')}</th>
                    <th>{t('market.col-avg')}</th>
                    <th>{t('market.col-min')}</th>
                    <th>{t('market.col-max')}</th>
                    <th>{t('market.col-rent')}</th>
                    <th>{t('market.col-change')}</th>
                    <th>{t('market.col-count')}</th>
                    <th>{t('market.col-req')}</th>
                    <th>{t('market.col-off')}</th>
                  </tr>
                </thead>
                <tbody>
                  {cityQuotes.map((q) => (
                    <tr key={`row-${q.city}`}>
                      <td className="market-quote-symbol">{q.city}</td>
                      <td className="num">{q.avgPpm != null ? formatMoney(q.avgPpm) : t('dash')}</td>
                      <td className="num market-quote-low">{q.minPpm != null ? formatMoney(q.minPpm) : t('dash')}</td>
                      <td className="num market-quote-high">{q.maxPpm != null ? formatMoney(q.maxPpm) : t('dash')}</td>
                      <td className="num">{q.avgRentPpm != null ? formatMoney(q.avgRentPpm) : t('dash')}</td>
                      <td className="num">
                        {q.changePct == null
                          ? t('dash')
                          : (
                            <span className={q.changePct >= 0 ? 'pulse-chip-up' : 'pulse-chip-down'}>
                              {q.changePct >= 0 ? '+' : ''}{q.changePct.toLocaleString('en-US', { maximumFractionDigits: 1 })}%
                            </span>
                          )}
                      </td>
                      <td className="num">{q.count}</td>
                      <td className="num">{q.requests}</td>
                      <td className="num">{q.offers}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      )}

      {!city && topDistrictQuotes.length > 0 && (
        <div className="content-card market-fin-panel market-fin-board">
          <div className="card-header market-fin-head">
            <div className="card-title">
              <MapPinned size={17} strokeWidth={1.8} aria-hidden /> {t('market.top-districts-title')}
            </div>
            <span className="market-fin-chip">{currencySymbol()} / م²</span>
          </div>
          <div className="market-fin-frame market-fin-board-frame">
            <div className="market-quote-table-wrap">
              <table className="market-quote-table market-fin-quote-table">
                <thead>
                  <tr>
                    <th>{t('market.col-geo')}</th>
                    <th>{t('market.col-avg')}</th>
                    <th>{t('market.col-min')}</th>
                    <th>{t('market.col-max')}</th>
                    <th>{t('market.col-rent')}</th>
                    <th>{t('market.col-change')}</th>
                    <th>{t('market.col-count')}</th>
                    <th>{t('market.col-req')}</th>
                    <th>{t('market.col-off')}</th>
                  </tr>
                </thead>
                <tbody>
                  {topDistrictQuotes.map((q) => (
                    <tr key={`topd-${q.city}`}>
                      <td className="market-quote-symbol">{q.city}</td>
                      <td className="num">{q.avgPpm != null ? formatMoney(q.avgPpm) : t('dash')}</td>
                      <td className="num market-quote-low">{q.minPpm != null ? formatMoney(q.minPpm) : t('dash')}</td>
                      <td className="num market-quote-high">{q.maxPpm != null ? formatMoney(q.maxPpm) : t('dash')}</td>
                      <td className="num">{q.avgRentPpm != null ? formatMoney(q.avgRentPpm) : t('dash')}</td>
                      <td className="num">
                        {q.changePct == null
                          ? t('dash')
                          : (
                            <span className={q.changePct >= 0 ? 'pulse-chip-up' : 'pulse-chip-down'}>
                              {q.changePct >= 0 ? '+' : ''}{q.changePct.toLocaleString('en-US', { maximumFractionDigits: 1 })}%
                            </span>
                          )}
                      </td>
                      <td className="num">{q.count}</td>
                      <td className="num">{q.requests}</td>
                      <td className="num">{q.offers}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      )}

      {cityQuotes.length > 0 && (
        <div className="charts-row market-fin-charts">
          <div className="content-card market-fin-panel">
            <div className="card-header market-fin-head">
              <div className="card-title">
                <MapPinned size={17} strokeWidth={1.8} aria-hidden /> {t('market.chart-city')}
              </div>
              <span className="market-fin-chip">{currencySymbol()} / م²</span>
            </div>
            <div className="market-fin-frame">
              <ApexBarChart
                height={300}
                horizontal
                categories={cityQuotes.filter((q) => q.avgPpm != null).slice(0, 8).map((q) => q.city)}
                series={[{
                  name: t('market.ppm'),
                  color: chartAccent,
                  data: cityQuotes.filter((q) => q.avgPpm != null).slice(0, 8).map((q) => Math.round(q.avgPpm || 0))
                }]}
                valueFormatter={(v) => `${Math.round(v).toLocaleString('en-US')} ${currencySymbol()}`}
              />
            </div>
          </div>

          <div className="content-card market-fin-panel">
            <div className="card-header market-fin-head">
              <div className="card-title">
                <BarChart3 size={17} strokeWidth={1.8} aria-hidden /> {t('market.chart-count')}
              </div>
              <span className="market-fin-chip">
                {cityQuotes.slice(0, 8).reduce((sum, q) => sum + (q.count || 0), 0).toLocaleString('en-US')} {t('market.col-count')}
              </span>
            </div>
            <div className="market-fin-frame">
              <ApexBarChart
                height={300}
                horizontal
                integerAxis
                categories={cityQuotes.slice(0, 8).map((q) => q.city)}
                series={[{
                  name: t('market.col-count'),
                  color: chartPrimary,
                  data: cityQuotes.slice(0, 8).map((q) => q.count)
                }]}
                valueFormatter={(v) => `${Math.round(v).toLocaleString('en-US')} ${t('market.col-count')}`}
              />
            </div>
          </div>

          {marketTrend.points.length >= 2 && (
            <div className="content-card market-fin-panel">
              <div className="card-header market-fin-head">
                <div className="card-title">
                  <Activity size={17} strokeWidth={1.8} aria-hidden /> {t('market.chart-trend')}
                </div>
                <span className="market-fin-chip">{currencySymbol()} / م²</span>
              </div>
              <div className="market-fin-frame">
                <ApexAreaChart
                  height={300}
                  categories={marketTrend.labels}
                  series={[{ name: t('market.chart-avg-series'), color: chartAccent, data: marketTrend.points.map((p) => Math.round(p)) }]}
                  valueFormatter={(v) => `${Math.round(v).toLocaleString('en-US')} ${currencySymbol()}`}
                />
              </div>
            </div>
          )}
        </div>
      )}

      <section className="content-card market-fin-panel market-fin-records">
        <header className="market-fin-head">
          <div className="card-title">
            <List size={17} strokeWidth={1.8} aria-hidden />
            <span>{t('market.records')}</span>
          </div>
          <div className="market-fin-records-actions">
            {items.length > 0 && (
              <>
                <button
                  type="button"
                  className={`btn ${allPageSelected ? 'btn-primary' : 'btn-outline'}`}
                  onClick={togglePageSelection}
                  aria-pressed={allPageSelected}
                  title={t('market.select-page')}
                >
                  {allPageSelected ? <CheckSquare size={15} strokeWidth={1.8} /> : <Square size={15} strokeWidth={1.8} />}
                  {t('market.select-page')}
                </button>
                <button
                  type="button"
                  className={`btn ${selectMode === 'all' ? 'btn-primary' : 'btn-outline'}`}
                  onClick={selectAllResults}
                  aria-pressed={selectMode === 'all'}
                >
                  {t('market.select-all-results')}
                </button>
              </>
            )}
            {selectedCount > 0 && (
              <>
                <span className="market-fin-chip" role="status">{tf('market.selected-count', { n: selectedCount })}</span>
                <button type="button" className="btn btn-danger" onClick={handleDeleteSelected} disabled={deletingSelected}>
                  <Trash2 size={15} strokeWidth={1.8} /> {t('market.delete-selected')}
                </button>
              </>
            )}
            <span className="market-fin-chip">{total.toLocaleString('en-US')} {t('market.col-count')}</span>
            <button type="button" className="btn btn-outline" onClick={exportAllRecords} disabled={exportingAll || total === 0}>
              {exportingAll ? t('market.exporting') : <><FileSpreadsheet size={15} strokeWidth={1.8} /> {t('market.export-all')}</>}
            </button>
          </div>
        </header>
        <div className="market-fin-records-body">
          {loadError && <div className="error-alert" role="alert">{loadError} <button type="button" className="btn btn-outline" onClick={loadItems}>{t('retry')}</button></div>}
          {statsError && <div className="error-alert" role="alert">{statsError} <button type="button" className="btn btn-outline" onClick={loadStats}>{t('retry')}</button></div>}
          {geoError && <div className="error-alert" role="alert">{geoError} <button type="button" className="btn btn-outline" onClick={loadGeo}>{t('retry')}</button></div>}
          {loading ? (
            <div>{t('loading')}</div>
          ) : loadError ? null : items.length === 0 ? (
            <div className="empty-state">
              <div className="icon"><BarChart3 size={48} strokeWidth={1.4} /></div>
              <h3>{t('empty')}</h3>
              <p>{t('market.empty-hint')}</p>
            </div>
          ) : (
            <>
              <div className="campaign-board market-fin-records-board">
                {items.map((m: any) => {
                  const recordId = String(m.id);
                  const recordChecked = selectMode === 'all' || selectedIds.has(recordId);
                  return (
                  <article key={m.id} className={`campaign-card${recordChecked ? ' record-selected' : ''}`}>
                    <label className="record-select" aria-label={t('market.select-record')}>
                      <input
                        type="checkbox"
                        checked={recordChecked}
                        onChange={() => toggleRecord(recordId)}
                      />
                    </label>
                    <div>
                      <div className="campaign-card-title">
                        {[m.city, m.district].filter(Boolean).join(' · ') || t('market.no-location')}
                      </div>
                      <div className="campaign-card-meta">
                        {m.property_type || t('dash')}
                        {m.usage_type ? ` · ${m.usage_type}` : ''}
                        {' · '}
                        {m.area != null ? tf('market.area-m2', { n: m.area }) : t('dash')}
                        {' · '}
                        {m.entry_date || t('market.no-date')}
                      </div>
                      <div className="campaign-card-meta">
                        {tf('market.req-off-meta', { req: m.requests_count || 0, off: m.offers_count || 0 })}
                        {m.data_source ? ` · ${m.data_source}` : ''}
                      </div>
                    </div>
                    <div className="campaign-card-metrics">
                      <div className="campaign-card-metric">
                        <b className="num">{formatMoney(m.sale_price, m.currency)}</b>
                        <span>{t('market.sale-total')}</span>
                      </div>
                      <div className="campaign-card-metric">
                        <b className="num">{formatMoney(m.price_per_meter_sale, m.currency)}</b>
                        <span>{t('market.ppm')}</span>
                      </div>
                      <div className="campaign-card-metric">
                        <b className="num">{formatMoney(m.rent_value, m.currency)}</b>
                        <span>{t('market.rent-value')}</span>
                      </div>
                      <div className="campaign-card-metric">
                        <b className="num">{formatMoney(m.rent_per_meter, m.currency)}</b>
                        <span>{t('market.rpm')}</span>
                      </div>
                    </div>
                    <div className="campaign-card-actions">
                      <button type="button" className="btn btn-outline" onClick={() => openEdit(m)} aria-label={t('market.edit-aria')}>
                        <Pencil size={14} strokeWidth={1.8} />
                      </button>
                      <button type="button" className="btn btn-outline" onClick={() => handleDelete(m)} aria-label={t('market.delete-aria')}>
                        <Trash2 size={14} strokeWidth={1.8} />
                      </button>
                    </div>
                  </article>
                  );
                })}
              </div>
              <PaginationBar total={total} page={page} onPage={setPage} label={t('market.record')} />
            </>
          )}
        </div>
      </section>
    </div>
  );
}

function unwrapCell(v: any): any {
  if (v === null || v === undefined) return '';
  if (v instanceof Date) return v;
  if (typeof v === 'object') {
    // richText: [{ text }, ...]
    if (Array.isArray(v) && v.length && v[0] && typeof v[0] === 'object' && 'text' in v[0]) {
      return v.map((p: any) => p.text).join('');
    }
    if ('result' in v) return v.result ?? '';
    if ('text' in v) return v.text ?? '';
    if ('error' in v) return '';
  }
  return v;
}

/** serial إكسل → YYYY-MM-DD (يُقرأ بأجزاء التوقيت المحلي — منطقة UTC+3 بلا DST) */
function excelSerialToYmd(serial: number): string | null {
  if (!Number.isFinite(serial) || serial < 1) return null;
  const d = new Date(Math.round((serial - 25569) * 86400 * 1000));
  if (isNaN(d.getTime())) return null;
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

async function parseMarketExcel(bytes: Uint8Array) {
  const wb = new ExcelJS.Workbook();
  try {
    await wb.xlsx.load(bytes as any);
  } catch {
    return { total_rows: 0, errors: [tLocale('market.excel-no-data')], has_errors: true, valid_count: 0, preview: [] as any[] };
  }
  const templateContext = wb.getWorksheet('دولة القالب');
  if (templateContext && (templateContext.getCell('B1').value !== getCountry() || templateContext.getCell('B2').value !== countryProfile(getCountry()).currency)) {
    throw new Error('دولة القالب تختلف عن الإعدادات؛ اختر دولة القالب قبل الاستيراد.');
  }
  const ws = wb.worksheets[0];
  if (!ws) {
    return { total_rows: 0, errors: [tLocale('market.excel-no-sheet')], has_errors: true, valid_count: 0, preview: [] as any[] };
  }
  // eachRow بترتيب الأرقام؛ values مفهرسة من 1 → تُحول إلى 0-based مثل sheet_to_json header:1
  const header = ((ws.getRow(1).values as any[]) || []).slice(1).map(String);
  const countryColumn = header.indexOf('الدولة');
  const currencyColumn = header.indexOf('العملة');
  const isExport = countryColumn === 14;
  const rawRows: any[][] = [];
  ws.eachRow({ includeEmpty: false }, (row, rowNumber) => {
    if (rowNumber === 1) return; // صف الرؤوس
    const vals = [...((row.values as any[]) ?? [])];
    vals.shift();
    rawRows.push(vals.map(unwrapCell));
  });

  if (rawRows.length < 1) {
    return { total_rows: 0, errors: [tLocale('market.excel-no-data')], has_errors: true, valid_count: 0, preview: [] as any[] };
  }

  const errors: string[] = [];
  const preview: any[] = [];

  for (let i = 0; i < rawRows.length; i++) {
    const rowNum = i + 2; // رقم الصف الفعلي في الملف (بعد الرؤوس)
    const r = rawRows[i];
    const cityV = r[0];
    const districtV = r[1];
    const propType = r[2];
    const usage = r[3];
    const area = r[4];
    const salePrice = r[5];
    const rent = r[isExport ? 7 : 6];
    const reqCount = r[isExport ? 9 : 7];
    const offCount = r[isExport ? 10 : 8];
    const date = r[isExport ? 11 : 9];
    const source = r[isExport ? 12 : 10];
    const notes = r[isExport ? 13 : 11];
    if ((countryColumn >= 0 && r[countryColumn] && r[countryColumn] !== getCountry()) ||
        (currencyColumn >= 0 && r[currencyColumn] && r[currencyColumn] !== countryProfile(getCountry()).currency)) {
      throw new Error(`عملة أو دولة الصف ${rowNum} تختلف عن الإعدادات؛ اختر دولة البيانات قبل الاستيراد. لا يجري تحويل عملات.`);
    }

    if (!cityV && !districtV && !propType && !area && !salePrice) {
      continue;
    }

    let areaNum = toNum(area);
    let priceNum = toNum(salePrice);
    let rentNum = toNum(rent);
    if (area && areaNum === null) { errors.push(tfLocale('market.row-err-area', { n: rowNum })); areaNum = null; }
    if (salePrice && priceNum === null) { errors.push(tfLocale('market.row-err-sale', { n: rowNum })); priceNum = null; }
    if (rent && rentNum === null) { errors.push(tfLocale('market.row-err-rent', { n: rowNum })); rentNum = null; }

    preview.push({
      city: cityV || null,
      district: districtV || null,
      property_type: propType || null,
      usage_type: usage || null,
      area: areaNum,
      sale_price: priceNum,
      price_per_meter_sale: areaNum && priceNum ? (priceNum / areaNum) : null,
      rent_value: rentNum,
      rent_per_meter: areaNum && rentNum ? (rentNum / areaNum) : null,
      requests_count: toNum(reqCount) || 0,
      offers_count: toNum(offCount) || 0,
      entry_date: excelDateToIso(date),
      data_source: source || 'استيراد Excel',
      notes: notes || null,
      _rowNum: rowNum
    });
  }

  const invalidRows = new Set(
    errors.map((er) => rowNumFromError(er))
  );

  return {
    total_rows: preview.length,
    errors,
    has_errors: errors.length > 0,
    valid_count: preview.filter((row) => !invalidRows.has(row._rowNum)).length,
    preview
  };
}

function excelDateToIso(value: any): string | null {
  if (value === null || value === undefined || value === '') return null;
  if (value instanceof Date) {
    if (isNaN(value.getTime())) return null;
    const mm = String(value.getMonth() + 1).padStart(2, '0');
    const dd = String(value.getDate()).padStart(2, '0');
    return normalizeEntryDate(`${value.getFullYear()}-${mm}-${dd}`);
  }
  if (typeof value === 'number' && Number.isFinite(value)) {
    if (value < 1) return null; // يرفض serial 0 → 1899/1900
    const ymd = excelSerialToYmd(value);
    return ymd ? normalizeEntryDate(ymd) : null;
  }
  return normalizeEntryDate(value);
}

function toNum(v: any): number | null {
  if (v === null || v === undefined || v === '') return null;
  const n = Number(v);
  return isNaN(n) ? null : n;
}
