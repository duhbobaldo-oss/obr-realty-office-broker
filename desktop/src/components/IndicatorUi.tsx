import React from 'react';
import type { LucideIcon } from 'lucide-react';
import { ArrowDownRight, ArrowUpRight, Minus, Activity } from 'lucide-react';
import { useLocale } from '../hooks/useLocale';

export type IndicatorTone = 'default' | 'gold' | 'primary' | 'danger';
export type IndicatorStatus = 'جيد' | 'متوسط' | 'ضعيف' | 'سلبي';

type IconComp = React.ComponentType<any>;

export function DeltaBadge({ value }: { value: number }) {
  const up = value >= 0;
  return (
    <span className={`ind-delta ${up ? 'is-up' : 'is-down'}`}>
      {up ? <ArrowUpRight size={12} strokeWidth={2.2} /> : <ArrowDownRight size={12} strokeWidth={2.2} />}
      {Math.abs(value).toLocaleString('en-US', { maximumFractionDigits: 1 })}%
    </span>
  );
}

export function StatusBadge({ status }: { status: IndicatorStatus }) {
  const { t } = useLocale();
  const cls =
    status === 'جيد'
      ? 'is-good'
      : status === 'متوسط'
        ? 'is-mid'
        : status === 'ضعيف'
          ? 'is-weak'
          : 'is-bad';
  const label =
    status === 'جيد'
      ? t('ind_good')
      : status === 'متوسط'
        ? t('ind_mid')
        : status === 'ضعيف'
          ? t('ind_weak')
          : t('ind_bad');
  return <span className={`ind-status ${cls}`}>{label}</span>;
}

export function SectionHairlineTitle({
  title,
  subtitle,
  help,
  icon: Icon
}: {
  title: string;
  subtitle?: string;
  help?: React.ReactNode;
  icon?: LucideIcon;
}) {
  return (
    <div className="ind-section-head">
      <div className="ind-section-title-row">
        <span className="ind-section-hairline" aria-hidden />
        <h2 className="ind-section-title">
          {Icon ? <Icon className="ind-section-icon" size={17} strokeWidth={1.8} aria-hidden /> : null}
          {title}
          {help ? <span className="ind-section-help">{help}</span> : null}
        </h2>
      </div>
      {subtitle ? <p className="ind-section-sub">{subtitle}</p> : null}
    </div>
  );
}

/** بطاقة مؤشر: kpi-3d + قيمة كبيرة + تلميح/دلتا/حالة */
export function IndicatorKpi({
  label,
  value,
  unit,
  hint,
  delta,
  status,
  icon: Icon,
  tone = 'default',
  children
}: {
  label: string;
  value: string;
  unit?: string;
  hint?: string;
  delta?: number | null;
  status?: IndicatorStatus;
  icon?: IconComp;
  tone?: IndicatorTone;
  children?: React.ReactNode;
}) {
  return (
    <article className="ind-kpi surface-card kpi-3d">
      <div className="ind-kpi-hairline" aria-hidden />
      <div className="ind-kpi-top">
        <p className="ind-kpi-label">{label}</p>
        {Icon ? (
          <span className="ind-kpi-icon" aria-hidden>
            <Icon size={18} strokeWidth={1.8} />
          </span>
        ) : null}
      </div>
      <p className={`ind-kpi-value num tone-${tone}`} title={value}>
        {value}
        {unit ? <span className="ind-kpi-unit">{unit}</span> : null}
      </p>
      {(delta != null || hint || status) && (
        <div className="ind-kpi-meta">
          {delta != null ? <DeltaBadge value={delta} /> : null}
          {status ? <StatusBadge status={status} /> : null}
          {hint ? <span className="ind-kpi-hint">{hint}</span> : null}
        </div>
      )}
      {children}
    </article>
  );
}

export type PulseTickerItem = {
  id: string;
  label: string;
  kindLabel?: string;
  salePerSqm?: number | null;
  rentPerSqm?: number | null;
  changePct?: number | null;
  dealsCount?: number | null;
  dealsLabel?: string;
};

