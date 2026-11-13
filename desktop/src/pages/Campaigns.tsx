import { countryPlatforms, countryProfile, getCountry, recordOfficeSettings } from '../domain/countryProfile';
import React, { useEffect, useMemo, useState } from 'react';
import { Megaphone, Plus, Pencil, Trash2, TriangleAlert, Filter, Share2, BarChart3, CircleDollarSign, ScrollText, Link2, Copy, ExternalLink, PauseCircle, RefreshCw, Eye, Printer } from 'lucide-react';
import { api } from '../api';
import { invoke } from '@tauri-apps/api/core';
import { DonutChart, FunnelChart } from '../components/Charts';
import PaginationBar from '../components/PaginationBar';
import { TABLE_PAGE_SIZE } from '../pagination';
import { noAutofill } from '../noAutofill';
import {
  CAMPAIGN_PLATFORMS,
  campaignRate,
  campaignTotalViews,
  formatCount,
  formatDateStamp,
  formatMoneyAmount,
  formatRate,
  platformLabel,
  destinationHostLabel,
  trackingDisplayName
} from '../campaigns';
import { clientConversionStagesTitle } from '../domain/labels';
import SectionGuideBanner from '../components/SectionGuide';
import { useLocale } from '../hooks/useLocale';
import { currencySymbol, t as tLocale, tf as tfLocale } from '../domain/locale';

type CampaignFormState = {
  name: string;
  property_id: string;
  platform: string;
  ad_link: string;
  start_date: string;
  end_date: string;
  views: string;
  inquiries: string;
  viewings: string;
  deals: string;
  cost: string;
  notes: string;
};

const EMPTY_FORM: CampaignFormState = {
  name: '',
  property_id: '',
  platform: '',
  ad_link: '',
  start_date: '',
  end_date: '',
  views: '0',
  inquiries: '0',
  viewings: '0',
  deals: '0',
  cost: '0',
  notes: ''
};

function toForm(item: any): CampaignFormState {
  return {
    name: item.name || '',
    property_id: item.property_id || '',
    platform: item.platform || '',
    ad_link: item.ad_link || '',
    start_date: (item.start_date || '').slice(0, 10),
    end_date: (item.end_date || '').slice(0, 10),
    views: String(item.views ?? 0),
    inquiries: String(item.inquiries ?? 0),
    viewings: String(item.viewings ?? 0),
    deals: String(item.deals ?? 0),
    cost: String(item.cost ?? 0),
    notes: item.notes || ''
  };
}

function isHttpsAdLink(value: string): boolean {
  return value.trim().toLowerCase().startsWith('https://');
}

function CampaignReportBody({ item, busy = false }: { item: any; busy?: boolean }) {
  const { t } = useLocale();
  const totalViews = campaignTotalViews(item.views, item.tracked_clicks);
  const steps = [
    { key: 'total-views', label: t('camp.total-views'), value: totalViews, rate: null as string | null },
    { key: 'inquiries', label: t('camp.funnel.inquiries'), value: Number(item.inquiries || 0), rate: campaignRate(item.inquiries, totalViews) },
    { key: 'viewings', label: t('camp.funnel.viewings'), value: Number(item.viewings || 0), rate: formatRate(item.viewing_conversion) },
    { key: 'deals', label: t('camp.funnel.deals'), value: Number(item.deals || 0), rate: formatRate(item.deal_conversion) }
  ];

  return (
    <div className="campaign-report-body">
      <div className="campaign-preview-head">
        <h3>{item.name}</h3>
        <p>
          {platformLabel(item.platform)}
          {' · '}
          {item.property_name || t('no_property')}
        </p>
        <p className="campaign-preview-period">
          {t('camp.preview-period')}: {formatDateStamp(item.start_date)} — {formatDateStamp(item.end_date)}
          {busy ? ` · ${t('loading')}` : ''}
        </p>
      </div>

      <section className="campaign-preview-section">
        <h4>{t('camp.preview-workflow')}</h4>
        <ol className="campaign-workflow">
          {steps.map((step, idx, arr) => (
            <li key={step.key} className={`campaign-workflow-step${step.value > 0 ? ' is-active' : ''}`}>
              <div className="campaign-workflow-index">{idx + 1}</div>
              <div className="campaign-workflow-body">
                <div className="campaign-workflow-label">{step.label}</div>
                <div className="campaign-workflow-value num">{formatCount(step.value)}</div>
                {idx > 0 ? (
                  <div className="campaign-workflow-rate">
                    {t('camp.preview-step-rate')}: {step.rate}
                  </div>
                ) : null}
              </div>
              {idx < arr.length - 1 ? <div className="campaign-workflow-arrow" aria-hidden="true" /> : null}
            </li>
          ))}
        </ol>
      </section>

      <section className="campaign-preview-section">
        <h4>{t('camp.preview-costs')}</h4>
        <div className="campaign-preview-kpis">
          <div><span>{t('camp.kpi.cost')}</span><b className="num">{formatMoneyAmount(item.cost, item.currency)}</b></div>
          <div><span>{t('camp.kpi.cpi')}</span><b className="num">{formatMoneyAmount(item.cost_per_inquiry, item.currency)}</b></div>
          <div><span>{t('camp.kpi.cpv')}</span><b className="num">{formatMoneyAmount(item.cost_per_viewing, item.currency)}</b></div>
          <div><span>{t('camp.kpi.cpd')}</span><b className="num">{formatMoneyAmount(item.cost_per_deal, item.currency)}</b></div>
        </div>
      </section>

      <section className="campaign-preview-section">
        <h4>{t('camp.preview-tracking')}</h4>
        <div className="campaign-preview-kpis">
          <div><span>{t('camp.total-views')}</span><b className="num">{formatCount(totalViews)}</b></div>
          <div><span>{t('camp.manual-views')}</span><b className="num">{formatCount(item.views)}</b></div>
          <div><span>{t('camp.obr-clicks')}</span><b className="num">{formatCount(item.tracked_clicks)}</b></div>
          <div><span>{t('camp.tracking-unique')}</span><b className="num">{formatCount(item.unique_clicks)}</b></div>
          <div><span>{t('camp.tracking-last')}</span><b className="num">{formatDateStamp(item.last_click_at)}</b></div>
        </div>
        {!item.tracking_enabled ? (
          <p className="text-secondary">{t('camp.preview-no-tracking')}</p>
        ) : null}
        {item.ad_link ? (
          <div className="campaign-url-row"><span>{t('camp.ad-link')}</span><a className="campaign-url" dir="ltr" href={item.ad_link}>{item.ad_link}</a></div>
        ) : null}
        {item.tracking_url ? (
          <div className="campaign-url-row"><span>{t('camp.tracking-title')}</span><a className="campaign-url" dir="ltr" href={item.tracking_url}>{item.tracking_url}</a></div>
        ) : null}
      </section>

      <section className="campaign-preview-section">
        <h4>{t('camp.preview-notes')}</h4>
        <p className="campaign-preview-notes">
          {(item.notes || '').trim() || t('camp.preview-empty-notes')}
        </p>
      </section>
    </div>
  );
}

