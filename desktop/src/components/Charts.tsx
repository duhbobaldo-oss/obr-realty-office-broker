import React from 'react';

// ===== رسوم بيانية SVG خفيفة تعمل Offline بدون مكتبات =====

export interface Slice {
  label: string;
  value: number;
  color: string;
}

/** مخطط دائري مجوّف (Donut) مع وسط رقمي */
export function DonutChart({ slices, centerValue, centerLabel }: {
  slices: Slice[];
  centerValue?: string | number;
  centerLabel?: string;
}) {
  const total = slices.reduce((s, x) => s + x.value, 0);
  const R = 70;
  const C = 2 * Math.PI * R;
  let offset = 0;

  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 28, flexWrap: 'wrap' }}>
      <svg width="176" height="176" viewBox="0 0 180 180" style={{ flexShrink: 0 }}>
        <circle cx="90" cy="90" r={R} fill="none" stroke="var(--chart-track)" strokeWidth="16" />
        {total > 0 && slices.map((s, i) => {
          const frac = s.value / total;
          const dash = frac * C;
          const el = (
            <circle
              key={i}
              cx="90" cy="90" r={R} fill="none"
              stroke={s.color}
              strokeWidth="16"
              strokeDasharray={`${dash} ${C - dash}`}
              strokeDashoffset={-offset}
              transform="rotate(-90 90 90)"
              strokeLinecap="butt"
            />
          );
          offset += dash;
          return el;
        })}
        <text
          x="90" y="87" textAnchor="middle"
          fontSize="22" fontWeight="600" fill="var(--ink)"
          fontFamily='"Segoe UI Variable", "Segoe UI", Tahoma, Arial, sans-serif'
        >
          {centerValue ?? total}
        </text>
        <text x="90" y="108" textAnchor="middle" fontSize="10.5" fontWeight="600" fill="var(--gray)">
          {centerLabel || 'الإجمالي'}
        </text>
      </svg>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 9, minWidth: 158, flex: 1 }}>
        {slices.map((s, i) => (
          <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 9, fontSize: 12.5 }}>
            <span style={{ width: 8, height: 8, borderRadius: 2, background: s.color, flexShrink: 0 }} />
            <span style={{ color: 'var(--muted-foreground)', fontWeight: 600, flex: 1 }}>{s.label}</span>
            <span style={{ fontWeight: 800, fontVariantNumeric: 'tabular-nums', color: 'var(--ink)' }}>
              {s.value}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

export interface BarItem {
  label: string;
  value: number;
  display?: string;
  color?: string;
}

/** أعمدة أفقية أنيقة */
export function BarList({ items }: { items: BarItem[] }) {
  const max = Math.max(...items.map(i => i.value), 1);
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
      {items.map((it, i) => (
        <div key={i}>
          <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 12.5, marginBottom: 5 }}>
            <span style={{ fontWeight: 700 }}>{it.label}</span>
            <span style={{ color: 'var(--gray)', fontWeight: 700, fontVariantNumeric: 'tabular-nums' }}>
              {it.display ?? it.value}
            </span>
          </div>
          <div style={{ height: 10, borderRadius: 6, background: '#eee5d8', overflow: 'hidden' }}>
            <div style={{
              height: '100%',
              width: `${Math.max((it.value / max) * 100, 2)}%`,
              borderRadius: 6,
              background: it.color || 'linear-gradient(120deg, #b45309, #d97706)',
              transition: 'width 0.6s cubic-bezier(0.22,1,0.36,1)'
            }} />
          </div>
        </div>
      ))}
      {items.length === 0 && (
        <div style={{ color: 'var(--gray)', textAlign: 'center', padding: 20 }}>لا توجد بيانات بعد</div>
      )}
    </div>
  );
}

export interface FunnelStage {
  label: string;
  value: number;
  rateLabel?: string;
  color?: string;
}