function formatMoneyShort(v: number): string {
  return Math.round(v).toLocaleString('en-US');
}

function hasPulseValue(item: PulseTickerItem): boolean {
  return (
    (item.salePerSqm != null && Number.isFinite(item.salePerSqm)) ||
    (item.rentPerSqm != null && Number.isFinite(item.rentPerSqm)) ||
    (item.changePct != null && Number.isFinite(item.changePct)) ||
    (item.dealsCount != null && Number.isFinite(item.dealsCount))
  );
}

function PulseChip({ item, currency }: { item: PulseTickerItem; currency: string }) {
  const { t } = useLocale();
  const sqm = t('unit_sqm_short');
  return (
    <span className="pulse-chip">
      <span className="pulse-chip-label">{item.label}</span>
      {item.kindLabel ? <span className="pulse-chip-muted">· {item.kindLabel}</span> : null}
      {item.salePerSqm != null && Number.isFinite(item.salePerSqm) ? (
        <span className="num">{t('pulse_sale')} {formatMoneyShort(item.salePerSqm)} {currency}/{sqm}</span>
      ) : null}
      {item.rentPerSqm != null && Number.isFinite(item.rentPerSqm) ? (
        <span className="num">{t('pulse_rent')} {formatMoneyShort(item.rentPerSqm)} {currency}/{sqm}</span>
      ) : null}
      {item.changePct != null && Number.isFinite(item.changePct) ? (
        item.changePct > 0 ? (
          <span className="pulse-chip-up num">
            <ArrowUpRight size={12} />
            {Math.abs(item.changePct).toLocaleString('en-US', { maximumFractionDigits: 1 })}%
          </span>
        ) : item.changePct < 0 ? (
          <span className="pulse-chip-down num">
            <ArrowDownRight size={12} />
            {Math.abs(item.changePct).toLocaleString('en-US', { maximumFractionDigits: 1 })}%-
          </span>
        ) : (
          <span className="pulse-chip-muted">
            <Minus size={12} />
            0%
          </span>
        )
      ) : null}
      {item.dealsCount != null && Number.isFinite(item.dealsCount) ? (
        <span className="pulse-chip-muted num">
          {item.dealsCount.toLocaleString('en-US')} {item.dealsLabel || t('market.record')}
        </span>
      ) : null}
    </span>
  );
}

/** شريط نبض المؤشر — تمرير RTL من بيانات حقيقية فقط */
export function MarketPulseTicker({
  items,
  currency,
  ariaLabel
}: {
  items: PulseTickerItem[];
  currency: string;
  ariaLabel?: string;
}) {
  const { t, lang } = useLocale();
  const rendered = items.filter(hasPulseValue).slice(0, 12);
  if (rendered.length === 0) return null;
  const repeats = rendered.length <= 2 ? 8 : rendered.length <= 4 ? 6 : 4;
  const label = ariaLabel || t('market.pulse-aria');

  return (
    <div className="content-card market-fin-panel market-fin-pulse" role="status" aria-label={label}>
      <div className="card-header market-fin-head">
        <div className="card-title">
          <Activity size={17} strokeWidth={1.8} aria-hidden /> {t('market.pulse')}
        </div>
        <span className="market-fin-chip">{currency} / م²</span>
      </div>
      <div className="market-fin-frame market-fin-pulse-frame">
        <div className="pulse-ticker" dir={lang === 'en' ? 'ltr' : 'rtl'}>
          <div className="pulse-ticker-viewport">
            <div className="pulse-ticker-fade start" aria-hidden />
            <div className="pulse-ticker-fade end" aria-hidden />
            <div className="pulse-ticker-track">
              {Array.from({ length: repeats }, (_, i) => (
                <div key={i} className="pulse-ticker-group" aria-hidden={i >= repeats / 2 || undefined}>
                  {rendered.map((item) => (
                    <PulseChip key={`${i}-${item.id}`} item={item} currency={currency} />
                  ))}
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