/** طباعة سجل الحملة: جداول منسّقة بنفس بيانات المعاينة */
function CampaignPrintTables({ item, identity }: { item: any; identity?: any }) {
  identity = recordOfficeSettings(identity, item);
  const { t } = useLocale();
  const totalViews = campaignTotalViews(item.views, item.tracked_clicks);
  const funnelRows = [
    { stage: t('camp.total-views'), count: formatCount(totalViews), rate: '' },
    { stage: t('camp.funnel.inquiries'), count: formatCount(item.inquiries), rate: campaignRate(item.inquiries, totalViews) },
    { stage: t('camp.funnel.viewings'), count: formatCount(item.viewings), rate: campaignRate(item.viewings, item.inquiries) },
    { stage: t('camp.funnel.deals'), count: formatCount(item.deals), rate: campaignRate(item.deals, item.viewings) }
  ];
  const costRows = [
    { label: t('camp.kpi.cost'), value: formatMoneyAmount(item.cost, item.currency) },
    { label: t('camp.kpi.cpi'), value: formatMoneyAmount(item.cost_per_inquiry, item.currency) },
    { label: t('camp.kpi.cpv'), value: formatMoneyAmount(item.cost_per_viewing, item.currency) },
    { label: t('camp.kpi.cpd'), value: formatMoneyAmount(item.cost_per_deal, item.currency) }
  ];
  // رابط التتبع تشغيلي للمشاركة فقط — لا يُطبع في التقرير.
  const trackingRows = [
    { label: t('camp.tracking-clicks'), value: formatCount(item.tracked_clicks) },
    { label: t('camp.tracking-unique'), value: formatCount(item.unique_clicks) },
    { label: t('camp.tracking-last'), value: formatDateStamp(item.last_click_at) },
    { label: t('camp.tracking-status'), value: item.tracking_enabled ? t('camp.tracking-on') : t('camp.tracking-off') }
  ];

  return (
    <div className="campaign-print-tables">
      <header className="camp-print-header">
        {String(identity?.office_name || '').trim() ? (
          <p className="camp-print-office">{String(identity.office_name).trim()}</p>
        ) : null}
        {String(identity?.fal_license_number || '').trim() ? (
          <p className="camp-print-office-meta">{`الترخيص / السجل التجاري: ${String(identity.fal_license_number).trim()}`}</p>
        ) : null}
        <h1>{t('camp.print-title')}</h1>
        <p className="camp-print-identity">
          <strong>{item.name}</strong>
          <span>
            {platformLabel(item.platform)}
            {' · '}
            {item.property_name || t('no_property')}
          </span>
          <span>
            {t('camp.preview-period')}: {formatDateStamp(item.start_date)} — {formatDateStamp(item.end_date)}
          </span>
        </p>
      </header>

      <section className="camp-print-section">
        <h2>{t('camp.preview-workflow')}</h2>
        <table className="camp-print-table">
          <thead>
            <tr>
              <th scope="col">{t('camp.col.stage')}</th>
              <th scope="col">{t('camp.col.count')}</th>
              <th scope="col">{t('camp.preview-step-rate')}</th>
            </tr>
          </thead>
          <tbody>
            {funnelRows.map((row) => (
              <tr key={row.stage}>
                <td>{row.stage}</td>
                <td className="num">{row.count}</td>
                <td className="num">{row.rate}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>

      <section className="camp-print-section">
        <h2>{t('camp.preview-costs')}</h2>
        <table className="camp-print-table">
          <thead>
            <tr>
              <th scope="col">{t('camp.col.metric')}</th>
              <th scope="col">{t('camp.col.value')}</th>
            </tr>
          </thead>
          <tbody>
            {costRows.map((row) => (
              <tr key={row.label}>
                <td>{row.label}</td>
                <td className="num">{row.value}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>

      <section className="camp-print-section">
        <h2>{t('camp.preview-tracking')}</h2>
        <table className="camp-print-table">
          <thead>
            <tr>
              <th scope="col">{t('camp.col.metric')}</th>
              <th scope="col">{t('camp.col.value')}</th>
            </tr>
          </thead>
          <tbody>
            {trackingRows.map((row) => (
              <tr key={row.label}>
                <td>{row.label}</td>
                <td className={row.label === t('camp.tracking-status') ? undefined : 'num'}>{row.value}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>

      <section className="camp-print-section">
        <h2>{t('camp.preview-notes')}</h2>
        <p className="campaign-preview-notes">
          {(item.notes || '').trim() || t('camp.preview-empty-notes')}
        </p>
      </section>
    </div>
  );
}

export default function Campaigns() {
  const { t, tf, lang } = useLocale();
  const [items, setItems] = useState<any[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [stats, setStats] = useState<any>(null);
  const [properties, setProperties] = useState<any[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [formError, setFormError] = useState('');
  const [saving, setSaving] = useState(false);
  const [showForm, setShowForm] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editingItem, setEditingItem] = useState<any | null>(null);
  const [form, setForm] = useState<CampaignFormState>(EMPTY_FORM);
  const [linkMode, setLinkMode] = useState<'none' | 'link'>('none');
  const [platform, setPlatform] = useState('');
  const [propertyId, setPropertyId] = useState('');
  const [periodFrom, setPeriodFrom] = useState('');
  const [periodTo, setPeriodTo] = useState('');
  const [trackingAvailable, setTrackingAvailable] = useState(false);
  const [trackingBusy, setTrackingBusy] = useState(false);
  const [trackingMsg, setTrackingMsg] = useState('');
  const [previewItem, setPreviewItem] = useState<any | null>(null);
  const [previewBusy, setPreviewBusy] = useState(false);
  const [printItem, setPrintItem] = useState<any | null>(null);
  const [officeIdentity, setOfficeIdentity] = useState<any | null>(null);

  const friendlyTrackingError = (err: any, fallbackKey: string) => {
    const raw = String(err?.message || '').trim();
    if (!raw) return tLocale(fallbackKey);
    if (raw.includes('غير مهيأة') || raw.includes('not configured') || raw.includes('OBR_API_KEY') || raw.includes('مفتاح خدمة التتبع')) {
      return tLocale('camp.tracking-auth-missing');
    }
    const lower = raw.toLowerCase();
    if (
      lower.includes('localhost') ||
      lower.includes('127.0.0.1') ||
      lower.includes('8787') ||
      lower.includes('endpoint') ||
      lower.includes('econnrefused') ||
      lower.includes('ureq') ||
      lower.includes('stack') ||
      lower.includes('npm') ||
      lower.includes('powershell')
    ) {
      return tLocale(fallbackKey);
    }
    return raw;
  };

  const loadProperties = async () => {
    try {
      const data = await api('/api/properties?limit=500&page=1');
      setProperties(Array.isArray(data?.items) ? data.items : []);
    } catch (err: any) {
      setProperties([]);
      setError(err?.message || tLocale('camp.load-props-fail'));
    }
  };

  const filterQuery = () => {
    const params = new URLSearchParams();
    if (platform) params.set('platform', platform);
    if (propertyId) params.set('property_id', propertyId);
    if (periodFrom) params.set('period_from', periodFrom);
    if (periodTo) params.set('period_to', periodTo);
    return params;
  };

  const loadStats = async () => {
    const params = filterQuery();
    const data = await api(`/api/campaigns/stats?${params}`);
    setStats(data);
  };

  const syncTrackingQuietly = async (list: any[]) => {
    const enabled = (list || []).filter((c) => c.tracking_enabled && c.tracking_code);
    if (!enabled.length) return list;
    const updated = await Promise.all(
      enabled.map(async (c) => {
        try {
          return await api(`/api/campaigns/${c.id}/tracking/sync`, { method: 'POST', body: {} });
        } catch {
          return { ...c, _tracking_sync_failed: true };
        }
      })
    );
    const byId = new Map(updated.map((c) => [c.id, c]));
    return (list || []).map((c) => byId.get(c.id) || c);
  };

  const loadItems = async () => {
    const params = filterQuery();
    params.set('page', String(page));
    params.set('limit', String(TABLE_PAGE_SIZE));
    const data = await api(`/api/campaigns?${params}`);
    const raw = data.items || [];
    const merged = await syncTrackingQuietly(raw);
    setItems(merged);
    setTotal(data.total || 0);
    if (editingId) {
      const current = merged.find((c: any) => c.id === editingId);
      if (current) setEditingItem(current);
    }
  };

  const reload = async () => {
    setLoading(true);
    setError('');
    try {
      await Promise.all([loadStats(), loadItems()]);
    } catch (err: any) {
      setError(err?.message || tLocale('camp.load-fail'));
      setItems([]);
      setStats(null);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadProperties();
    (async () => {
      try {
        const status = await api('/api/campaigns/tracking/status');
        setTrackingAvailable(Boolean(status?.available));
      } catch {
        setTrackingAvailable(false);
      }
    })();
    // هوية مكتب العميل من الإعدادات — لمخرجات الطباعة فقط.
    api('/api/settings').then((s) => setOfficeIdentity(s || null)).catch(() => setOfficeIdentity(null));
  }, []);

  useEffect(() => {
    reload();
  }, [page, platform, propertyId, periodFrom, periodTo]);

  const openCreate = () => {
    setEditingId(null);
    setEditingItem(null);
    setForm(EMPTY_FORM);
    setLinkMode('none');
    setFormError('');
    setTrackingMsg('');
    setShowForm(true);
    void loadProperties();
  };

  const openEdit = async (item: any) => {
    setEditingId(item.id);
    setEditingItem(item);
    setForm(toForm(item));
    setLinkMode(item.property_id ? 'link' : 'none');
    setFormError('');
    setTrackingMsg('');
    setShowForm(true);
    void loadProperties();
    if (item.tracking_enabled && item.tracking_code) {
      try {
        const synced = await api(`/api/campaigns/${item.id}/tracking/sync`, { method: 'POST', body: {} });
        setEditingItem(synced);
        setForm(toForm(synced));
        setLinkMode(synced.property_id ? 'link' : 'none');
      } catch (err: any) {
        setTrackingMsg(friendlyTrackingError(err, 'camp.tracking-sync-fail'));
      }
    }
  };

  const payloadFromForm = () => ({
    name: form.name.trim(),
    property_id: linkMode === 'link' && form.property_id ? form.property_id : null,
    platform: form.platform,
    ad_link: form.ad_link.trim(),
    start_date: form.start_date,
    end_date: form.end_date,
    views: Number(form.views),
    inquiries: Number(form.inquiries),
    viewings: Number(form.viewings),
    deals: Number(form.deals),
    cost: Number(form.cost),
    notes: form.notes.trim()
  });

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    setFormError('');
    if (linkMode === 'link' && !form.property_id) {
      setFormError(t('camp.need-property'));
      return;
    }
    setSaving(true);
    try {
      const body = payloadFromForm();
      let saved: any;
      if (editingId) {
        saved = await api(`/api/campaigns/${editingId}`, { method: 'PUT', body });
      } else {
        saved = await api('/api/campaigns', { method: 'POST', body });
      }
      setEditingId(saved.id);
      setEditingItem(saved);
      setForm(toForm(saved));
      setLinkMode(saved.property_id ? 'link' : 'none');
      await reload();
    } catch (err: any) {
      setFormError(err?.message || tLocale('camp.save-fail'));
    } finally {
      setSaving(false);
    }
  };

  const canCreateTracking =
    form.name.trim().length >= 2 &&
    Boolean(form.platform) &&
    isHttpsAdLink(form.ad_link) &&
    trackingAvailable &&
    !trackingBusy;

  const handleCreateTracking = async () => {
    if (!trackingAvailable) {
      setTrackingMsg(t('camp.tracking-unavailable'));
      return;
    }
    if (form.name.trim().length < 2) {
      setTrackingMsg(t('camp.tracking-need-name'));
      return;
    }
    if (!form.platform) {
      setTrackingMsg(t('camp.tracking-need-platform'));
      return;
    }
    if (!isHttpsAdLink(form.ad_link)) {
      setTrackingMsg(t('camp.tracking-need-https'));
      return;
    }
    if (linkMode === 'link' && !form.property_id) {
      setTrackingMsg(t('camp.need-property'));
      return;
    }
    setTrackingBusy(true);
    setTrackingMsg('');
    try {
      const body = payloadFromForm();
      let campaignId = editingId;
      let saved: any;
      if (!campaignId) {
        saved = await api('/api/campaigns', { method: 'POST', body });
        campaignId = saved.id;
        setEditingId(saved.id);
      } else {
        saved = await api(`/api/campaigns/${campaignId}`, { method: 'PUT', body });
      }
      const tracked = await api(`/api/campaigns/${campaignId}/tracking/enable`, {
        method: 'POST',
        body: {}
      });
      setEditingItem(tracked);
      setForm(toForm({ ...saved, ...tracked }));
      setLinkMode((saved.property_id || tracked.property_id) ? 'link' : 'none');
      await reload();
    } catch (err: any) {
      setTrackingMsg(friendlyTrackingError(err, 'camp.tracking-create-fail'));
    } finally {
      setTrackingBusy(false);
    }
  };

  const handleDisableTracking = async () => {
    if (!editingId) return;
    setTrackingBusy(true);
    setTrackingMsg('');
    try {
      const tracked = await api(`/api/campaigns/${editingId}/tracking/disable`, { method: 'POST', body: {} });
      setEditingItem(tracked);
      await reload();
    } catch (err: any) {
      setTrackingMsg(friendlyTrackingError(err, 'camp.tracking-sync-fail'));
    } finally {
      setTrackingBusy(false);
    }
  };

  const handleSyncTracking = async () => {
    if (!editingId) return;
    setTrackingBusy(true);
    setTrackingMsg('');
    try {
      const tracked = await api(`/api/campaigns/${editingId}/tracking/sync`, { method: 'POST', body: {} });
      setEditingItem(tracked);
      await reload();
    } catch (err: any) {
      setTrackingMsg(friendlyTrackingError(err, 'camp.tracking-sync-fail'));
    } finally {
      setTrackingBusy(false);
    }
  };

  const copyTrackingUrl = async () => {
    const url = editingItem?.tracking_url;
    if (!url) return;
    try {
      await navigator.clipboard.writeText(url);
      setTrackingMsg(t('camp.tracking-copied'));
    } catch {
      setTrackingMsg(t('camp.tracking-create-fail'));
    }
  };

  const openTrackingUrl = async () => {
    const url = editingItem?.tracking_url;
    if (!url) return;
    try {
      await invoke('open_external_url', { url });
    } catch (err: any) {
      setTrackingMsg(friendlyTrackingError(err, 'camp.tracking-sync-fail'));
    }
  };

  const trackingFriendlyName = trackingDisplayName({
    platform: form.platform || editingItem?.platform,
    adLink: form.ad_link || editingItem?.ad_link,
    originalAdUrl: editingItem?.original_ad_url,
    fallback: t('camp.tracking-obr-name')
  });

  const trackingDestinationHost =
    destinationHostLabel(editingItem?.original_ad_url) ||
    destinationHostLabel(form.ad_link || editingItem?.ad_link);

  const openPreview = async (item: any) => {
    setPreviewItem(item);
    if (!item?.id || !item?.tracking_enabled || !item?.tracking_code) return;
    setPreviewBusy(true);
    try {
      const synced = await api(`/api/campaigns/${item.id}/tracking/sync`, { method: 'POST', body: {} });
      setPreviewItem(synced);
      setItems((prev) => prev.map((c) => (c.id === synced.id ? { ...c, ...synced } : c)));
    } catch {
      /* المعاينة تعرض البيانات المحلية إن فشل المزامنة */
    } finally {
      setPreviewBusy(false);
    }
  };

  const syncCampaignReport = async (item: any) => {
    if (!item?.id || !item?.tracking_enabled || !item?.tracking_code) return item;
    try {
      const synced = await api(`/api/campaigns/${item.id}/tracking/sync`, { method: 'POST', body: {} });
      setItems((prev) => prev.map((c) => (c.id === synced.id ? { ...c, ...synced } : c)));
      return synced;
    } catch {
      return item;
    }
  };

  const printCampaign = async (item: any) => {
    const report = await syncCampaignReport(item);
    setPrintItem(report);
  };

  const handleDelete = async (item: any) => {
    if (!window.confirm(tfLocale('camp.delete-confirm', { name: item.name }))) return;
    setError('');
    try {
      await api(`/api/campaigns/${item.id}`, { method: 'DELETE' });
      if (editingId === item.id) {
        setShowForm(false);
        setEditingId(null);
        setEditingItem(null);
      }
      await reload();
    } catch (err: any) {
      setError(err?.message || tLocale('camp.delete-fail'));
    }
  };

  useEffect(() => {
    if (!previewItem) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setPreviewItem(null);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [previewItem]);

  useEffect(() => {
    if (!printItem) return;
    document.body.classList.add('camp-printing');
    // إخفاء ترويسة/تذييل الطباعة الافتراضية (tauri.localhost + 1/1):
    // Chromium يرسمها داخل هوامش الصفحة، فتصفير الهامش يلغيها — نفس نمط ods-print-page-style.
    let pageStyle = document.getElementById('camp-print-page-style') as HTMLStyleElement | null;
    if (!pageStyle) {
      pageStyle = document.createElement('style');
      pageStyle.id = 'camp-print-page-style';
      document.head.appendChild(pageStyle);
    }
    pageStyle.textContent = '@media print { @page { size: A4; margin: 0; } }';
    let cleaned = false;
    const clearPrint = () => {
      if (cleaned) return;
      cleaned = true;
      pageStyle?.remove();
      document.body.classList.remove('camp-printing');
      setPrintItem(null);
    };
    const onAfterPrint = () => clearPrint();
    const onPrintMedia = (e: MediaQueryListEvent) => {
      if (!e.matches) clearPrint();
    };
    const printMql = window.matchMedia('print');
    window.addEventListener('afterprint', onAfterPrint);
    printMql.addEventListener('change', onPrintMedia);
    const printTimer = window.setTimeout(() => window.print(), 60);
    return () => {
      window.clearTimeout(printTimer);
      window.removeEventListener('afterprint', onAfterPrint);
      printMql.removeEventListener('change', onPrintMedia);
      pageStyle?.remove();
      document.body.classList.remove('camp-printing');
    };
  }, [printItem]);

  const funnelStages = useMemo(() => ([
    { label: t('camp.funnel.views'), value: Number(stats?.views || 0) },
    { label: t('camp.funnel.inquiries'), value: Number(stats?.inquiries || 0), rateLabel: formatRate(stats?.inquiry_rate) },
    { label: t('camp.funnel.viewings'), value: Number(stats?.viewings || 0), rateLabel: formatRate(stats?.viewing_conversion) },
    { label: t('camp.funnel.deals'), value: Number(stats?.deals || 0), rateLabel: formatRate(stats?.deal_conversion) }
  ]), [stats, lang, t]);

  const platformSlices = useMemo(() => {
    const buckets: Record<string, number> = {};
    for (const item of items) {
      const key = item.platform || 'other';
      buckets[key] = (buckets[key] || 0) + Number(item.inquiries || 0);
    }
    const colors = ['var(--chart-1)', 'var(--chart-2)', 'var(--chart-3)', 'var(--chart-4)', 'var(--chart-5)'];
    return Object.entries(buckets)
      .filter(([, value]) => value > 0)
      .sort((a, b) => b[1] - a[1])
      .map(([key, value], i) => ({
        label: platformLabel(key),
        value,
        color: colors[i % colors.length]
      }));
  }, [items, lang]);

  const compareRows = useMemo(() => (
    [...items]
      .map((item) => ({
        id: item.id,
        name: item.name,
        inquiries: Number(item.inquiries || 0),
        viewings: Number(item.viewings || 0),
        deals: Number(item.deals || 0),
        inquiryRate: formatRate(item.inquiry_rate),
        viewingRate: formatRate(item.viewing_conversion),
        dealRate: formatRate(item.deal_conversion)
      }))
      .sort((a, b) => b.inquiries - a.inquiries || b.viewings - a.viewings || b.deals - a.deals)
  ), [items, lang]);

  const costRows = useMemo(() => {
    const scoped = items.filter(item => item.country === getCountry());
    const totalCost = scoped.reduce((sum, item) => sum + Number(item.cost || 0), 0);
    return [...scoped]
      .map((item) => {
        const cost = Number(item.cost || 0);
        const share = totalCost > 0 ? (cost / totalCost) * 100 : 0;
        return {
          id: item.id,
          name: item.name,
          cost,
          costDisplay: formatMoneyAmount(item.cost ?? 0, item.currency),
          cpi: formatMoneyAmount(item.cost_per_inquiry, item.currency),
          cpv: formatMoneyAmount(item.cost_per_viewing, item.currency),
          cpd: formatMoneyAmount(item.cost_per_deal, item.currency),
          share,
          shareDisplay: totalCost > 0 ? `${share.toFixed(1)}%` : '—'
        };
      })
      .sort((a, b) => b.cost - a.cost);
  }, [items, lang]);

  const kpis = [
    { label: t('camp.kpi.count'), value: formatCount(stats?.count) },
    { label: t('camp.kpi.views'), value: formatCount(stats?.views) },
    { label: t('camp.kpi.inquiries'), value: formatCount(stats?.inquiries) },
    { label: t('camp.kpi.viewings'), value: formatCount(stats?.viewings) },
    { label: t('camp.kpi.deals'), value: formatCount(stats?.deals) },
    { label: t('camp.kpi.cost'), value: formatMoneyAmount(stats?.cost) },
    { label: t('camp.kpi.inq-rate'), value: formatRate(stats?.inquiry_rate) },
    { label: t('camp.kpi.view-conv'), value: formatRate(stats?.viewing_conversion) },
    { label: t('camp.kpi.deal-conv'), value: formatRate(stats?.deal_conversion) },
    { label: t('camp.kpi.cpi'), value: formatMoneyAmount(stats?.cost_per_inquiry) },
    { label: t('camp.kpi.cpv'), value: formatMoneyAmount(stats?.cost_per_viewing) },
    { label: t('camp.kpi.cpd'), value: formatMoneyAmount(stats?.cost_per_deal) }
  ];

  return (
    <div>
      <div className="card-header">
        <div className="card-title">
          <Megaphone size={17} strokeWidth={1.8} style={{ verticalAlign: '-3px' }} /> {t('camp.title')} <SectionGuideBanner id="campaigns" />
        </div>
        <button className="btn btn-primary" type="button" onClick={openCreate}>
          <Plus size={15} strokeWidth={1.8} /> {t('camp.add')}
        </button>
      </div>

      <div className="alert-note">
        <TriangleAlert size={14} strokeWidth={1.8} style={{ verticalAlign: '-2px' }} />
        {' '}{t('camp.hint')}
      </div>

      {error && <div className="error-alert" role="alert">{error} <button type="button" className="btn btn-outline" onClick={reload}>{t('retry')}</button></div>}

      {showForm && (
        <div className="content-card obr-panel">
          <div className="card-header" style={{ marginTop: 0 }}>
            <div className="card-title"><Pencil size={17} strokeWidth={1.8} style={{ verticalAlign: '-3px' }} /> {editingId ? t('camp.edit-title') : t('camp.new-title')}</div>
          </div>
          {formError && <div className="error-alert">{formError}</div>}
          <form {...noAutofill} onSubmit={handleSave}>
            <div className="form-grid">
              <div className="form-group">
                <label htmlFor="camp-name">{t('camp.name')}</label>
                <input id="camp-name" className="form-input" autoComplete="off" value={form.name} required minLength={2} onChange={(e) => setForm({ ...form, name: e.target.value })} />
              </div>
              <div className="form-group">
                <label htmlFor="camp-link-mode">{t('camp.link-mode')}</label>
                <select
                  id="camp-link-mode"
                  className="form-input"
                  autoComplete="off"
                  value={linkMode}
                  onChange={(e) => {
                    const next = e.target.value === 'link' ? 'link' : 'none';
                    setLinkMode(next);
                    if (next === 'none') {
                      setForm({ ...form, property_id: '' });
                    }
                  }}
                >
                  <option value="none">{t('camp.link-none')}</option>
                  <option value="link">{t('camp.link-yes')}</option>
                </select>
              </div>
              {linkMode === 'link' ? (
                <div className="form-group">
                  <label htmlFor="camp-property">{t('camp.linked-property')}</label>
                  <select
                    id="camp-property"
                    className="form-input"
                    autoComplete="off"
                    value={form.property_id}
                    required
                    onChange={(e) => setForm({ ...form, property_id: e.target.value })}
                  >
                    <option value="">{t('camp.choose-property')}</option>
                    {properties.map((p) => (
                      <option key={p.id} value={p.id}>{p.name}</option>
                    ))}
                  </select>
                  {properties.length === 0 ? (
                    <p className="muted" style={{ margin: '6px 0 0', fontSize: 12 }}>{t('camp.no-listings-to-link')}</p>
                  ) : null}
                </div>
              ) : null}
              <div className="form-group">
                <label htmlFor="camp-platform">{t('camp.platform')}</label>
                <select id="camp-platform" className="form-input" autoComplete="off" value={form.platform} required onChange={(e) => setForm({ ...form, platform: e.target.value })}>
                  <option value="">{t('camp.choose-platform')}</option>
                  {countryPlatforms(editingItem?.country || properties.find(p => p.id === form.property_id)?.country).map((p) => (
                    <option key={p.id} value={p.id}>{p.name}</option>
                  ))}
                </select>
              </div>
              <div className="form-group">
                <label htmlFor="camp-link">{t('camp.ad-link')}</label>
                <input id="camp-link" className="form-input campaign-url" dir="ltr" autoComplete="off" type="url" placeholder="https://" value={form.ad_link} onChange={(e) => setForm({ ...form, ad_link: e.target.value })} />
              </div>
              <div className="form-group full-row camp-tracking-box">
                <label>{t('camp.tracking-title')}</label>
                <p className="muted" style={{ margin: '0 0 10px', fontSize: 12, lineHeight: 1.65 }}>{t('camp.tracking-hint')}</p>
                {!trackingAvailable ? (
                  <div className="error-alert" role="status">{t('camp.tracking-unavailable')}</div>
                ) : !editingItem?.tracking_url ? (
                  <button
                    type="button"
                    className="btn btn-outline"
                    disabled={!canCreateTracking}
                    onClick={handleCreateTracking}
                  >
                    <Link2 size={15} strokeWidth={1.8} /> {t('camp.tracking-create')}
                  </button>
                ) : (
                  <div className="camp-tracking-panel">
                    <label>{t('camp.tracking-url')}</label>
                    <button
                      type="button"
                      className="form-input camp-tracking-friendly"
                      style={{
                        textAlign: 'start',
                        cursor: trackingBusy ? 'default' : 'pointer',
                        fontWeight: 700,
                        color: 'var(--primary)'
                      }}
                      disabled={trackingBusy}
                      onClick={openTrackingUrl}
                      title={t('camp.tracking-open')}
                    >
                      {trackingFriendlyName}
                    </button>
                    {trackingDestinationHost ? (
                      <p className="muted" style={{ margin: '6px 0 0', fontSize: 12 }}>
                        {tf('camp.tracking-destination', { host: trackingDestinationHost })}
                      </p>
                    ) : null}
                    <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8, marginTop: 8 }}>
                      <button type="button" className="btn btn-outline" disabled={trackingBusy} onClick={copyTrackingUrl}>
                        <Copy size={14} strokeWidth={1.8} /> {t('camp.tracking-copy')}
                      </button>
                      <button type="button" className="btn btn-outline" disabled={trackingBusy} onClick={openTrackingUrl}>
                        <ExternalLink size={14} strokeWidth={1.8} /> {t('camp.tracking-open')}
                      </button>
                      {editingItem.tracking_enabled ? (
                        <button type="button" className="btn btn-outline" disabled={trackingBusy} onClick={handleDisableTracking}>
                          <PauseCircle size={14} strokeWidth={1.8} /> {t('camp.tracking-disable')}
                        </button>
                      ) : null}
                    </div>
                    <div className="campaign-card-metrics" style={{ marginTop: 12 }}>
                      <table className="campaign-metrics-table">
                        <thead>
                          <tr>
                            <th scope="col">{t('camp.tracking-clicks')}</th>
                            <th scope="col">{t('camp.tracking-unique')}</th>
                            <th scope="col">{t('camp.tracking-last')}</th>
                          </tr>
                        </thead>
                        <tbody>
                          <tr>
                            <td className="num">{formatCount(editingItem.tracked_clicks ?? 0)}</td>
                            <td className="num">{formatCount(editingItem.unique_clicks ?? 0)}</td>
                            <td className="num">{editingItem.last_click_at ? formatDateStamp(editingItem.last_click_at) : t('dash')}</td>
                          </tr>
                        </tbody>
                      </table>
                    </div>
                    <button
                      type="button"
                      className="btn btn-outline"
                      style={{ marginTop: 10 }}
                      disabled={trackingBusy}
                      onClick={handleSyncTracking}
                    >
                      <RefreshCw size={14} strokeWidth={1.8} /> {t('camp.tracking-sync')}
                    </button>
                  </div>
                )}
                {trackingMsg ? <div className="error-alert" style={{ marginTop: 10 }}>{trackingMsg}</div> : null}
              </div>
              <div className="form-group">
                <label htmlFor="camp-start">{t('camp.start')}</label>
                <input id="camp-start" className="form-input" autoComplete="off" type="date" value={form.start_date} onChange={(e) => setForm({ ...form, start_date: e.target.value })} />
              </div>
              <div className="form-group">
                <label htmlFor="camp-end">{t('camp.end')}</label>
                <input id="camp-end" className="form-input" autoComplete="off" type="date" value={form.end_date} onChange={(e) => setForm({ ...form, end_date: e.target.value })} />
              </div>
              <div className="form-group">
                <label htmlFor="camp-views">{t('camp.views')}</label>
                <input id="camp-views" className="form-input" autoComplete="off" type="number" min={0} step={1} value={form.views} onChange={(e) => setForm({ ...form, views: e.target.value })} />
              </div>
              <div className="form-group">
                <label htmlFor="camp-inquiries">{t('camp.inquiries')}</label>
                <input id="camp-inquiries" className="form-input" autoComplete="off" type="number" min={0} step={1} value={form.inquiries} onChange={(e) => setForm({ ...form, inquiries: e.target.value })} />
              </div>
              <div className="form-group">
                <label htmlFor="camp-viewings">{t('camp.viewings')}</label>
                <input id="camp-viewings" className="form-input" autoComplete="off" type="number" min={0} step={1} value={form.viewings} onChange={(e) => setForm({ ...form, viewings: e.target.value })} />
              </div>
              <div className="form-group">
                <label htmlFor="camp-deals">{t('camp.deals')}</label>
                <input id="camp-deals" className="form-input" autoComplete="off" type="number" min={0} step={1} value={form.deals} onChange={(e) => setForm({ ...form, deals: e.target.value })} />
              </div>
              <div className="form-group">
                <label htmlFor="camp-cost">{tf('camp.cost-cur', { cur: editingItem?.currency || properties.find(p => p.id === form.property_id)?.currency || currencySymbol() })}</label>
                <input id="camp-cost" className="form-input" autoComplete="off" type="number" min={0} step="0.01" value={form.cost} onChange={(e) => setForm({ ...form, cost: e.target.value })} />
              </div>
              <div className="form-group full-row">
                <label htmlFor="camp-notes">{t('notes')}</label>
                <input id="camp-notes" className="form-input" autoComplete="off" value={form.notes} onChange={(e) => setForm({ ...form, notes: e.target.value })} />
              </div>
            </div>
            <div style={{ display: 'flex', gap: 8, marginTop: 16 }}>
              <button className="btn btn-primary" type="submit" disabled={saving}>{saving ? t('saving') : t('save')}</button>
              <button className="btn btn-outline" type="button" onClick={() => { setShowForm(false); setFormError(''); setEditingItem(null); setTrackingMsg(''); }}>{t('cancel')}</button>
            </div>
          </form>
        </div>
      )}

      <div className="content-card obr-panel">
        <div className="toolbar">
          <select className="search-input" value={platform} onChange={(e) => { setPage(1); setPlatform(e.target.value); }} aria-label={t('camp.filter-platform')}>
            <option value="">{t('camp.all-platforms')}</option>
            {CAMPAIGN_PLATFORMS.map((p) => (
              <option key={p.id} value={p.id}>{p.label}</option>
            ))}
          </select>
          <select className="search-input" value={propertyId} onChange={(e) => { setPage(1); setPropertyId(e.target.value); }} aria-label={t('camp.filter-property')}>
            <option value="">{t('camp.all-listings')}</option>
            {properties.map((p) => (
              <option key={p.id} value={p.id}>{p.name}</option>
            ))}
          </select>
          <input className="search-input" autoComplete="off" type="date" value={periodFrom} onChange={(e) => { setPage(1); setPeriodFrom(e.target.value); }} aria-label={t('from_date')} />
          <input className="search-input" autoComplete="off" type="date" value={periodTo} onChange={(e) => { setPage(1); setPeriodTo(e.target.value); }} aria-label={t('to_date')} />
        </div>

        {stats && (
          <div className="stats-grid" style={{ marginBottom: 0, padding: '8px 14px 14px' }}>
            {kpis.map((kpi) => (
              <div className="stat-card" key={kpi.label}>
                <div className="stat-value">{kpi.value}</div>
                <div className="stat-label">{kpi.label}</div>
              </div>
            ))}
          </div>
        )}
      </div>

      {loading ? (
        <div className="content-card obr-panel"><div className="obr-panel-body">{t('loading')}</div></div>
      ) : items.length === 0 ? (
        <div className="content-card obr-panel">
          <div className="empty-state">
            <Megaphone size={40} strokeWidth={1.5} aria-hidden style={{ marginBottom: 8, color: 'var(--gold)' }} />
            <div>{t('camp.empty')}</div>
          </div>
        </div>
      ) : (
        <>
            <div className="camp-sections">
            <div className="charts-row">
              <div className="content-card obr-panel" style={{ marginBottom: 0, flex: 1.15, minWidth: 320 }}>
                <div className="card-header">
                  <div className="card-title"><Filter size={17} strokeWidth={1.8} style={{ verticalAlign: '-3px' }} /> {clientConversionStagesTitle()}</div>
                </div>
                <div className="obr-panel-body">
                <FunnelChart stages={funnelStages} />
                </div>
              </div>
              <div className="content-card obr-panel" style={{ marginBottom: 0, flex: 1, minWidth: 300 }}>
                <div className="card-header">
                  <div className="card-title"><Share2 size={17} strokeWidth={1.8} style={{ verticalAlign: '-3px' }} /> {t('camp.by-platform')}</div>
                </div>
                <div className="obr-panel-body">
                {platformSlices.length > 0 ? (
                  <DonutChart
                    slices={platformSlices}
                    centerValue={formatCount(stats?.inquiries)}
                    centerLabel={t('camp.inquiry-one')}
                  />
                ) : (
                  <div className="empty-state" style={{ padding: 28 }}>{t('camp.no-inq-dist')}</div>
                )}
                </div>
              </div>
            </div>

            <div className="charts-row camp-analytics-row">
              <div className="content-card obr-panel" style={{ marginBottom: 0 }}>
                <div className="card-header">
                  <div className="card-title"><BarChart3 size={17} strokeWidth={1.8} style={{ verticalAlign: '-3px' }} /> {t('camp.performance')}</div>
                </div>
                <div className="obr-panel-body">
                  {compareRows.length > 0 ? (
                    <div className="table-wrap camp-analytics-wrap">
                      <table className="data-table camp-analytics-table">
                        <thead>
                          <tr>
                            <th scope="col">{t('camp.col.campaign')}</th>
                            <th scope="col" className="num">{t('camp.series.inquiry')}</th>
                            <th scope="col" className="num">{t('camp.series.viewing')}</th>
                            <th scope="col" className="num">{t('camp.series.deal')}</th>
                            <th scope="col" className="num">{t('camp.metric.inq-rate')}</th>
                            <th scope="col" className="num">{t('camp.metric.to-viewing')}</th>
                            <th scope="col" className="num">{t('camp.kpi.deal-conv')}</th>
                          </tr>
                        </thead>
                        <tbody>
                          {compareRows.map((row) => (
                            <tr key={row.id}>
                              <td>{row.name}</td>
                              <td className="num">{formatCount(row.inquiries)}</td>
                              <td className="num">{formatCount(row.viewings)}</td>
                              <td className="num">{formatCount(row.deals)}</td>
                              <td className="num">{row.inquiryRate}</td>
                              <td className="num">{row.viewingRate}</td>
                              <td className="num">{row.dealRate}</td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>
                  ) : (
                    <div className="empty-state" style={{ padding: 28 }}>{t('camp.no-inq-dist')}</div>
                  )}
                </div>
              </div>
              <div className="content-card obr-panel" style={{ marginBottom: 0 }}>
                <div className="card-header">
                  <div className="card-title"><CircleDollarSign size={17} strokeWidth={1.8} style={{ verticalAlign: '-3px' }} /> {t('camp.cost-chart')}</div>
                </div>
                <div className="obr-panel-body">
                  {costRows.length > 0 ? (
                    <div className="table-wrap camp-analytics-wrap">
                      <table className="data-table camp-analytics-table">
                        <thead>
                          <tr>
                            <th scope="col">{t('camp.col.campaign')}</th>
                            <th scope="col" className="num">{t('camp.kpi.cost')}</th>
                            <th scope="col" className="num">{t('camp.kpi.cpi')}</th>
                            <th scope="col" className="num">{t('camp.kpi.cpd')}</th>
                            <th scope="col">{t('camp.col.share')}</th>
                          </tr>
                        </thead>
                        <tbody>
                          {costRows.map((row) => (
                            <tr key={row.id}>
                              <td>{row.name}</td>
                              <td className="num">{row.costDisplay}</td>
                              <td className="num">{row.cpi}</td>
                              <td className="num">{row.cpd}</td>
                              <td>
                                <div className="camp-share-cell">
                                  <span className="num">{row.shareDisplay}</span>
                                  <span className="camp-share-track" aria-hidden="true">
                                    <span className="camp-share-fill" style={{ width: `${Math.min(100, Math.max(0, row.share))}%` }} />
                                  </span>
                                </div>
                              </td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>
                  ) : (
                    <div className="empty-state" style={{ padding: 28 }}>{t('camp.no-inq-dist')}</div>
                  )}
                </div>
              </div>
            </div>

            <div className="content-card obr-panel" style={{ marginBottom: 0 }}>
              <div className="card-header">
                <div className="card-title"><ScrollText size={17} strokeWidth={1.8} style={{ verticalAlign: '-3px' }} /> {t('camp.log')}</div>
              </div>
              <div className="campaign-board">
              {items.map((item) => (
                <article key={item.id} className="campaign-card">
                  <div>
                    <div className="campaign-card-title">{item.name}</div>
                    <div className="campaign-card-meta">
                      {platformLabel(item.platform)}
                      {' · '}
                      {item.property_name || t('no_property')}
                      {' · '}
                      {formatDateStamp(item.start_date)} — {formatDateStamp(item.end_date)}
                    </div>
                    <div className="campaign-card-meta">{tf('camp.updated', { date: formatDateStamp(item.updated_at) })}</div>
                  </div>
                  <div className="campaign-card-metrics">
                    <table className="campaign-metrics-table">
                      <thead>
                        <tr>
                          <th scope="col">{t('camp.total-views')}</th>
                          <th scope="col">{t('camp.tracking-clicks')}</th>
                          <th scope="col">{t('camp.metric.inq-rate')}</th>
                          <th scope="col">{t('camp.metric.to-viewing')}</th>
                          <th scope="col">{t('camp.metric.deal-cost')}</th>
                        </tr>
                      </thead>
                      <tbody>
                        <tr>
                          <td className="num">{formatCount(campaignTotalViews(item.views, item.tracked_clicks))}</td>
                          <td className="num">{formatCount(item.tracked_clicks)}</td>
                          <td className="num">{campaignRate(item.inquiries, campaignTotalViews(item.views, item.tracked_clicks))}</td>
                          <td className="num">{formatRate(item.viewing_conversion)}</td>
                          <td className="num">{formatMoneyAmount(item.cost_per_deal, item.currency)}</td>
                        </tr>
                      </tbody>
                    </table>
                  </div>
                  <div className="campaign-card-actions">
                    <button className="btn btn-outline" type="button" onClick={() => openPreview(item)} aria-label={`${t('camp.preview')} ${item.name}`} title={t('camp.preview')}>
                      <Eye size={14} strokeWidth={1.8} />
                    </button>
                    <button className="btn btn-outline" type="button" onClick={() => void printCampaign(item)} aria-label={`${t('camp.print')} ${item.name}`} title={t('camp.print')}>
                      <Printer size={14} strokeWidth={1.8} />
                    </button>
                    <button className="btn btn-outline" type="button" onClick={() => openEdit(item)} aria-label={`${t('edit')} ${item.name}`}>
                      <Pencil size={14} strokeWidth={1.8} />
                    </button>
                    <button className="btn btn-outline" type="button" onClick={() => handleDelete(item)} aria-label={`${t('delete')} ${item.name}`}>
                      <Trash2 size={14} strokeWidth={1.8} />
                    </button>
                  </div>
                </article>
              ))}
              </div>
            </div>
            </div>

            <PaginationBar total={total} page={page} onPage={setPage} label={t('campaign_label')} />
        </>
      )}

      {previewItem && (
        <div className="modal-overlay" role="presentation" onClick={() => setPreviewItem(null)}>
          <div
            className="modal campaign-preview-modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="camp-preview-title"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="modal-header">
              <div className="modal-title" id="camp-preview-title">
                <Eye size={17} strokeWidth={1.8} style={{ verticalAlign: '-3px' }} /> {t('camp.preview-title')}
              </div>
              <button type="button" className="modal-close" onClick={() => setPreviewItem(null)} aria-label={t('camp.preview-close')}>
                ×
              </button>
            </div>
            <div className="modal-body">
              <CampaignReportBody item={previewItem} busy={previewBusy} />
            </div>
            <div className="modal-footer">
              <button type="button" className="btn btn-outline" onClick={() => setPreviewItem(null)}>
                {t('camp.preview-close')}
              </button>
              <button
                type="button"
                className="btn btn-outline"
                onClick={() => {
                  const item = previewItem;
                  setPreviewItem(null);
                  void printCampaign(item);
                }}
              >
                <Printer size={14} strokeWidth={1.8} /> {t('camp.print')}
              </button>
              <button
                type="button"
                className="btn btn-primary"
                onClick={() => {
                  const item = previewItem;
                  setPreviewItem(null);
                  openEdit(item);
                }}
              >
                <Pencil size={14} strokeWidth={1.8} /> {t('edit')}
              </button>
            </div>
          </div>
        </div>
      )}

      {printItem ? (
        <div className="camp-print-root" aria-hidden={!printItem}>
          <CampaignPrintTables item={printItem} identity={officeIdentity} />
        </div>
      ) : null}
    </div>
  );
}
