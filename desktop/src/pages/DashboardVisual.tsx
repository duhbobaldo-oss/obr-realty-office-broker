import { getCountry } from '../domain/countryProfile';
import React, { useMemo } from 'react';
import { FunnelChart } from '../components/Charts';
import { ApexAreaChart, ApexBarChart, ApexDonutChart, ApexRadialSet } from '../components/ApexChart';
import { IndicatorKpi, MarketPulseTicker, SectionHairlineTitle, type PulseTickerItem } from '../components/IndicatorUi';
import {
  clientConversionStagesTitle,
  CLIENT_STATUSES,
  PROPERTY_PURPOSES,
  PROPERTY_STATUSES,
  roleLabel
} from '../domain/labels';
import { formatRate, platformLabel } from '../campaigns';
import { EXPENSE_CATEGORIES, formatMoneyAmount as formatFinance, STATUS_OPTIONS } from '../finance';
import { templateDisplayName } from '../offer-design/templates-v2';
import { getTemplate, type TemplateId } from '../offer-design/templates';
import { Building2, Users, Eye, Wallet, LayoutDashboard, Percent, TrendingUp, BarChart3, CircleDollarSign, Activity, Target, PieChart, MapPinned, Megaphone, Filter, Share2, UserCog, Banknote, Receipt, Tags, LayoutTemplate, FileText, FileCheck2, Archive, Clock3, FileWarning, HandCoins } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import { currencySymbol } from '../domain/locale';
import { useLocale } from '../hooks/useLocale';

const COLORS = ['#23365f', '#d97706', '#3b4966', '#b45309', '#6f7380'];

type ListPack = { items: any[]; total: number; error: string };
type AggRow = { label: string; value: number };

function num(value: unknown): number {
  const n = Number(value);
  return Number.isFinite(n) ? n : 0;
}

function aggRows(rows: unknown): AggRow[] {
  if (!Array.isArray(rows)) return [];
  return rows.map((r: any) => ({ label: String(r?.label ?? ''), value: num(r?.value) }));
}

/** يربط القيم الخام القادمة من SQLite بالتسميات العربية المعروفة، ويحافظ على القيم غير المعروفة. */
function mapAggLabels(rows: AggRow[], labelFor: (raw: string) => string): AggRow[] {
  return rows.map((r) => ({ label: labelFor(r.label) || r.label, value: r.value }));
}

function toSlices(rows: AggRow[]) {
  return rows
    .filter((row) => row.value > 0)
    .map((row, i) => ({ ...row, color: COLORS[i % COLORS.length] }));
}

function shortDay(value: string): string {
  return value.length >= 10 ? value.slice(5) : value;
}

function shortCampaignChartLabel(name: string, fallback: string): string {
  const raw = String(name || '').trim() || fallback;
  return raw
    .replace(/^حملة\s*OBR\s*[—\-–]\s*/i, '')
    .replace(/^حملة\s+/i, '')
    .replace(/\s*\(إعادة\)\s*$/i, '')
    .replace(/\s*\(تتبع\)\s*$/i, '')
    .trim() || raw;
}

function ChartCard({
  title,
  empty,
  icon: Icon,
  children
}: {
  title: string;
  empty?: boolean;
  icon?: LucideIcon;
  children: React.ReactNode;
}) {
  const { t } = useLocale();
  return (
    <div className="content-card dash-chart-card">
      <div className="card-header">
        <div className="card-title">
          {Icon ? <Icon size={17} strokeWidth={1.8} aria-hidden /> : null}
          {title}
        </div>
      </div>
      {empty ? <div className="dash-empty">{t('dash.empty-chart')}</div> : children}
    </div>
  );
}

function VisualBlock({
  title,
  icon,
  children
}: {
  title: string;
  icon?: LucideIcon;
  children: React.ReactNode;
}) {
  return (
    <div className="dash-visual-block">
      <SectionHairlineTitle title={title} icon={icon} />
      <div className="charts-row dash-visual-row">{children}</div>
    </div>
  );
}

