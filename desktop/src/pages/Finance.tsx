import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { useLocation } from 'react-router-dom';
import { Banknote, Plus, Pencil, Trash2, Printer, TriangleAlert, FilePenLine, FileText, Receipt, Briefcase, Percent, Wallet, CreditCard } from 'lucide-react';
import { api } from '../api';
import {
  EXPENSE_CATEGORIES,
  FINANCE_TABS,
  LEDGER_KINDS,
  PAYMENT_METHODS,
  STATUS_OPTIONS,
  formatMoneyAmount,
  periodQuery,
  todayIso,
  type TabId
} from '../finance';
import PaginationBar from '../components/PaginationBar';
import { slicePage, TABLE_PAGE_SIZE } from '../pagination';
import { noAutofill } from '../noAutofill';
import SectionGuideBanner from '../components/SectionGuide';
import { useLocale } from '../hooks/useLocale';
import { currencySymbol, t as tLocale } from '../domain/locale';
import ContractsTab from '../components/ContractsTab';
import AppContextMenu, { type AppMenuItem } from '../components/AppContextMenu';

function RelSelects({
  form,
  setForm,
  clients,
  properties,
  campaigns,
  operations
}: {
  form: any;
  setForm: (next: any) => void;
  clients: any[];
  properties: any[];
  campaigns: any[];
  operations?: any[];
}) {
  const { t } = useLocale();
  return (
    <>
      <div className="form-group">
        <label>{t('client_col')}</label>
        <select className="form-input" autoComplete="off" value={form.client_id || ''} onChange={(e) => setForm({ ...form, client_id: e.target.value })}>
          <option value="">{t('none_link')}</option>
          {clients.map((c) => <option key={c.id} value={c.id}>{c.name}</option>)}
        </select>
      </div>
      <div className="form-group">
        <label>{t('fin.linked-property')}</label>
        <select className="form-input" autoComplete="off" value={form.property_id || ''} onChange={(e) => setForm({ ...form, property_id: e.target.value })}>
          <option value="">{t('none_link')}</option>
          {properties.map((p) => <option key={p.id} value={p.id}>{p.name}</option>)}
        </select>
      </div>
      <div className="form-group">
        <label>{t('campaign_col')}</label>
        <select className="form-input" autoComplete="off" value={form.campaign_id || ''} onChange={(e) => setForm({ ...form, campaign_id: e.target.value })}>
          <option value="">{t('none_link')}</option>
          {campaigns.map((c) => <option key={c.id} value={c.id}>{c.name}</option>)}
        </select>
      </div>
      {operations && (
        <div className="form-group">
          <label>{t('fin.operation-opt')}</label>
          <select className="form-input" autoComplete="off" value={form.operation_id || ''} onChange={(e) => setForm({ ...form, operation_id: e.target.value })}>
            <option value="">{t('none_link')}</option>
            {operations.map((o) => <option key={o.id} value={o.id}>{o.title}</option>)}
          </select>
        </div>
      )}
    </>
  );
}

function Money({ value, currency }: { value: any; currency?: string }) {
  return <span className="num">{formatMoneyAmount(value, currency)}</span>;
}

function StatusBadge({ status, label }: { status?: string; label?: string }) {
  if (!status && !label) return <span>—</span>;
  return <span className={`finance-status finance-status-${status || ''}`}>{label || status}</span>;
}

