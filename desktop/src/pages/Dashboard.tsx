import React, { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { api } from '../api';
import OfficeAgenda from '../components/OfficeAgenda';
import { getAppTitle } from '../title';
import {
  CLIENT_STATUSES,
  formatMoney,
  propertyStatusLabel,
  purposeLabel,
  roleLabel
} from '../domain/labels';
import { formatCount, formatMoneyAmount, formatDateStamp, platformLabel } from '../campaigns';
import { formatMoneyAmount as formatFinance, statusLabel } from '../finance';
import { templateDisplayName } from '../offer-design/templates-v2';
import { getTemplate, type TemplateId } from '../offer-design/templates';
import DashboardVisual from './DashboardVisual';
import PaginationBar, { usePagedItems } from '../components/PaginationBar';
import SectionGuideBanner from '../components/SectionGuide';
import { useLocale } from '../hooks/useLocale';
import { t as tLocale } from '../domain/locale';
import { getCountry } from '../domain/countryProfile';
import type { LucideIcon } from 'lucide-react';
import {
  LayoutDashboard,
  Building2,
  Users,
  TrendingUp,
  Megaphone,
  Receipt,
  Briefcase,
  Percent,
  Wallet,
  CreditCard,
  FolderOpen,
  ClipboardList,
  BarChart3,
  Table2
} from 'lucide-react';

const LIST_LIMIT = 200;

type ListPack = { items: any[]; total: number; error: string };

function emptyList(): ListPack {
  return { items: [], total: 0, error: '' };
}

function asList(raw: any): ListPack {
  const items = Array.isArray(raw) ? raw : (raw?.items || []);
  const total = Array.isArray(raw) ? raw.length : Number(raw?.total ?? items.length);
  return { items, total: Number.isFinite(total) ? total : items.length, error: '' };
}

async function loadList(path: string): Promise<ListPack> {
  try {
    return asList(await api(path));
  } catch {
    return { items: [], total: 0, error: tLocale('load_failed') };
  }
}

function cell(value: unknown): string {
  if (value === null || value === undefined || value === '') return '—';
  return String(value);
}

function clientStatusLabel(value?: string | null): string {
  return CLIENT_STATUSES.find((s) => s.value === value)?.label || value || '—';
}

function brochureTemplateLabel(id?: string | null): string {
  if (!id) return '—';
  const named = templateDisplayName(id);
  if (named !== id) return named;
  const v1 = getTemplate(id as TemplateId);
  if (v1.id === id) return v1.name;
  return id;
}

function clientRoles(item: any): string {
  const roles: string[] = Array.isArray(item.roles) && item.roles.length
    ? item.roles
    : (item.relationship_type ? [item.relationship_type] : []);
  if (!roles.length) return '—';
  return roles.map((r) => roleLabel(r)).join('، ');
}

function DashSection({
  title,
  total,
  shown,
  to,
  error,
  icon: Icon,
  children
}: {
  title: string;
  total: number;
  shown: number;
  to: string;
  error: string;
  icon?: LucideIcon;
  children: React.ReactNode;
}) {
  const navigate = useNavigate();
  const { t, tf } = useLocale();
  const hint = error
    ? ''
    : shown < total
      ? tf('dash.shown-of', { shown, total })
      : tf('dash.n-records', { n: total });
  return (
    <section className="content-card dash-section">
      <div className="dash-section-head">
        <div>
          <div className="dash-section-title">
            {Icon ? <Icon size={17} strokeWidth={1.8} aria-hidden /> : null}
            {title}
          </div>
          {hint ? <div className="dash-section-hint">{hint}</div> : null}
        </div>
        <button type="button" className="btn btn-outline btn-sm" onClick={() => navigate(to)}>
          {t('dash.view-all')}
        </button>
      </div>
      {error ? <div className="error-alert">{error}</div> : children}
    </section>
  );
}

function EmptyRows({ colSpan }: { colSpan: number }) {
  const { t } = useLocale();
  return (
    <tr>
      <td colSpan={colSpan} className="dash-empty">{t('dash.empty-section')}</td>
    </tr>
  );
}

function DashRecords({
  title,
  to,
  error,
  items,
  apiTotal,
  colSpan,
  label,
  head,
  icon,
  renderRow
}: {
  title: string;
  to: string;
  error: string;
  items: any[];
  apiTotal: number;
  colSpan: number;
  label: string;
  head: React.ReactNode;
  icon?: LucideIcon;
  renderRow: (item: any) => React.ReactNode;
}) {
  const paged = usePagedItems(items);
  return (
    <DashSection title={title} total={apiTotal} shown={paged.slice.length} to={to} error={error} icon={icon}>
      <div className="table-container">
        <table className="data-table">
          <thead>{head}</thead>
          <tbody>
            {paged.slice.length === 0 ? <EmptyRows colSpan={colSpan} /> : paged.slice.map(renderRow)}
          </tbody>
        </table>
      </div>
      <PaginationBar total={paged.total} page={paged.page} onPage={paged.setPage} label={label} />
    </DashSection>
  );
}

export default function Dashboard() {
  const { t, tf } = useLocale();
  const [stats, setStats] = useState<any>(null);
  const [statsError, setStatsError] = useState('');
  const [summary, setSummary] = useState<any>(null);
  const [summaryError, setSummaryError] = useState('');
  const [properties, setProperties] = useState<ListPack>(emptyList());
  const [clients, setClients] = useState<ListPack>(emptyList());
  const [market, setMarket] = useState<ListPack>(emptyList());
  const [campaigns, setCampaigns] = useState<ListPack>(emptyList());
  const [invoices, setInvoices] = useState<ListPack>(emptyList());
  const [operations, setOperations] = useState<ListPack>(emptyList());
  const [commissions, setCommissions] = useState<ListPack>(emptyList());
  const [expenses, setExpenses] = useState<ListPack>(emptyList());
  const [payments, setPayments] = useState<ListPack>(emptyList());
  const [brochures, setBrochures] = useState<ListPack>(emptyList());
  const [campaignStats, setCampaignStats] = useState<any>(null);
  const [aggregates, setAggregates] = useState<any>(null);
  const [aggregatesError, setAggregatesError] = useState('');
  const [tableLimit] = useState<50 | 100>(100);
  const [pane, setPane] = useState<'visual' | 'tables'>('visual');
  const [loading, setLoading] = useState(true);
  const [officeName, setOfficeName] = useState(getAppTitle());
  const navigate = useNavigate();

  useEffect(() => {
    loadData();
  }, [tableLimit]);

  useEffect(() => {
    const onTitle = (e: Event) => {
      const title = (e as CustomEvent).detail;
      if (typeof title === 'string' && title.trim()) setOfficeName(title.trim());
    };
    window.addEventListener('mawasib:title', onTitle);
    return () => window.removeEventListener('mawasib:title', onTitle);
  }, []);

  const loadData = async () => {
    setLoading(true);
    const [
      statsRes,
      summaryRes,
      propertiesRes,
      clientsRes,
      marketRes,
      campaignsRes,
      invoicesRes,
      operationsRes,
      commissionsRes,
      expensesRes,
      paymentsRes,
      brochuresRes,
      campaignStatsRes,
      aggregatesRes
    ] = await Promise.all([
      api('/api/dashboard/stats').then((d) => ({ data: d, error: '' })).catch(() => ({ data: null, error: tLocale('dash.load-summary-fail') })),
      api('/api/finance/summary').then((d) => ({ data: d, error: '' })).catch(() => ({ data: null, error: tLocale('dash.load-finance-fail') })),
      loadList(`/api/properties?limit=${LIST_LIMIT}`),
      loadList(`/api/clients?limit=${LIST_LIMIT}`),
      loadList(`/api/market?limit=${LIST_LIMIT}`),
      loadList(`/api/campaigns?limit=${LIST_LIMIT}`),
      loadList(`/api/finance/invoices?limit=${tableLimit}`),
      loadList(`/api/finance/operations?limit=${tableLimit}`),
      loadList(`/api/finance/commissions?limit=${tableLimit}`),
      loadList(`/api/finance/expenses?limit=${tableLimit}`),
      loadList(`/api/finance/payments?limit=${tableLimit}`),
      loadList(`/api/brochures?limit=${tableLimit}`),
      api('/api/campaigns/stats').then((d) => ({ data: d, error: '' })).catch(() => ({ data: null, error: tLocale('dash.load-campaigns-fail') })),
      api(`/api/dashboard/aggregates?country=${encodeURIComponent(getCountry())}`).then((d) => ({ data: d, error: '' })).catch(() => ({ data: null, error: tLocale('dash.load-aggregates-fail') }))
    ]);

    setStats(statsRes.data);
    setStatsError(statsRes.error);
    setSummary(summaryRes.data);
    setSummaryError(summaryRes.error);
    setProperties(propertiesRes);
    setClients(clientsRes);
    setMarket(marketRes);
    setCampaigns(campaignsRes);
    setInvoices(invoicesRes);
    setOperations(operationsRes);
    setCommissions(commissionsRes);
    setExpenses(expensesRes);
    setPayments(paymentsRes);
    setBrochures(brochuresRes);
    setCampaignStats(campaignStatsRes.data);
    setAggregates(aggregatesRes.data);
    setAggregatesError(aggregatesRes.error);
    setLoading(false);
  };

  const p = stats?.properties || {};
  const c = stats?.clients || {};

  const overviewRows = [
    { section: t('dash.listings'), count: p.total, extra: tf('dash.extra-listings', { a: p.available || 0, n: p.negotiating || 0, r: p.reserved || 0, s: p.sold || 0 }) },
    { section: t('clients'), count: c.total, extra: tf('dash.extra-clients', { f: c.needs_followup || 0, c: stats?.communications || 0 }) },
    { section: t('dash.market-rows'), count: stats?.market_rows, extra: '' },
    { section: t('campaigns'), count: stats?.campaigns, extra: '' },
    { section: t('dash.invoices'), count: stats?.finance_invoices, extra: summary ? tf('dash.extra-finance', { col: formatFinance(summary.collections, summary.currency), rec: formatFinance(summary.receivables, summary.currency), net: formatFinance(summary.net_cash, summary.currency) }) : '' },
    { section: t('dash.sales-ops'), count: operations.total, extra: '' },
    { section: t('dash.commissions'), count: commissions.total, extra: '' },
    { section: t('dash.expenses'), count: expenses.total, extra: summary ? tf('dash.extra-expenses', { n: formatFinance(summary.expense_total, summary.currency) }) : '' },
    { section: t('dash.payments'), count: payments.total, extra: '' },
    { section: t('dash.brochures'), count: stats?.brochures, extra: '' },
    { section: t('dash.sent-offers'), count: stats?.sent_offers, extra: '' },
    { section: t('dash.registered-sales'), count: stats?.sales, extra: '' }
  ];

  if (loading) {
    return <div className="content-card">{t('dash.loading-office')}</div>;
  }

  const failed = statsError || summaryError || properties.error || clients.error || market.error || campaigns.error || invoices.error || operations.error || commissions.error || expenses.error || payments.error || brochures.error;

  return (
    <div className="filing-dashboard dash-board">
      {failed && <div className="error-alert" role="alert"><button type="button" className="btn btn-outline" onClick={loadData}>{t('retry')}</button></div>}
      <header className="filing-masthead">
        <div className="filing-masthead-line" aria-hidden="true" />
        <div className="filing-masthead-core">
          <div className="filing-kicker">
            <LayoutDashboard size={15} strokeWidth={1.8} aria-hidden />
            {t('dashboard')} <SectionGuideBanner id="dashboard" />
          </div>
          <h1 className="filing-title">{officeName}</h1>
          <p className="filing-sub">{t('dash.welcome')}</p>
        </div>
        <div className="filing-masthead-line" aria-hidden="true" />
      </header>

      {stats?.training_data_present ? (
        <div className="training-banner" role="status">{t('training.banner')}</div>
      ) : null}

      <div className="dash-pane-switch" role="group" aria-label={t('dash.pane-aria')}>
        <button
          type="button"
          className={`btn ${pane === 'visual' ? 'btn-primary' : 'btn-outline'}`}
          aria-pressed={pane === 'visual'}
          onClick={() => setPane('visual')}
        >
          <BarChart3 size={15} strokeWidth={1.8} aria-hidden />
          {t('dash.pane-visual')}
        </button>
        <button
          type="button"
          className={`btn ${pane === 'tables' ? 'btn-primary' : 'btn-outline'}`}
          aria-pressed={pane === 'tables'}
          onClick={() => setPane('tables')}
        >
          <Table2 size={15} strokeWidth={1.8} aria-hidden />
          {t('dash.pane-tables')}
        </button>
      </div>

      <OfficeAgenda />

      {pane === 'visual' && (
        <DashboardVisual
          stats={stats}
          statsError={statsError}
          summary={summary}
          summaryError={summaryError}
          campaignStats={campaignStats}
          aggregates={aggregates}
          aggregatesError={aggregatesError}
          properties={properties}
          clients={clients}
          market={market}
          campaigns={campaigns}
          invoices={invoices}
          operations={operations}
          commissions={commissions}
          expenses={expenses}
          payments={payments}
          brochures={brochures}
        />
      )}

      {pane === 'tables' && (
        <>
      <section className="content-card dash-section">
        <div className="dash-section-head">
          <div>
            <div className="dash-section-title">
              <ClipboardList size={17} strokeWidth={1.8} aria-hidden />
              {t('dash.overview')}
            </div>
            <div className="dash-section-hint">{t('dash.overview-hint')}</div>
          </div>
          <button type="button" className="btn btn-primary btn-sm" onClick={() => navigate('/properties/new')}>
            {t('dash.add-listing')}
          </button>
        </div>
        {statsError ? <div className="error-alert">{statsError}</div> : null}
        {summaryError ? <div className="error-alert">{summaryError}</div> : null}
        {!statsError && (
          <div className="table-container">
            <table className="data-table">
              <thead>
                <tr>
                  <th>{t('section')}</th>
                  <th>{t('count')}</th>
                  <th>{t('details')}</th>
                </tr>
              </thead>
              <tbody>
                {overviewRows.map((row) => (
                  <tr key={row.section}>
                    <td>{row.section}</td>
                    <td className="num">{row.count == null ? '—' : Number(row.count).toLocaleString('en-US')}</td>
                    <td>{row.extra || '—'}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>

      <DashRecords
        title={t('dash.listings')}
        icon={Building2}
        to="/properties"
        error={properties.error}
        items={properties.items}
        apiTotal={properties.total}
        colSpan={6}
        label={t('dash.label-listing')}
        head={<tr><th>{t('name')}</th><th>{t('city_district')}</th><th>{t('type')}</th><th>{t('purpose')}</th><th>{t('status')}</th><th>{t('price')}</th></tr>}
        renderRow={(item) => (
          <tr key={item.id}>
            <td>{cell(item.name)}</td>
            <td>{[item.city, item.district].filter(Boolean).join(' / ') || '—'}</td>
            <td>{cell(item.property_type)}</td>
            <td>{purposeLabel(item.purpose)}</td>
            <td>{propertyStatusLabel(item.status)}</td>
            <td className="num">{formatMoney(item.price, item.currency)}</td>
          </tr>
        )}
      />

      <DashRecords
        title={t('clients')}
        icon={Users}
        to="/clients"
        error={clients.error}
        items={clients.items}
        apiTotal={clients.total}
        colSpan={6}
        label={t('dash.label-client')}
        head={<tr><th>{t('name')}</th><th>{t('mobile')}</th><th>{t('roles')}</th><th>{t('status')}</th><th>{t('city')}</th><th>{t('followup')}</th></tr>}
        renderRow={(item) => (
          <tr key={item.id}>
            <td>{cell(item.name)}</td>
            <td className="num">{cell(item.phone)}</td>
            <td>{clientRoles(item)}</td>
            <td>{clientStatusLabel(item.status)}</td>
            <td>{cell(item.city)}</td>
            <td className="num">{cell(item.follow_up_date)}</td>
          </tr>
        )}
      />

      <DashRecords
        title={t('dash.market-rows')}
        icon={TrendingUp}
        to="/market"
        error={market.error}
        items={market.items}
        apiTotal={market.total}
        colSpan={6}
        label={t('dash.label-record')}
        head={<tr><th>{t('city')}</th><th>{t('district')}</th><th>{t('type')}</th><th>{t('area')}</th><th>{t('ppm')}</th><th>{t('date')}</th></tr>}
        renderRow={(item) => (
          <tr key={item.id}>
            <td>{cell(item.city)}</td>
            <td>{cell(item.district)}</td>
            <td>{cell(item.property_type)}</td>
            <td className="num">{item.area ? tf('market.area-m2', { n: item.area }) : '—'}</td>
            <td className="num">{formatMoney(item.price_per_meter_sale, item.currency)}</td>
            <td className="num">{cell(item.entry_date)}</td>
          </tr>
        )}
      />

      <DashRecords
        title={t('campaigns')}
        icon={Megaphone}
        to="/campaigns"
        error={campaigns.error}
        items={campaigns.items}
        apiTotal={campaigns.total}
        colSpan={6}
        label={t('dash.label-campaign')}
        head={<tr><th>{t('campaign_col')}</th><th>{t('platform')}</th><th>{t('property_col')}</th><th>{t('views')}</th><th>{t('inquiries')}</th><th>{t('cost')}</th></tr>}
        renderRow={(item) => (
          <tr key={item.id}>
            <td>{cell(item.name)}</td>
            <td>{platformLabel(item.platform)}</td>
            <td>{cell(item.property_name)}</td>
            <td className="num">{formatCount(item.views)}</td>
            <td className="num">{formatCount(item.inquiries)}</td>
            <td className="num">{formatMoneyAmount(item.cost, item.currency)}</td>
          </tr>
        )}
      />

      <DashRecords
        title={t('dash.invoices')}
        icon={Receipt}
        to="/finance"
        error={invoices.error}
        items={invoices.items}
        apiTotal={invoices.total}
        colSpan={6}
        label={t('dash.label-invoice')}
        head={<tr><th>{t('number_col')}</th><th>{t('title_col')}</th><th>{t('amount')}</th><th>{t('paid')}</th><th>{t('remaining')}</th><th>{t('status')}</th></tr>}
        renderRow={(item) => (
          <tr key={item.id}>
            <td className="num">{cell(item.number)}</td>
            <td>{cell(item.title)}</td>
            <td className="num">{formatFinance(item.amount, item.currency)}</td>
            <td className="num">{formatFinance(item.paid, item.currency)}</td>
            <td className="num">{formatFinance(item.remaining, item.currency)}</td>
            <td>{item.status_label || statusLabel(item.status)}</td>
          </tr>
        )}
      />

      <DashRecords
        title={t('dash.sales-ops')}
        icon={Briefcase}
        to="/finance"
        error={operations.error}
        items={operations.items}
        apiTotal={operations.total}
        colSpan={5}
        label={t('dash.label-operation')}
        head={<tr><th>{t('title_col')}</th><th>{t('deal_value')}</th><th>{t('date')}</th><th>{t('client_col')}</th><th>{t('property_col')}</th></tr>}
        renderRow={(item) => (
          <tr key={item.id}>
            <td>{cell(item.title)}</td>
            <td className="num">{formatFinance(item.deal_value, item.currency)}</td>
            <td className="num">{cell(item.occurs_on)}</td>
            <td>{cell(item.client_name)}</td>
            <td>{cell(item.property_name)}</td>
          </tr>
        )}
      />

      <DashRecords
        title={t('dash.commissions')}
        icon={Percent}
        to="/finance"
        error={commissions.error}
        items={commissions.items}
        apiTotal={commissions.total}
        colSpan={5}
        label={t('dash.label-commission')}
        head={<tr><th>{t('title_col')}</th><th>{t('amount')}</th><th>{t('remaining')}</th><th>{t('status')}</th><th>{t('date')}</th></tr>}
        renderRow={(item) => (
          <tr key={item.id}>
            <td>{cell(item.title)}</td>
            <td className="num">{formatFinance(item.computed_amount ?? item.amount_fixed, item.currency)}</td>
            <td className="num">{formatFinance(item.remaining, item.currency)}</td>
            <td>{item.status_label || statusLabel(item.status)}</td>
            <td className="num">{cell(item.occurs_on)}</td>
          </tr>
        )}
      />

      <DashRecords
        title={t('dash.expenses')}
        icon={Wallet}
        to="/finance"
        error={expenses.error}
        items={expenses.items}
        apiTotal={expenses.total}
        colSpan={4}
        label={t('dash.label-expense')}
        head={<tr><th>{t('title_col')}</th><th>{t('category')}</th><th>{t('amount')}</th><th>{t('date')}</th></tr>}
        renderRow={(item) => (
          <tr key={item.id}>
            <td>{cell(item.title)}</td>
            <td>{cell(item.category)}</td>
            <td className="num">{formatFinance(item.amount, item.currency)}</td>
            <td className="num">{cell(item.occurs_on)}</td>
          </tr>
        )}
      />

      <DashRecords
        title={t('dash.payments')}
        icon={CreditCard}
        to="/finance"
        error={payments.error}
        items={payments.items}
        apiTotal={payments.total}
        colSpan={4}
        label={t('dash.label-payment')}
        head={<tr><th>{t('target')}</th><th>{t('amount')}</th><th>{t('method')}</th><th>{t('date')}</th></tr>}
        renderRow={(item) => (
          <tr key={item.id}>
            <td>{cell(item.target_title)}</td>
            <td className="num">{formatFinance(item.amount, item.currency)}</td>
            <td>{cell(item.method)}</td>
            <td className="num">{cell(item.occurs_on)}</td>
          </tr>
        )}
      />

      <DashRecords
        title={t('dash.brochures')}
        icon={FolderOpen}
        to="/brochures"
        error={brochures.error}
        items={brochures.items}
        apiTotal={brochures.total}
        colSpan={4}
        label={t('dash.label-item')}
        head={<tr><th>{t('title_col')}</th><th>{t('property_col')}</th><th>{t('template')}</th><th>{t('date')}</th></tr>}
        renderRow={(item) => (
          <tr key={item.id}>
            <td>{cell(item.title)}</td>
            <td>{cell(item.property_name)}</td>
            <td>{brochureTemplateLabel(item.template_id)}</td>
            <td className="num">{formatDateStamp(item.created_at || item.updated_at)}</td>
          </tr>
        )}
      />
        </>
      )}
    </div>
  );
}