/** مراحل تحويل العملاء: عرض كل مرحلة بنسبة قيمتها الحقيقية، والمعدل يظهر فقط إن وُجد */
export function FunnelChart({ stages }: { stages: FunnelStage[] }) {
  const max = Math.max(...stages.map((s) => s.value), 1);
  const palette = ['var(--chart-1)', 'var(--chart-2)', 'var(--chart-3)', 'var(--chart-4)'];

  return (
    <div className="funnel-chart" role="img" aria-label="مراحل تحويل العملاء">
      {stages.map((stage, i) => {
        const widthPct = Math.max((stage.value / max) * 100, stage.value > 0 ? 8 : 0);
        return (
          <div key={stage.label} className="funnel-step">
            {i > 0 && (
              <div className="funnel-bridge">
                <span className="funnel-bridge-line" aria-hidden="true" />
                <span className="funnel-bridge-rate">{stage.rateLabel || '—'}</span>
              </div>
            )}
            <div className="funnel-stage">
              <div className="funnel-stage-copy">
                <span className="funnel-stage-label">{stage.label}</span>
                <span className="funnel-stage-value num">{stage.value.toLocaleString('en-US')}</span>
              </div>
              <div className="funnel-track">
                <div
                  className="funnel-fill"
                  style={{
                    width: `${widthPct}%`,
                    background: stage.color || palette[i % palette.length]
                  }}
                />
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );
}

export interface SeriesBar {
  key: string;
  label: string;
  color: string;
}

/** مقارنة أفقية مجمّعة بين الحملات — الأرقام تبقى tabular وفي اتجاهها الصحيح */
export function GroupedBarList({
  rows,
  series
}: {
  rows: { label: string; values: Record<string, number> }[];
  series: SeriesBar[];
}) {
  const max = Math.max(
    1,
    ...rows.flatMap((row) => series.map((s) => Number(row.values[s.key] || 0)))
  );

  return (
    <div className="grouped-bars">
      {rows.length === 0 && (
        <div style={{ color: 'var(--gray)', textAlign: 'center', padding: 20 }}>لا توجد بيانات بعد</div>
      )}
      {rows.map((row) => (
        <div key={row.label} className="grouped-bar-row">
          <div className="grouped-bar-name">{row.label}</div>
          <div className="grouped-bar-stack">
            {series.map((s) => {
              const value = Number(row.values[s.key] || 0);
              return (
                <div key={s.key} className="grouped-bar-line">
                  <span className="grouped-bar-key">{s.label}</span>
                  <div className="grouped-bar-track">
                    <div
                      className="grouped-bar-fill"
                      style={{
                        width: `${Math.max((value / max) * 100, value > 0 ? 3 : 0)}%`,
                        background: s.color
                      }}
                    />
                  </div>
                  <span className="grouped-bar-num num">{value.toLocaleString('en-US')}</span>
                </div>
              );
            })}
          </div>
        </div>
      ))}
    </div>
  );
}

/** مخطط خطي بسيط لعرض الاتجاهات */
export function LineChart({ points, labels, height = 160, color = '#d97706' }: {
  points: number[];
  labels?: string[];
  height?: number;
  color?: string;
}) {
  const uid = React.useId().replace(/:/g, '');
  const W = 560;
  const H = height;
  const pad = 28;
  const max = Math.max(...points, 1);
  const min = Math.min(...points, 0);
  const range = max - min || 1;
  const stepX = points.length > 1 ? (W - pad * 2) / (points.length - 1) : 0;
  const y = (v: number) => H - pad - ((v - min) / range) * (H - pad * 2);
  const coords = points.map((p, i) => [pad + i * stepX, y(p)] as const);
  const path = coords.map(([x, yy], i) => `${i === 0 ? 'M' : 'L'}${x},${yy}`).join(' ');
  const area = points.length > 1 ? `${path} L${pad + (points.length - 1) * stepX},${H - pad} L${pad},${H - pad} Z` : '';

  return (
    <svg width="100%" viewBox={`0 0 ${W} ${H}`} style={{ display: 'block' }} role="img">
      <defs>
        <linearGradient id={`lineFill-${uid}`} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor={color} stopOpacity="0.28" />
          <stop offset="100%" stopColor={color} stopOpacity="0.02" />
        </linearGradient>
      </defs>
      {[0.25, 0.5, 0.75].map((g) => (
        <line key={g} x1={pad} x2={W - pad} y1={pad + g * (H - pad * 2)} y2={pad + g * (H - pad * 2)}
          stroke="#e9e0d2" strokeDasharray="4 4" />
      ))}
      {area && <path d={area} fill={`url(#lineFill-${uid})`} />}
      <path d={path} fill="none" stroke={color} strokeWidth="2.6" strokeLinecap="round" strokeLinejoin="round" />
      {coords.map(([x, yy], i) => (
        <g key={i}>
          <circle cx={x} cy={yy} r="4" fill="#fff8ee" stroke={color} strokeWidth="2" />
          {labels?.[i] && (
            <text x={x} y={H - 8} textAnchor="middle" fontSize="10" fontWeight="600" fill="var(--gray)">
              {labels[i]}
            </text>
          )}
        </g>
      ))}
    </svg>
  );
}

export function MultiLineChart({
  labels,
  series,
  height = 220
}: {
  labels: string[];
  series: { label: string; color: string; points: number[] }[];
  height?: number;
}) {
  const uid = React.useId().replace(/:/g, '');
  const W = 640;
  const H = height;
  const padL = 36;
  const padR = 16;
  const padT = 18;
  const padB = 36;
  const all = series.flatMap((s) => s.points);
  const max = Math.max(...all, 1);
  const innerW = W - padL - padR;
  const innerH = H - padT - padB;
  const n = Math.max(labels.length, 1);
  const x = (i: number) => padL + (n === 1 ? innerW / 2 : (i / (n - 1)) * innerW);
  const y = (v: number) => padT + innerH - (v / max) * innerH;

  return (
    <div>
      <svg width="100%" viewBox={`0 0 ${W} ${H}`} style={{ display: 'block' }} role="img">
        {[0.25, 0.5, 0.75, 1].map((g) => (
          <line key={g} x1={padL} x2={W - padR} y1={padT + innerH * (1 - g)} y2={padT + innerH * (1 - g)}
            stroke="#e9e0d2" strokeDasharray="4 4" />
        ))}
        {series.map((s, si) => {
          const coords = s.points.map((p, i) => `${i === 0 ? 'M' : 'L'}${x(i)},${y(p)}`).join(' ');
          const area = s.points.length > 1
            ? `${coords} L${x(s.points.length - 1)},${padT + innerH} L${x(0)},${padT + innerH} Z`
            : '';
          return (
            <g key={s.label}>
              <defs>
                <linearGradient id={`ml-${uid}-${si}`} x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor={s.color} stopOpacity="0.22" />
                  <stop offset="100%" stopColor={s.color} stopOpacity="0" />
                </linearGradient>
              </defs>
              {area && <path d={area} fill={`url(#ml-${uid}-${si})`} />}
              <path d={coords} fill="none" stroke={s.color} strokeWidth="2.6" strokeLinejoin="round" strokeLinecap="round" />
              {s.points.map((p, i) => (
                <circle key={i} cx={x(i)} cy={y(p)} r="3.5" fill="#fff8ee" stroke={s.color} strokeWidth="2" />
              ))}
            </g>
          );
        })}
        {labels.map((label, i) => (
          <text key={label + i} x={x(i)} y={H - 10} textAnchor="middle" fontSize="10" fontWeight="600" fill="var(--gray)">
            {label}
          </text>
        ))}
      </svg>
      <div className="chart-legend">
        {series.map((s) => (
          <span key={s.label} className="chart-legend-item">
            <i style={{ background: s.color }} />
            {s.label}
          </span>
        ))}
      </div>
    </div>
  );
}

export function ColumnChart({
  labels,
  series,
  height = 210,
  formatValue
}: {
  labels: string[];
  series: { label: string; color: string; values: number[] }[];
  height?: number;
  formatValue?: (value: number) => string;
}) {
  const W = 560;
  const H = height;
  const padL = 28;
  const padR = 12;
  const padT = 28;
  const padB = 12;
  const max = Math.max(1, ...series.flatMap((s) => s.values));
  const innerW = W - padL - padR;
  const innerH = H - padT - padB;
  const groupW = innerW / Math.max(labels.length, 1);
  const barW = Math.min(22, (groupW * 0.7) / Math.max(series.length, 1));
  const y = (v: number) => padT + innerH - (v / max) * innerH;
  const fmt = formatValue || ((value: number) => value.toLocaleString('en-US', { maximumFractionDigits: value >= 100 ? 0 : 1 }));

  return (
    <div>
      <svg width="100%" viewBox={`0 0 ${W} ${H}`} style={{ display: 'block' }} role="img">
        {[0.25, 0.5, 0.75, 1].map((g) => (
          <line key={g} x1={padL} x2={W - padR} y1={padT + innerH * (1 - g)} y2={padT + innerH * (1 - g)}
            stroke="#e9e0d2" strokeDasharray="4 4" />
        ))}
        {labels.map((label, i) => {
          const gx = padL + i * groupW + (groupW - series.length * (barW + 4)) / 2;
          return (
            <g key={label + i}>
              {series.map((s, si) => {
                const value = Number(s.values[i] || 0);
                const bh = (value / max) * innerH;
                const bx = gx + si * (barW + 4);
                return (
                  <g key={s.label}>
                    <rect
                      x={bx}
                      y={y(value)}
                      width={barW}
                      height={Math.max(bh, value > 0 ? 2 : 0)}
                      rx="4"
                      fill={s.color}
                    />
                    {value > 0 && (
                      <text
                        x={bx + barW / 2}
                        y={y(value) - 6}
                        textAnchor="middle"
                        fontSize="10"
                        fontWeight="700"
                        fill="var(--ink)"
                      >
                        {fmt(value)}
                      </text>
                    )}
                  </g>
                );
              })}
            </g>
          );
        })}
      </svg>
      <div className="chart-index" role="table">
        <div className="chart-index-row chart-index-head" role="row">
          <span>البند</span>
          {series.map((s) => <span key={s.label}>{s.label}</span>)}
        </div>
        {labels.map((label, i) => (
          <div key={label + i} className="chart-index-row" role="row">
            <span>{label}</span>
            {series.map((s) => (
              <span key={s.label} className="num">{fmt(Number(s.values[i] || 0))}</span>
            ))}
          </div>
        ))}
      </div>
      {series.length > 1 && (
        <div className="chart-legend">
          {series.map((s) => (
            <span key={s.label} className="chart-legend-item">
              <i style={{ background: s.color }} />
              {s.label}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}

export function Sparkline({ points, color = '#d97706' }: { points: number[]; color?: string }) {
  const W = 132;
  const H = 38;
  if (points.length === 0) return null;
  const max = Math.max(...points, 1);
  const min = Math.min(...points, 0);
  const range = max - min || 1;
  const step = points.length > 1 ? W / (points.length - 1) : 0;
  const y = (v: number) => H - 4 - ((v - min) / range) * (H - 8);
  const d = points.map((p, i) => `${i === 0 ? 'M' : 'L'}${(points.length === 1 ? W / 2 : i * step).toFixed(1)},${y(p).toFixed(1)}`).join(' ');
  return (
    <svg width="100%" viewBox={`0 0 ${W} ${H}`} className="dash-sparkline" aria-hidden="true">
      <path d={d} fill="none" stroke={color} strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

export function RadialProgress({
  percent,
  label,
  color = 'var(--chart-1)'
}: {
  percent: number;
  label: string;
  color?: string;
}) {
  const R = 46;
  const C = 2 * Math.PI * R;
  const pct = Math.max(0, Math.min(100, percent));
  const dash = (pct / 100) * C;
  return (
    <div className="dash-radial">
      <svg width="118" height="118" viewBox="0 0 120 120">
        <circle cx="60" cy="60" r={R} fill="none" stroke="var(--chart-track)" strokeWidth="11" />
        <circle
          cx="60" cy="60" r={R} fill="none"
          stroke={color}
          strokeWidth="11"
          strokeDasharray={`${dash} ${C - dash}`}
          strokeLinecap="round"
          transform="rotate(-90 60 60)"
        />
        <text x="60" y="56" textAnchor="middle" fontSize="20" fontWeight="800" fill="var(--ink)" style={{ fontVariantNumeric: 'tabular-nums' }}>
          {Math.round(pct)}%
        </text>
        <text x="60" y="74" textAnchor="middle" fontSize="10" fontWeight="600" fill="var(--gray)">النسبة</text>
      </svg>
      <div className="dash-radial-label">{label}</div>
    </div>
  );
}