export default function Finance() {
  const { t, tf } = useLocale();
  const [tab, setTab] = useState<TabId>('overview');
  const [periodFrom, setPeriodFrom] = useState('');
  const [periodTo, setPeriodTo] = useState('');
  const [statusFilter, setStatusFilter] = useState('');
  const [kindFilter, setKindFilter] = useState('');
  const [error, setError] = useState('');
  const [formError, setFormError] = useState('');
  const [saving, setSaving] = useState(false);
  const [loading, setLoading] = useState(false);
  const [summary, setSummary] = useState<any>(null);
  const [contractsSummary, setContractsSummary] = useState<any>(null);
  const [items, setItems] = useState<any[]>([]);
  const [clients, setClients] = useState<any[]>([]);
  const [properties, setProperties] = useState<any[]>([]);
  const [campaigns, setCampaigns] = useState<any[]>([]);
  const [operations, setOperations] = useState<any[]>([]);
  const [invoices, setInvoices] = useState<any[]>([]);
  const [commissions, setCommissions] = useState<any[]>([]);
  const [showForm, setShowForm] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [form, setForm] = useState<any>({});
  const [pdfNote, setPdfNote] = useState('');
  const [page, setPage] = useState(1);
  const [addMenu, setAddMenu] = useState<{ x: number; y: number } | null>(null);
  const [contractsNonce, setContractsNonce] = useState(0);
  const addBtnRef = React.useRef<HTMLButtonElement>(null);
  const location = useLocation() as { state?: { tab?: TabId; newDraft?: boolean } };

  const q = useMemo(() => periodQuery(periodFrom, periodTo), [periodFrom, periodTo]);
  const linked = (form.target_type === 'commission' ? commissions : invoices).find(x => x.id === form.target_id)
    || invoices.find(x => x.id === form.invoice_id) || operations.find(x => x.id === form.operation_id)
    || properties.find(x => x.id === form.property_id) || campaigns.find(x => x.id === form.campaign_id);
  const cur = editingId ? form.currency : linked?.currency || currencySymbol();

  const loadLookups = useCallback(async () => {
    const [c, p, camp, ops, inv, com] = await Promise.all([
      api('/api/clients?limit=200'),
      api('/api/properties?limit=200'),
      api('/api/campaigns?limit=200'),
      api('/api/finance/operations'),
      api('/api/finance/invoices'),
      api('/api/finance/commissions')
    ]);
    setClients(c.items || []);
    setProperties(p.items || []);
    setCampaigns(camp.items || []);
    setOperations(ops.items || []);
    setInvoices(inv.items || []);
    setCommissions(com.items || []);
  }, []);

  const loadTab = useCallback(async () => {
    setLoading(true);
    setError('');
    try {
      const extra: Record<string, string> = {};
      if (statusFilter && (tab === 'invoices' || tab === 'commissions' || tab === 'receivables' || tab === 'ledger')) extra.status = statusFilter;
      if (kindFilter && tab === 'ledger') extra.kind = kindFilter;
      const suffix = periodQuery(periodFrom, periodTo, extra);
      if (tab === 'overview' || tab === 'reports') {
        const s = await api(`/api/finance/summary${q}`);
        setSummary(s);
        setItems([]);
        if (tab === 'overview') {
          try {
            setContractsSummary(await api('/api/contracts/summary'));
          } catch {
            setContractsSummary(null);
            setError(tLocale('fin.load-fail'));
          }
        }
      } else if (tab === 'invoices') {
        const d = await api(`/api/finance/invoices${suffix}`);
        setItems(d.items || []);
      } else if (tab === 'operations') {
        const d = await api(`/api/finance/operations${suffix}`);
        setItems(d.items || []);
      } else if (tab === 'commissions') {
        const d = await api(`/api/finance/commissions${suffix}`);
        setItems(d.items || []);
      } else if (tab === 'expenses') {
        const d = await api(`/api/finance/expenses${suffix}`);
        setItems(d.items || []);
      } else if (tab === 'payments') {
        const d = await api(`/api/finance/payments${suffix}`);
        setItems(d.items || []);
      } else if (tab === 'receivables') {
        const d = await api(`/api/finance/receivables${suffix}`);
        setItems(d.items || []);
      } else if (tab === 'ledger') {
        const d = await api(`/api/finance/ledger${suffix}`);
        setItems(d.items || []);
      }
    } catch (err: any) {
      setError(err?.message || tLocale('fin.load-fail'));
      setItems([]);
    } finally {
      setLoading(false);
    }
  }, [tab, periodFrom, periodTo, statusFilter, kindFilter, q]);

  useEffect(() => {
    loadLookups().catch((err) => setError(err?.message || tLocale('fin.lookups-fail')));
  }, [loadLookups]);

  // نية قادمة من زر + العلوي أو قائمة الزر الأيمن: فتح تبويب العقود مع مسودة جديدة.
  useEffect(() => {
    const st = location.state;
    if (!st?.tab) return;
    setTab(st.tab);
    closeForm();
    if (st.tab === 'contracts' && st.newDraft) setContractsNonce((n) => n + 1);
    window.history.replaceState({}, '');
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    loadTab();
  }, [loadTab]);

  useEffect(() => {
    setPage(1);
  }, [tab, periodFrom, periodTo, statusFilter, kindFilter]);

  const closeForm = () => {
    setShowForm(false);
    setEditingId(null);
    setForm({});
    setFormError('');
  };

  const openCreate = (target: TabId = tab) => {
    const today = todayIso();
    const base: any = { notes: '', client_id: '', property_id: '', campaign_id: '', operation_id: '' };
    if (target === 'invoices') setForm({ ...base, title: '', issue_date: today, due_date: '', amount: '' });
    else if (target === 'operations') setForm({ ...base, title: '', occurs_on: today, deal_value: '' });
    else if (target === 'commissions') {
      setForm({
        ...base,
        title: '',
        occurs_on: today,
        mode: 'percent',
        basis_amount: '',
        rate_percent: '',
        amount_fixed: '',
        office_share_pct: '100',
        broker_share_pct: '0',
        marketer_share_pct: '0',
        partner_share_pct: '0',
        invoice_id: ''
      });
    } else if (target === 'expenses') setForm({ ...base, title: '', occurs_on: today, amount: '', category: 'أخرى', reference: '' });
    else if (target === 'payments') setForm({ occurs_on: today, amount: '', target_type: 'invoice', target_id: '', method: 'تحويل', reference: '', notes: '' });
    else return;
    setEditingId(null);
    setShowForm(true);
    setFormError('');
  };

  const gotoCreate = (target: TabId) => {
    if (target === 'contracts') {
      setTab('contracts');
      closeForm();
      setContractsNonce((n) => n + 1);
      return;
    }
    setTab(target);
    closeForm();
    openCreate(target);
  };

  /** قائمة + المنسدلة: إضافة مرتبة بأيقونات هوية OBR تشمل عقدًا جديدًا. */
  const addMenuItems: AppMenuItem[] = [
    { id: 'new-invoice', label: 'فاتورة جديدة', icon: <Receipt size={15} strokeWidth={1.8} />, onClick: () => gotoCreate('invoices') },
    { id: 'new-operation', label: 'عملية بيع / صفقة جديدة', icon: <Briefcase size={15} strokeWidth={1.8} />, onClick: () => gotoCreate('operations') },
    { id: 'new-commission', label: 'عمولة جديدة', icon: <Percent size={15} strokeWidth={1.8} />, onClick: () => gotoCreate('commissions') },
    { id: 'new-expense', label: 'مصروف جديد', icon: <Wallet size={15} strokeWidth={1.8} />, onClick: () => gotoCreate('expenses') },
    { id: 'new-payment', label: 'دفعة جديدة', icon: <CreditCard size={15} strokeWidth={1.8} />, onClick: () => gotoCreate('payments') },
    { id: 'sep-contracts', separator: true },
    { id: 'new-contract', label: 'عقد جديد', icon: <FileText size={15} strokeWidth={1.8} />, onClick: () => gotoCreate('contracts') }
  ];

  const openAddMenu = () => {
    const el = addBtnRef.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    setAddMenu({ x: Math.max(8, rect.right - 240), y: rect.bottom + 6 });
  };

  const openEdit = (item: any) => {
    setForm({ ...item });
    setEditingId(item.id);
    setShowForm(true);
    setFormError('');
  };

  const endpoint = () => {
    if (tab === 'invoices') return '/api/finance/invoices';
    if (tab === 'operations') return '/api/finance/operations';
    if (tab === 'commissions') return '/api/finance/commissions';
    if (tab === 'expenses') return '/api/finance/expenses';
    if (tab === 'payments') return '/api/finance/payments';
    return '';
  };

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    const path = endpoint();
    if (!path) return;
    setSaving(true);
    setFormError('');
    try {
      const body = { ...form };
      if (editingId) await api(`${path}/${editingId}`, { method: 'PUT', body });
      else await api(path, { method: 'POST', body });
      closeForm();
      await loadLookups();
      await loadTab();
    } catch (err: any) {
      setFormError(err?.message || t('fin.save-fail'));
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (id: string) => {
    const path = endpoint();
    if (!path) return;
    if (!window.confirm(t('fin.delete-confirm'))) return;
    try {
      await api(`${path}/${id}`, { method: 'DELETE' });
      await loadLookups();
      await loadTab();
    } catch (err: any) {
      setError(err?.message || t('fin.delete-fail'));
    }
  };

  const makeInvoicePdf = async (id: string) => {
    setPdfNote('');
    try {
      const res = await api(`/api/finance/invoices/${id}/pdf`, { method: 'POST', body: { open: true } });
      setPdfNote(tf('fin.pdf-ok', { path: res.pdf_path }));
    } catch (err: any) {
      setError(err?.message || t('fin.pdf-fail'));
    }
  };

  const printRecord = async (item: any) => {
    setPdfNote('');
    try {
      if (tab === 'invoices') {
        await makeInvoicePdf(item.id);
        return;
      }
      const recordKind = tab === 'operations' ? 'operation'
        : tab === 'commissions' ? 'commission'
        : tab === 'expenses' ? 'expense'
        : tab === 'payments' ? 'payment'
        : tab === 'receivables' ? `receivable_${item.kind}`
        : item.kind;
      const res = await api('/api/finance/record/pdf', { method: 'POST', body: { record_kind: recordKind, id: item.id, open: true } });
      setPdfNote(tf('fin.pdf-ok', { path: res.pdf_path }));
    } catch (err: any) {
      setError(err?.message || t('fin.pdf-fail'));
    }
  };

  const makeReportPdf = async () => {
    setPdfNote('');
    try {
      const res = await api('/api/finance/report/pdf', {
        method: 'POST',
        body: { period_from: periodFrom, period_to: periodTo, open: true }
      });
      setPdfNote(tf('fin.report-ok', { path: res.pdf_path }));
    } catch (err: any) {
      setError(err?.message || t('fin.report-fail'));
    }
  };

  const crudTabs = tab === 'invoices' || tab === 'operations' || tab === 'commissions' || tab === 'expenses' || tab === 'payments';
  const contextualAddLabel = tab === 'operations' ? '+ عملية جديدة'
    : tab === 'invoices' ? '+ فاتورة جديدة'
    : tab === 'commissions' ? '+ عمولة جديدة'
    : tab === 'expenses' ? '+ مصروف جديد'
    : tab === 'payments' ? '+ دفعة جديدة'
    : '';
  const pagedItems = slicePage(items, page, TABLE_PAGE_SIZE);

  return (
    <div>
      <div className="card-header">
        <div className="card-title">
          <Banknote size={17} strokeWidth={1.8} style={{ verticalAlign: '-3px' }} /> {t('fin.title')} <SectionGuideBanner id="finance" />
        </div>
        <button ref={addBtnRef} className="btn btn-primary" type="button" aria-haspopup="menu" aria-expanded={addMenu != null} onClick={() => (addMenu ? setAddMenu(null) : openAddMenu())} title="إضافة جديدة">
          <Plus size={15} strokeWidth={1.8} /> إضافة
        </button>
        {addMenu && <AppContextMenu x={addMenu.x} y={addMenu.y} items={addMenuItems} onClose={() => setAddMenu(null)} />}
      </div>

      <div className="alert-note">
        <TriangleAlert size={14} strokeWidth={1.8} style={{ verticalAlign: '-2px' }} />
        {' '}{t('fin.hint')}
      </div>

      <div className="finance-toolbar">
        <label>
          {t('from_date')}
          <input className="form-input" autoComplete="off" type="date" value={periodFrom} onChange={(e) => setPeriodFrom(e.target.value)} />
        </label>
        <label>
          {t('to_date')}
          <input className="form-input" autoComplete="off" type="date" value={periodTo} onChange={(e) => setPeriodTo(e.target.value)} />
        </label>
        {(tab === 'invoices' || tab === 'commissions' || tab === 'receivables' || tab === 'ledger') && (
          <label>
            {t('status')}
            <select className="form-input" autoComplete="off" value={statusFilter} onChange={(e) => setStatusFilter(e.target.value)}>
              <option value="">{t('all')}</option>
              {STATUS_OPTIONS.map((s) => <option key={s.id} value={s.id}>{s.label}</option>)}
            </select>
          </label>
        )}
        {tab === 'ledger' && (
          <label>
            {t('type')}
            <select className="form-input" autoComplete="off" value={kindFilter} onChange={(e) => setKindFilter(e.target.value)}>
              <option value="">{t('all')}</option>
              {LEDGER_KINDS.map((s) => <option key={s.id} value={s.id}>{s.label}</option>)}
            </select>
          </label>
        )}
      </div>

      <div className="finance-tabs" role="tablist">
        {FINANCE_TABS.map((tabItem) => (
          <button
            key={tabItem.id}
            type="button"
            role="tab"
            aria-selected={tab === tabItem.id}
            className={tab === tabItem.id ? 'is-active' : ''}
            onClick={() => { setTab(tabItem.id as TabId); closeForm(); }}
          >
            {tabItem.label}
          </button>
        ))}
      </div>

      {crudTabs && (
        <div className="finance-contextual-actions">
          <button className="btn btn-primary" type="button" onClick={() => openCreate(tab)}>
            <Plus size={15} strokeWidth={1.8} /> {contextualAddLabel}
          </button>
        </div>
      )}

      {error && <div className="error-alert" role="alert">{error} <button type="button" className="btn btn-outline" onClick={() => void loadTab()}>{t('retry')}</button></div>}
      {pdfNote && <div className="alert-note">{pdfNote}</div>}
      {loading && <p className="muted">{t('loading')}</p>}

      {showForm && crudTabs && (
        <div className="content-card obr-panel">
          <div className="card-header" style={{ marginTop: 0 }}>
            <div className="card-title"><FilePenLine size={17} strokeWidth={1.8} style={{ verticalAlign: '-3px' }} /> {editingId ? t('edit') : t('new_record')}</div>
          </div>
          {formError && <div className="error-alert">{formError}</div>}
          <form {...noAutofill} onSubmit={handleSave}>
            <div className="form-grid">
              {tab !== 'payments' && (
                <div className="form-group">
                  <label>{t('title_col')}</label>
                  <input className="form-input" autoComplete="off" required minLength={2} value={form.title || ''} onChange={(e) => setForm({ ...form, title: e.target.value })} />
                </div>
              )}
              {tab === 'invoices' && (
                <>
                  <div className="form-group">
                    <label>{t('fin.issue-date')}</label>
                    <input className="form-input" autoComplete="off" type="date" required value={form.issue_date || ''} onChange={(e) => setForm({ ...form, issue_date: e.target.value })} />
                  </div>
                  <div className="form-group">
                    <label>{t('fin.due-date')}</label>
                    <input className="form-input" autoComplete="off" type="date" value={form.due_date || ''} onChange={(e) => setForm({ ...form, due_date: e.target.value })} />
                  </div>
                  <div className="form-group">
                    <label>{tf('amount_cur', { cur })}</label>
                    <input className="form-input" autoComplete="off" type="number" min={0.01} step="0.01" required value={form.amount ?? ''} onChange={(e) => setForm({ ...form, amount: e.target.value })} />
                  </div>
                  <RelSelects form={form} setForm={setForm} clients={clients} properties={properties} campaigns={campaigns} operations={operations} />
                </>
              )}
              {tab === 'operations' && (
                <>
                  <div className="form-group">
                    <label>{t('date')}</label>
                    <input className="form-input" autoComplete="off" type="date" required value={form.occurs_on || ''} onChange={(e) => setForm({ ...form, occurs_on: e.target.value })} />
                  </div>
                  <div className="form-group">
                    <label>{tf('fin.deal-value-cur', { cur })}</label>
                    <input className="form-input" autoComplete="off" type="number" min={0} step="0.01" value={form.deal_value ?? ''} onChange={(e) => setForm({ ...form, deal_value: e.target.value })} />
                  </div>
                  <RelSelects form={form} setForm={setForm} clients={clients} properties={properties} campaigns={campaigns} />
                </>
              )}
              {tab === 'commissions' && (
                <>
                  <div className="form-group">
                    <label>{t('date')}</label>
                    <input className="form-input" autoComplete="off" type="date" required value={form.occurs_on || ''} onChange={(e) => setForm({ ...form, occurs_on: e.target.value })} />
                  </div>
                  <div className="form-group">
                    <label>{t('fin.comm-mode')}</label>
                    <select className="form-input" autoComplete="off" value={form.mode || 'percent'} onChange={(e) => setForm({ ...form, mode: e.target.value })}>
                      <option value="percent">{t('fin.comm-percent')}</option>
                      <option value="fixed">{t('fin.comm-fixed')}</option>
                    </select>
                  </div>
                  {form.mode !== 'fixed' && (
                    <>
                      <div className="form-group">
                        <label>{tf('fin.basis-cur', { cur })}</label>
                        <input className="form-input" autoComplete="off" type="number" min={0} step="0.01" value={form.basis_amount ?? ''} onChange={(e) => setForm({ ...form, basis_amount: e.target.value })} />
                      </div>
                      <div className="form-group">
                        <label>{t('fin.rate-pct')}</label>
                        <input className="form-input" autoComplete="off" type="number" min={0} max={100} step="0.01" value={form.rate_percent ?? ''} onChange={(e) => setForm({ ...form, rate_percent: e.target.value })} />
                      </div>
                    </>
                  )}
                  {form.mode === 'fixed' && (
                    <div className="form-group">
                      <label>{tf('fin.comm-amount-cur', { cur })}</label>
                      <input className="form-input" autoComplete="off" type="number" min={0} step="0.01" value={form.amount_fixed ?? ''} onChange={(e) => setForm({ ...form, amount_fixed: e.target.value })} />
                    </div>
                  )}
                  <div className="form-group">
                    <label>{t('fin.office-share')}</label>
                    <input className="form-input" autoComplete="off" type="number" min={0} max={100} step="0.01" value={form.office_share_pct ?? ''} onChange={(e) => setForm({ ...form, office_share_pct: e.target.value })} />
                  </div>
                  <div className="form-group">
                    <label>{t('fin.broker-share')}</label>
                    <input className="form-input" autoComplete="off" type="number" min={0} max={100} step="0.01" value={form.broker_share_pct ?? ''} onChange={(e) => setForm({ ...form, broker_share_pct: e.target.value })} />
                  </div>
                  <div className="form-group">
                    <label>{t('fin.marketer-share')}</label>
                    <input className="form-input" autoComplete="off" type="number" min={0} max={100} step="0.01" value={form.marketer_share_pct ?? ''} onChange={(e) => setForm({ ...form, marketer_share_pct: e.target.value })} />
                  </div>
                  <div className="form-group">
                    <label>{t('fin.partner-share')}</label>
                    <input className="form-input" autoComplete="off" type="number" min={0} max={100} step="0.01" value={form.partner_share_pct ?? ''} onChange={(e) => setForm({ ...form, partner_share_pct: e.target.value })} />
                  </div>
                  <div className="form-group">
                    <label>{t('fin.linked-invoice')}</label>
                    <select className="form-input" autoComplete="off" value={form.invoice_id || ''} onChange={(e) => setForm({ ...form, invoice_id: e.target.value })}>
                      <option value="">{t('none_opt')}</option>
                      {invoices.map((i) => <option key={i.id} value={i.id}>{i.number} — {i.title}</option>)}
                    </select>
                  </div>
                  <RelSelects form={form} setForm={setForm} clients={clients} properties={properties} campaigns={campaigns} operations={operations} />
                </>
              )}
              {tab === 'expenses' && (
                <>
                  <div className="form-group">
                    <label>{t('date')}</label>
                    <input className="form-input" autoComplete="off" type="date" required value={form.occurs_on || ''} onChange={(e) => setForm({ ...form, occurs_on: e.target.value })} />
                  </div>
                  <div className="form-group">
                    <label>{tf('amount_cur', { cur })}</label>
                    <input className="form-input" autoComplete="off" type="number" min={0.01} step="0.01" required value={form.amount ?? ''} onChange={(e) => setForm({ ...form, amount: e.target.value })} />
                  </div>
                  <div className="form-group">
                    <label>{t('category')}</label>
                    <select className="form-input" autoComplete="off" value={form.category || 'أخرى'} onChange={(e) => setForm({ ...form, category: e.target.value })}>
                      {EXPENSE_CATEGORIES.map((c) => <option key={c.id} value={c.id}>{c.label}</option>)}
                    </select>
                  </div>
                  <div className="form-group">
                    <label>{t('fin.reference')}</label>
                    <input className="form-input" autoComplete="off" value={form.reference || ''} onChange={(e) => setForm({ ...form, reference: e.target.value })} />
                  </div>
                  <RelSelects form={form} setForm={setForm} clients={clients} properties={properties} campaigns={campaigns} />
                </>
              )}
              {tab === 'payments' && (
                <>
                  <div className="form-group">
                    <label>{t('date')}</label>
                    <input className="form-input" autoComplete="off" type="date" required value={form.occurs_on || ''} onChange={(e) => setForm({ ...form, occurs_on: e.target.value })} />
                  </div>
                  <div className="form-group">
                    <label>{tf('amount_cur', { cur })}</label>
                    <input className="form-input" autoComplete="off" type="number" min={0.01} step="0.01" required value={form.amount ?? ''} onChange={(e) => setForm({ ...form, amount: e.target.value })} />
                  </div>
                  <div className="form-group">
                    <label>{t('fin.target-of')}</label>
                    <select className="form-input" autoComplete="off" value={form.target_type || 'invoice'} onChange={(e) => setForm({ ...form, target_type: e.target.value, target_id: '' })}>
                      <option value="invoice">{t('fin.kind.invoice')}</option>
                      <option value="commission">{t('fin.kind.commission')}</option>
                    </select>
                  </div>
                  <div className="form-group">
                    <label>{form.target_type === 'commission' ? t('fin.kind.commission') : t('fin.kind.invoice')}</label>
                    <select className="form-input" autoComplete="off" required value={form.target_id || ''} onChange={(e) => setForm({ ...form, target_id: e.target.value })}>
                      <option value="">{t('choose')}</option>
                      {(form.target_type === 'commission' ? commissions : invoices).map((i) => (
                        <option key={i.id} value={i.id}>{i.number ? `${i.number} — ${i.title}` : i.title}</option>
                      ))}
                    </select>
                  </div>
                  <div className="form-group">
                    <label>{t('fin.payment-method')}</label>
                    <select className="form-input" autoComplete="off" value={form.method || ''} onChange={(e) => setForm({ ...form, method: e.target.value })}>
                      <option value="">{t('none_opt')}</option>
                      {PAYMENT_METHODS.map((m) => <option key={m.id} value={m.id}>{m.label}</option>)}
                    </select>
                  </div>
                  <div className="form-group">
                    <label>{t('fin.reference')}</label>
                    <input className="form-input" autoComplete="off" value={form.reference || ''} onChange={(e) => setForm({ ...form, reference: e.target.value })} />
                  </div>
                </>
              )}
              <div className="form-group full-row">
                <label>{t('notes')}</label>
                <input className="form-input" autoComplete="off" value={form.notes || ''} onChange={(e) => setForm({ ...form, notes: e.target.value })} />
              </div>
            </div>
            <div style={{ display: 'flex', gap: 8, marginTop: 16 }}>
              <button className="btn btn-primary" type="submit" disabled={saving}>{saving ? t('saving') : t('save')}</button>
              <button className="btn btn-outline" type="button" onClick={closeForm}>{t('cancel')}</button>
            </div>
          </form>
        </div>
      )}

      {(tab === 'overview' || tab === 'reports') && summary && (
        <>
          {summary.training_data_present ? (
            <div className="training-banner" role="status">{t('training.banner')}</div>
          ) : null}
          <p className="muted">الملخص المالي للدولة المحددة — {summary.country} / {summary.currency}</p>
          <div className="metric-grid finance-kpis">
          <div className="metric-card kpi-3d">
            <div className="metric-label">{t('fin.kpi.inv-collected')}</div>
            <div className="metric-value"><Money currency={summary.currency} value={summary.invoice_collected} /></div>
            <div className="muted">{t('fin.of-total')} <Money currency={summary.currency} value={summary.invoice_total} /></div>
          </div>
          <div className="metric-card kpi-3d">
            <div className="metric-label">{t('fin.kpi.comm-collected')}</div>
            <div className="metric-value"><Money currency={summary.currency} value={summary.commission_collected} /></div>
            <div className="muted">{t('fin.of-total')} <Money currency={summary.currency} value={summary.commission_total} /></div>
          </div>
          <div className="metric-card kpi-3d">
            <div className="metric-label">{t('fin.kpi.expenses')}</div>
            <div className="metric-value"><Money currency={summary.currency} value={summary.expense_total} /></div>
          </div>
          <div className="metric-card kpi-3d">
            <div className="metric-label">{t('fin.kpi.receivables')}</div>
            <div className="metric-value"><Money currency={summary.currency} value={summary.receivables} /></div>
          </div>
          <div className="metric-card kpi-3d">
            <div className="metric-label">{t('fin.kpi.net')}</div>
            <div className="metric-value"><Money currency={summary.currency} value={summary.net_cash} /></div>
            <div className="muted">{t('fin.net-hint')}</div>
          </div>
          </div>
          {tab === 'overview' && contractsSummary && (
            <div className="content-card obr-panel" style={{ marginTop: 12 }}>
              <div className="card-header">
                <div className="card-title"><FileText size={17} strokeWidth={1.8} style={{ verticalAlign: '-3px' }} /> ملخص العقود</div>
                <button className="btn btn-outline btn-sm" type="button" onClick={() => { setTab('contracts'); closeForm(); }}>فتح العقود</button>
              </div>
              <div className="table-container">
                <table className="data-table">
                  <thead><tr><th>البيان</th><th>القيمة</th></tr></thead>
                  <tbody>
                    <tr><td>إجمالي العقود</td><td className="num">{contractsSummary.total ?? 0}</td></tr>
                    <tr><td>العقود النشطة</td><td className="num">{contractsSummary.active ?? 0}</td></tr>
                    <tr><td>العقود المنتهية</td><td className="num">{contractsSummary.expired ?? 0}</td></tr>
                    <tr><td>تنتهي قريبًا</td><td className="num">{contractsSummary.ending_soon ?? 0}</td></tr>
                    <tr><td>مسودات/غير موثقة</td><td className="num">{contractsSummary.drafts ?? 0}</td></tr>
                    <tr><td>عمولات العقود</td><td className="num">{Number(contractsSummary.commission_total || 0).toLocaleString('en-US')}</td></tr>
                  </tbody>
                </table>
              </div>
            </div>
          )}
        </>
      )}

      {tab === 'contracts' && <ContractsTab newDraftSignal={contractsNonce} />}

      {tab === 'reports' && (
        <div className="content-card obr-panel" style={{ marginTop: 12 }}>
          <div className="obr-panel-body">
            <p style={{ marginTop: 0 }}>{t('fin.report-hint')}</p>
            <button className="btn btn-primary" type="button" onClick={makeReportPdf}>
              <Printer size={15} strokeWidth={1.8} /> {t('fin.report-btn')}
            </button>
          </div>
        </div>
      )}

      {tab !== 'overview' && tab !== 'reports' && tab !== 'contracts' && (
        <>
        <div className="content-card obr-panel">
        <div className="table-container">
          <table className="data-table">
            <thead>
              <tr>
                <th>{t('date')}</th>
                <th>{t('statement')}</th>
                {(tab === 'invoices' || tab === 'commissions' || tab === 'receivables' || tab === 'ledger') && <th>{t('status')}</th>}
                <th>{t('amount')}</th>
                {(tab === 'invoices' || tab === 'commissions') && <th>{t('remaining')}</th>}
                {tab === 'commissions' && <th>{t('fin.shares')}</th>}
                {(crudTabs || tab === 'receivables') && <th></th>}
              </tr>
            </thead>
            <tbody>
              {items.length === 0 && !loading && (
                <tr><td colSpan={7}>{t('fin.empty')}</td></tr>
              )}
              {pagedItems.map((item) => (
                <tr key={item.id}>
                  <td>{item.issue_date || item.occurs_on || '—'}</td>
                  <td>
                    {item.number ? `${item.number} — ` : ''}{item.title || item.target_title || '—'}
                    {item.kind_label ? <div className="muted">{item.kind_label}</div> : null}
                    {item.client_name || item.property_name ? (
                      <div className="muted">{[item.client_name, item.property_name, item.campaign_name].filter(Boolean).join(' · ')}</div>
                    ) : null}
                  </td>
                  {(tab === 'invoices' || tab === 'commissions' || tab === 'receivables' || tab === 'ledger') && (
                    <td><StatusBadge status={item.status} label={item.status_label} /></td>
                  )}
                  <td><Money currency={item.currency} value={item.computed_amount ?? item.amount ?? item.deal_value} /></td>
                  {(tab === 'invoices' || tab === 'commissions') && <td><Money currency={item.currency} value={item.remaining} /></td>}
                  {tab === 'commissions' && (
                    <td className="muted">
                      {t('fin.share-office')} <Money currency={item.currency} value={item.office_amount} /> · {t('fin.share-broker')} <Money currency={item.currency} value={item.broker_amount} /> · {t('fin.share-marketer')} <Money currency={item.currency} value={item.marketer_amount} /> · {t('fin.share-partner')} <Money currency={item.currency} value={item.partner_amount} />
                    </td>
                  )}
                  {crudTabs && (
                    <td>
                      <button className="btn btn-outline" type="button" onClick={() => openEdit(item)}><Pencil size={14} /></button>
                      {' '}
                      <button className="btn btn-outline" type="button" onClick={() => handleDelete(item.id)}><Trash2 size={14} /></button>
                      {' '}
                      <button className="btn btn-outline" type="button" onClick={() => printRecord(item)}>
                        <Printer size={14} /> {t('print')}
                      </button>
                    </td>
                  )}
                  {tab === 'receivables' && (
                    <td>
                      <button className="btn btn-outline" type="button" onClick={() => printRecord(item)}>
                        <Printer size={14} /> {t('print')}
                      </button>
                    </td>
                  )}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        </div>
        <PaginationBar total={items.length} page={page} onPage={setPage} label={t('record_label')} />
        </>
      )}
    </div>
  );
}