export default function DashboardVisual({
  stats,
  statsError,
  summary,
  summaryError,
  campaignStats,
  aggregates,
  aggregatesError,
  properties,
  clients,
  market,
  campaigns,
  invoices,
  operations,
  commissions,
  expenses,
  payments,
  brochures
}: {
  stats: any;
  statsError: string;
  summary: any;
  summaryError: string;
  campaignStats: any;
  aggregates: any;
  aggregatesError: string;
  properties: ListPack;
  clients: ListPack;
  market: ListPack;
  campaigns: ListPack;
  invoices: ListPack;
  operations: ListPack;
  commissions: ListPack;
  expenses: ListPack;
  payments: ListPack;
  brochures: ListPack;
}) {
  const { t, tf, lang } = useLocale();
  const p = stats?.properties || {};

  const brochureTemplateLabel = (id?: string | null): string => {
    if (!id) return t('dash.no-template');
    const named = templateDisplayName(id);
    if (named !== id) return named;
    const v1 = getTemplate(id as TemplateId);
    if (v1.id === id) return v1.name;
    return id;
  };

  const officeBars = useMemo(() => ([
    { label: t('dash.listings'), value: num(p.total ?? properties.total) },
    { label: t('dash.clients'), value: num(stats?.clients?.total ?? clients.total) },
    { label: t('dash.market-rows'), value: num(stats?.market_rows ?? market.total) },
    { label: t('dash.campaigns'), value: num(stats?.campaigns ?? campaigns.total) },
    { label: t('dash.invoices'), value: num(stats?.finance_invoices ?? invoices.total) },
    { label: t('dash.operations'), value: operations.total },
    { label: t('dash.commissions'), value: commissions.total },
    { label: t('dash.expenses'), value: expenses.total },
    { label: t('dash.payments'), value: payments.total },
    { label: t('dash.brochures'), value: num(stats?.brochures ?? brochures.total) }
  ]), [lang, p.total, stats, properties.total, clients.total, market.total, campaigns.total, invoices.total, operations.total, commissions.total, expenses.total, payments.total, brochures.total]);

  // تجميعات المخططات تأتي جاهزة من SQLite (capacity.rs) محدودة بـ90 نقطة و20 فئة
  const agg = aggregates || {};

  const propertyStatusSlices = useMemo(() => toSlices(mapAggLabels(aggRows(agg.property_statuses), (raw) =>
    PROPERTY_STATUSES.find((s) => s.value === raw)?.label || raw
  )), [lang, agg]);

  const propertyPurposeBars = useMemo(() => mapAggLabels(aggRows(agg.property_purposes), (raw) =>
    PROPERTY_PURPOSES.find((p) => p.value === raw)?.label || raw
  ), [lang, agg]);

  const clientStatusSlices = useMemo(() => toSlices(mapAggLabels(aggRows(agg.client_statuses), (raw) =>
    CLIENT_STATUSES.find((s) => s.value === raw)?.label || raw
  )), [lang, agg]);

  const clientRoleBars = useMemo(() => {
    const merged = new Map<string, number>();
    for (const row of [...aggRows(agg.client_roles), ...aggRows(agg.client_reltypes)]) {
      const label = roleLabel(row.label) || row.label;
      merged.set(label, (merged.get(label) || 0) + row.value);
    }
    return [...merged.entries()]
      .filter(([, value]) => value > 0)
      .sort((a, b) => b[1] - a[1])
      .map(([label, value]) => ({ label, value }));
  }, [lang, agg]);

  const clientFollowBars = useMemo(() => ([
    { label: t('dash.need-followup'), value: num(stats?.clients?.needs_followup) },
    { label: t('dash.comms'), value: num(stats?.communications) },
    { label: t('dash.sent-offers'), value: num(stats?.sent_offers) }
  ].filter((row) => row.value > 0)), [lang, stats]);

  const marketCityBars = useMemo(() => aggRows(agg.market_cities).map((r) => ({ label: r.label, value: r.value })), [agg]);

  const marketPriceBars = useMemo(() => {
    const per = t('dash.per-sqm');
    return aggRows(agg.market_ppm)
      .filter((row) => row.value > 0)
      .map((row) => ({
        label: row.label,
        value: row.value,
        display: `${formatFinance(row.value)} ${per}`
      }))
      .sort((a, b) => b.value - a.value);
  }, [lang, agg]);

  const campaignTotals = useMemo(() => {
    if (campaignStats) return campaignStats;
    const views = campaigns.items.reduce((sum, item) => sum + num(item.views), 0);
    const inquiries = campaigns.items.reduce((sum, item) => sum + num(item.inquiries), 0);
    const viewings = campaigns.items.reduce((sum, item) => sum + num(item.viewings), 0);
    const deals = campaigns.items.reduce((sum, item) => sum + num(item.deals), 0);
    return {
      views,
      inquiries,
      viewings,
      deals,
      inquiry_rate: views > 0 ? inquiries / views : null,
      viewing_conversion: inquiries > 0 ? viewings / inquiries : null,
      deal_conversion: viewings > 0 ? deals / viewings : null
    };
  }, [campaignStats, campaigns.items]);

  const campaignFunnel = useMemo(() => ([
    { label: t('dash.funnel-views'), value: num(campaignTotals.views) },
    { label: t('dash.funnel-inquiries'), value: num(campaignTotals.inquiries), rateLabel: formatRate(campaignTotals.inquiry_rate) },
    { label: t('dash.funnel-viewings'), value: num(campaignTotals.viewings), rateLabel: formatRate(campaignTotals.viewing_conversion) },
    { label: t('dash.funnel-deals'), value: num(campaignTotals.deals), rateLabel: formatRate(campaignTotals.deal_conversion) }
  ]), [lang, campaignTotals]);

  const campaignPlatformSlices = useMemo(() => toSlices(mapAggLabels(aggRows(agg.campaign_platform), (raw) =>
    platformLabel(raw)
  )), [lang, agg]);

  const campaignCompare = useMemo(() => aggRows(agg.campaign_top_details).map((row) => ({
    label: shortCampaignChartLabel(row.label, t('dash.campaign-fallback')),
    values: {
      inquiries: row.value,
      viewings: num((row as any).viewings),
      deals: num((row as any).deals)
    }
  })), [lang, agg]);

  const financeBars = useMemo(() => {
    if (!summary) return [];
    return [
      { key: 'invoice_total', label: t('dash.invoice-total'), value: Math.abs(num(summary.invoice_total)), display: formatFinance(summary.invoice_total, summary?.currency) },
      { key: 'collections', label: t('dash.collections'), value: Math.abs(num(summary.collections)), display: formatFinance(summary.collections, summary?.currency) },
      { key: 'receivables', label: t('dash.receivables'), value: Math.abs(num(summary.receivables)), display: formatFinance(summary.receivables, summary?.currency) },
      { key: 'commissions', label: t('dash.commissions'), value: Math.abs(num(summary.commission_total)), display: formatFinance(summary.commission_total, summary?.currency) },
      { key: 'expenses', label: t('dash.expenses'), value: Math.abs(num(summary.expense_total)), display: formatFinance(summary.expense_total, summary?.currency) },
      { key: 'payments', label: t('dash.payments'), value: Math.abs(num(summary.payment_total)), display: formatFinance(summary.payment_total, summary?.currency) },
      { key: 'net_cash', label: t('dash.net-cash'), value: Math.abs(num(summary.net_cash)), display: formatFinance(summary.net_cash, summary?.currency) }
    ].filter((row) => row.value > 0 || row.key === 'net_cash');
  }, [lang, summary]);

  const invoiceStatusSlices = useMemo(() => toSlices(mapAggLabels(aggRows(agg.invoice_statuses), (raw) =>
    STATUS_OPTIONS.find((s) => s.id === raw)?.label || raw
  )), [lang, agg]);

  const expenseCategoryBars = useMemo(() => aggRows(agg.expense_categories)
    .filter((row) => row.value > 0)
    .map((row) => ({
      label: EXPENSE_CATEGORIES.find((c) => c.id === row.label)?.label
        || (row.label === '—' ? t('dash.uncategorized') : row.label),
      value: row.value,
      display: formatFinance(row.value)
    })), [lang, agg]);

  const brochureSlices = useMemo(() => toSlices(mapAggLabels(aggRows(agg.brochure_templates), (raw) =>
    brochureTemplateLabel(raw)
  )), [lang, agg]);

  // الخط الزمني: نقطتان جاهزتان من SQLite بحد 90 نقطة (بدل رسم كل سجل خام)
  const trend = useMemo(() => {
    const trendAgg = agg.trend || {};
    const listingsRows = aggRows(trendAgg.listings);
    const marketRows = aggRows(trendAgg.market_records);
    const fmt = (v: string) => (trendAgg.granularity === 'month' ? v : shortDay(v));
    const keys: string[] = [];
    const seen = new Set<string>();
    for (const row of [...listingsRows, ...marketRows]) {
      if (!seen.has(row.label)) {
        seen.add(row.label);
        keys.push(row.label);
      }
    }
    keys.sort();
    const pick = (rows: AggRow[]) => {
      const map = new Map(rows.map((r) => [r.label, r.value]));
      return keys.map((k) => map.get(k) || 0);
    };
    return {
      labels: keys.map(fmt),
      listings: pick(listingsRows),
      market: pick(marketRows)
    };
  }, [agg]);

  const trendDates = trend.labels;
  const propertySeries = trend.listings;
  const marketSeries = trend.market;

  const propertyTotal = num(p.total ?? properties.total);
  const clientTotal = num(stats?.clients?.total ?? clients.total);
  const activePct = propertyTotal > 0 ? (num(p.available) / propertyTotal) * 100 : null;
  const inquiryPct = campaignTotals.inquiry_rate == null ? null : Number(campaignTotals.inquiry_rate) * 100;
  const collectPct = summary && num(summary.invoice_total) > 0
    ? (num(summary.collections) / num(summary.invoice_total)) * 100
    : null;

  const officeHasData = officeBars.some((row) => row.value > 0);
  const funnelHasData = campaignFunnel.some((row) => row.value > 0);

  const propertyDelta = useMemo(() => {
    const pts = propertySeries;
    if (pts.length < 2 || pts[0] <= 0) return null;
    return ((pts[pts.length - 1] - pts[0]) / Math.max(pts[0], 1)) * 100;
  }, [propertySeries]);

  const marketPulse = useMemo<PulseTickerItem[]>(() => {
    return marketPriceBars.slice(0, 10).map((row) => ({
      id: row.label,
      label: row.label,
      salePerSqm: row.value,
      dealsCount: marketCityBars.find((c) => c.label === row.label)?.value ?? null,
      dealsLabel: t('market.record')
    }));
  }, [lang, marketPriceBars, marketCityBars]);

  void officeHasData;
  void clientFollowBars;

  return (
    <div className="dash-visual">
      {statsError ? <div className="error-alert">{statsError}</div> : null}
      {summaryError ? <div className="error-alert">{summaryError}</div> : null}
      {aggregatesError ? <div className="error-alert">{aggregatesError}</div> : null}

      {/* الترتيب المعتمد: التقويم (OfficeAgenda في Dashboard) ← نبض المؤشر ← مؤشرات لوحة العمل ← باقي البيانات */}
      {marketPulse.length > 0 && (
        <MarketPulseTicker items={marketPulse} currency={currencySymbol()} ariaLabel={t('dash.pulse')} />
      )}

      <SectionHairlineTitle
        title={t('dash.kpis')}
        subtitle={t('dash.kpis-sub')}
        icon={LayoutDashboard}
      />
      <div className="ind-kpi-grid" style={{ marginBottom: 14 }}>
        <IndicatorKpi
          label={t('dash.listings')}
          value={propertyTotal.toLocaleString('en-US')}
          icon={Building2}
          tone="primary"
          hint={tf('dash.active-hint', { n: num(p.available) })}
          delta={propertyDelta}
        />
        <IndicatorKpi
          label={t('dash.clients')}
          value={clientTotal.toLocaleString('en-US')}
          icon={Users}
          tone="gold"
          hint={tf('dash.followup-hint', { n: num(stats?.clients?.needs_followup) })}
        />
        <IndicatorKpi
          label={t('dash.campaign-views')}
          value={num(campaignTotals.views).toLocaleString('en-US')}
          icon={Eye}
          tone="primary"
          hint={tf('dash.inquiry-hint', { n: num(campaignTotals.inquiries).toLocaleString('en-US') })}
        />
        <IndicatorKpi
          label={t('dash.collections')}
          value={formatFinance(summary?.collections, summary?.currency)}
          icon={Wallet}
          tone="gold"
          hint={summary ? tf('dash.net-hint', { n: formatFinance(summary.net_cash, summary?.currency) }) : undefined}
        />
      </div>

      {/* بطاقات العقود — نفس مكوّن بطاقات العروض/العملاء/الحملات/التحصيل (IndicatorKpi) دون أي نمط جديد */}
      <div className="ind-kpi-grid" style={{ marginBottom: 14 }} aria-label="إحصائيات العقود">
        <IndicatorKpi
          label="إجمالي العقود"
          value={String(stats?.contracts?.total ?? 0)}
          icon={FileText}
          tone="primary"
        />
        <IndicatorKpi
          label="العقود النشطة"
          value={String(stats?.contracts?.active ?? 0)}
          icon={FileCheck2}
          tone="primary"
        />
        <IndicatorKpi
          label="العقود المنتهية"
          value={String(stats?.contracts?.expired ?? 0)}
          icon={Archive}
          tone="primary"
        />
        <IndicatorKpi
          label="تنتهي قريبًا"
          value={String(stats?.contracts?.ending_soon ?? 0)}
          icon={Clock3}
          tone="gold"
        />
        <IndicatorKpi
          label="مسودات/غير موثقة"
          value={String(stats?.contracts?.drafts ?? 0)}
          icon={FileWarning}
          tone="gold"
        />
        <IndicatorKpi
          label="إجمالي عمولات العقود"
          value={formatFinance(stats?.contracts?.commission_total)}
          icon={HandCoins}
          tone="primary"
        />
      </div>

      {(activePct != null || inquiryPct != null || collectPct != null) && (
        <div className="content-card dash-chart-card dash-radial-card">
          <SectionHairlineTitle title={t('dash.ratio')} icon={Percent} />
          <ApexRadialSet
            height={250}
            items={[
              ...(activePct != null ? [{ label: t('dash.available-pct'), percent: activePct, color: '#23365f' }] : []),
              ...(inquiryPct != null ? [{ label: t('dash.inquiry-rate'), percent: inquiryPct, color: '#d97706' }] : []),
              ...(collectPct != null ? [{ label: t('dash.collect-pct'), percent: collectPct, color: '#b45309' }] : [])
            ]}
          />
        </div>
      )}

      <SectionHairlineTitle title={t('dash.trend')} icon={TrendingUp} />
      <div className="dash-trend-grid">
        <ChartCard title={t('dash.input-motion')} icon={Activity} empty={trendDates.length === 0}>
          <ApexAreaChart
            height={280}
            categories={trendDates}
            series={[
              { name: t('dash.added-listings'), color: '#23365f', data: propertySeries },
              { name: t('dash.market-records'), color: '#d97706', data: marketSeries }
            ]}
          />
        </ChartCard>
        <ChartCard title={t('dash.by-purpose')} icon={Target} empty={propertyPurposeBars.length === 0}>
          <ApexBarChart
            height={280}
            categories={propertyPurposeBars.map((r) => r.label)}
            series={[{ name: t('dash.series-listings'), data: propertyPurposeBars.map((r) => r.value), color: '#23365f' }]}
          />
        </ChartCard>
        <ChartCard title={t('dash.listing-statuses')} icon={PieChart} empty={propertyStatusSlices.length === 0}>
          <ApexDonutChart
            height={280}
            labels={propertyStatusSlices.map((s) => s.label)}
            values={propertyStatusSlices.map((s) => s.value)}
            centerLabel={t('dash.center-listing')}
          />
        </ChartCard>
      </div>

      <VisualBlock title={t('dash.market-campaigns')} icon={BarChart3}>
        <ChartCard title={t('dash.avg-ppm-city')} icon={MapPinned} empty={marketPriceBars.length === 0}>
          <ApexBarChart
            height={280}
            horizontal
            categories={marketPriceBars.map((r) => r.label)}
            series={[{ name: t('dash.series-ppm'), data: marketPriceBars.map((r) => r.value), color: '#d97706' }]}
            valueFormatter={(v) => `${Math.round(v).toLocaleString('en-US')}`}
          />
        </ChartCard>
        <ChartCard title={t('dash.campaign-perf')} icon={Megaphone} empty={campaignCompare.length === 0}>
          <ApexBarChart
            height={280}
            horizontal
            categories={campaignCompare.map((r) => r.label)}
            series={[
              { name: t('dash.series-inquiry'), data: campaignCompare.map((r) => r.values.inquiries), color: '#d97706' },
              { name: t('dash.series-viewing'), data: campaignCompare.map((r) => r.values.viewings), color: '#23365f' },
              { name: t('dash.series-deal'), data: campaignCompare.map((r) => r.values.deals), color: '#b45309' }
            ]}
          />
        </ChartCard>
        <ChartCard title={clientConversionStagesTitle()} icon={Filter} empty={!funnelHasData}>
          <FunnelChart stages={campaignFunnel} />
        </ChartCard>
        <ChartCard title={t('dash.inquiries-by-platform')} icon={Share2} empty={campaignPlatformSlices.length === 0}>
          <ApexDonutChart
            height={280}
            labels={campaignPlatformSlices.map((s) => s.label)}
            values={campaignPlatformSlices.map((s) => s.value)}
            centerLabel={t('dash.center-inquiry')}
          />
        </ChartCard>
      </VisualBlock>

      <VisualBlock title={t('dash.clients-finance')} icon={CircleDollarSign}>
        <ChartCard title={t('dash.client-statuses')} icon={Users} empty={clientStatusSlices.length === 0}>
          <ApexDonutChart
            height={280}
            labels={clientStatusSlices.map((s) => s.label)}
            values={clientStatusSlices.map((s) => s.value)}
            centerLabel={t('dash.center-client')}
          />
        </ChartCard>
        <ChartCard title={t('dash.client-roles')} icon={UserCog} empty={clientRoleBars.length === 0}>
          <ApexBarChart
            height={280}
            horizontal
            categories={clientRoleBars.map((r) => r.label)}
            series={[{ name: t('dash.series-clients'), data: clientRoleBars.map((r) => r.value), color: '#23365f' }]}
          />
        </ChartCard>
        <ChartCard title={t('dash.finance-amounts')} icon={Banknote} empty={financeBars.filter((row) => row.value > 0).length === 0}>
          <ApexBarChart
            height={280}
            horizontal
            categories={financeBars.filter((row) => row.value > 0).map((r) => r.label)}
            series={[{
              name: t('dash.series-amount'),
              data: financeBars.filter((row) => row.value > 0).map((r) => r.value),
              color: '#d97706'
            }]}
            valueFormatter={(v) => formatFinance(v)}
          />
        </ChartCard>
        <ChartCard title={t('dash.invoice-statuses')} icon={Receipt} empty={invoiceStatusSlices.length === 0}>
          <ApexDonutChart
            height={280}
            labels={invoiceStatusSlices.map((s) => s.label)}
            values={invoiceStatusSlices.map((s) => s.value)}
            centerLabel={t('dash.center-invoice')}
          />
        </ChartCard>
        <ChartCard title={t('dash.expenses-by-cat')} icon={Tags} empty={expenseCategoryBars.length === 0}>
          <ApexBarChart
            height={280}
            horizontal
            categories={expenseCategoryBars.map((r) => r.label)}
            series={[{ name: t('dash.series-expense'), data: expenseCategoryBars.map((r) => r.value), color: '#b45309' }]}
            valueFormatter={(v) => formatFinance(v)}
          />
        </ChartCard>
        <ChartCard title={t('dash.designs-by-template')} icon={LayoutTemplate} empty={brochureSlices.length === 0}>
          <ApexDonutChart
            height={280}
            labels={brochureSlices.map((s) => s.label)}
            values={brochureSlices.map((s) => s.value)}
            centerLabel={t('dash.center-file')}
          />
        </ChartCard>
      </VisualBlock>
    </div>
  );
}
