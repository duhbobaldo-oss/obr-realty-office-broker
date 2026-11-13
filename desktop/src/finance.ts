import { formatMoneyValue, t } from './domain/locale';

type Opt = { id: string; label: string };

function expenseCategories(): Opt[] {
  return [
    { id: 'تسويق', label: t('fin.cat.marketing') },
    { id: 'انتقالات', label: t('fin.cat.travel') },
    { id: 'رسوم', label: t('fin.cat.fees') },
    { id: 'صيانة', label: t('fin.cat.maintenance') },
    { id: 'أخرى', label: t('fin.cat.other') }
  ];
}

function paymentMethods(): Opt[] {
  return [
    { id: 'تحويل', label: t('fin.pay.transfer') },
    { id: 'نقد', label: t('fin.pay.cash') },
    { id: 'شبكة', label: t('fin.pay.network') },
    { id: 'شيك', label: t('fin.pay.check') },
    { id: 'أخرى', label: t('fin.pay.other') }
  ];
}

function financeTabs(): Opt[] {
  return [
    { id: 'overview', label: t('fin.tab.overview') },
    { id: 'invoices', label: t('fin.tab.invoices') },
    { id: 'operations', label: t('fin.tab.operations') },
    { id: 'commissions', label: t('fin.tab.commissions') },
    { id: 'contracts', label: t('fin.tab.contracts') },
    { id: 'expenses', label: t('fin.tab.expenses') },
    { id: 'payments', label: t('fin.tab.payments') },
    { id: 'receivables', label: t('fin.tab.receivables') },
    { id: 'ledger', label: t('fin.tab.ledger') },
    { id: 'reports', label: t('fin.tab.reports') }
  ];
}

function statusOptions(): Opt[] {
  return [
    { id: 'due', label: t('fin.status.due') },
    { id: 'partial', label: t('fin.status.partial') },
    { id: 'paid', label: t('fin.status.paid') },
    { id: 'overdue', label: t('fin.status.overdue') }
  ];
}

function ledgerKinds(): Opt[] {
  return [
    { id: 'invoice', label: t('fin.kind.invoice') },
    { id: 'commission', label: t('fin.kind.commission') },
    { id: 'expense', label: t('fin.kind.expense') },
    { id: 'payment', label: t('fin.kind.payment') },
    { id: 'operation', label: t('fin.kind.operation') }
  ];
}

function proxyList(factory: () => Opt[]): Opt[] {
  return new Proxy([] as Opt[], {
    get(_t, prop) {
      const list = factory();
      const v = Reflect.get(list, prop, list);
      return typeof v === 'function' ? (v as (...a: unknown[]) => unknown).bind(list) : v;
    },
    ownKeys: () => Reflect.ownKeys(factory()),
    getOwnPropertyDescriptor: (_t, p) => Reflect.getOwnPropertyDescriptor(factory(), p),
    has: (_t, p) => Reflect.has(factory(), p)
  });
}

/** قيم التصنيف كما تُحفظ في قاعدة البيانات (عربي) — العرض مترجم */
export const EXPENSE_CATEGORIES: Opt[] = proxyList(expenseCategories);
/** قيم طريقة الدفع كما تُحفظ في قاعدة البيانات (عربي) — العرض مترجم */
export const PAYMENT_METHODS: Opt[] = proxyList(paymentMethods);

export const FINANCE_TABS: Opt[] = proxyList(financeTabs);
export const STATUS_OPTIONS: Opt[] = proxyList(statusOptions);
export const LEDGER_KINDS: Opt[] = proxyList(ledgerKinds);

export type TabId =
  | 'overview'
  | 'invoices'
  | 'operations'
  | 'commissions'
  | 'contracts'
  | 'expenses'
  | 'payments'
  | 'receivables'
  | 'ledger'
  | 'reports';

export function statusLabel(id: string | null | undefined): string {
  if (!id) return '—';
  return statusOptions().find((s) => s.id === id)?.label || '—';
}

export function formatMoneyAmount(value: number | null | undefined, currency?: string): string {
  if (value === null || value === undefined || Number.isNaN(Number(value))) return '—';
  return formatMoneyValue(value, 2, currency);
}

export function todayIso(): string {
  const d = new Date();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${d.getFullYear()}-${m}-${day}`;
}

export function periodQuery(from: string, to: string, extra: Record<string, string> = {}): string {
  const params = new URLSearchParams();
  if (from) params.set('period_from', from);
  if (to) params.set('period_to', to);
  for (const [k, v] of Object.entries(extra)) {
    if (v) params.set(k, v);
  }
  const q = params.toString();
  return q ? `?${q}` : '';
}
