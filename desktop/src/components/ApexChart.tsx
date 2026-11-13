import React, { useEffect, useMemo, useState } from 'react';
import Chart from 'react-apexcharts';
import type { ApexOptions } from 'apexcharts';

const FALLBACK_COLORS = ['#23365f', '#d97706', '#3b4966', '#b45309', '#6f7380', '#365247'];

function readCssVar(name: string, fallback: string): string {
  if (typeof document === 'undefined') return fallback;
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return value || fallback;
}

function readThemeBrandColors(): string[] {
  return [
    readCssVar('--viz-primary', FALLBACK_COLORS[0]),
    readCssVar('--viz-accent', FALLBACK_COLORS[1]),
    readCssVar('--chart-3', FALLBACK_COLORS[2]),
    readCssVar('--chart-4', FALLBACK_COLORS[3]),
    readCssVar('--chart-5', FALLBACK_COLORS[4]),
    readCssVar('--purple', FALLBACK_COLORS[5])
  ];
}

function useThemeBrandColors(): string[] {
  const [colors, setColors] = useState(readThemeBrandColors);
  useEffect(() => {
    const sync = () => setColors(readThemeBrandColors());
    sync();
    const root = document.documentElement;
    const observer = new MutationObserver(sync);
    observer.observe(root, { attributes: true, attributeFilter: ['data-theme'] });
    return () => observer.disconnect();
  }, []);
  return colors;
}

function useThemeInk() {
  const colors = useThemeBrandColors();
  return {
    brandColors: colors,
    primary: colors[0],
    accent: colors[1],
    ink: readCssVar('--primary', colors[0]),
    muted: readCssVar('--muted-foreground', '#6f7380'),
    border: readCssVar('--border', '#e9e0d2'),
    card: readCssVar('--card', '#f9f6f1'),
    onPrimary: readCssVar('--primary-foreground', '#f7f1e8'),
    track: readCssVar('--chart-track', '#eee5d8')
  };
}

const baseOptions: ApexOptions = {
  chart: {
    fontFamily: '"Segoe UI Variable", "Segoe UI", Tahoma, Arial, sans-serif',
    toolbar: { show: false },
    zoom: { enabled: false },
    animations: { enabled: true, speed: 550 },
    background: 'transparent'
  },
  dataLabels: { enabled: false },
  grid: {
    strokeDashArray: 4,
    padding: { left: 8, right: 8 }
  },
  stroke: { curve: 'smooth', width: 2.5 },
  legend: {
    position: 'bottom',
    fontWeight: 600,
    markers: { size: 6, offsetX: -2 }
  },
  tooltip: {
    theme: 'light',
    style: { fontSize: '12px' }
  }
};

function mergeOptions(extra: ApexOptions | undefined, brandColors: string[], muted: string, border: string): ApexOptions {
  return {
    ...baseOptions,
    ...extra,
    chart: {
      ...baseOptions.chart,
      foreColor: muted,
      ...extra?.chart
    },
    grid: {
      ...baseOptions.grid,
      borderColor: border,
      ...extra?.grid
    },
    legend: {
      ...baseOptions.legend,
      labels: { colors: muted },
      ...extra?.legend
    },
    stroke: { ...baseOptions.stroke, ...extra?.stroke },
    tooltip: { ...baseOptions.tooltip, ...extra?.tooltip },
    xaxis: {
      labels: { style: { colors: muted, fontWeight: 600, fontSize: '11px' } },
      axisBorder: { color: border },
      axisTicks: { color: border },
      ...extra?.xaxis
    },
    yaxis: Array.isArray(extra?.yaxis)
      ? extra?.yaxis
      : {
          labels: {
            style: { colors: muted, fontWeight: 600, fontSize: '11px' },
            formatter: (v: number) => Number(v).toLocaleString('en-US', { maximumFractionDigits: v >= 100 ? 0 : 1 })
          },
          ...(extra?.yaxis as object)
        },
    colors: extra?.colors || brandColors,
    plotOptions: { ...baseOptions.plotOptions, ...extra?.plotOptions },
    fill: { ...baseOptions.fill, ...extra?.fill },
    dataLabels: { ...baseOptions.dataLabels, ...extra?.dataLabels }
  };
}

type HostProps = {
  type: 'line' | 'area' | 'bar' | 'donut' | 'radialBar';
  series: any;
  options?: ApexOptions;
  height?: number;
};

// ===== حدود صلبة دفاعية قبل تمرير البيانات لمكتبة الرسم =====
// الارتفاع لا ينمو مع عدد السجلات؛ ولا تُرسم نقطة/تسمية لكل سجل قاعدة بيانات.
export const MAX_CHART_POINTS = 90;
export const MAX_CHART_CATEGORIES = 20;
const MAX_CHART_HEIGHT = 420;

function capSeriesData(series: any): any {
  if (Array.isArray(series)) {
    if (series.length && typeof series[0] === 'number') {
      // donut/radial: قيم مباشرة — حد الفئات
      if (series.length > MAX_CHART_CATEGORIES) {
        // إشارة تشخيصية: الـQuery كان يجب أن ترجع Dataset صغيرًا أصلًا
        console.warn(`UNEXPECTED DATASET SIZE: categories=${series.length} > ${MAX_CHART_CATEGORIES} (clipped)`);
        return series.slice(0, MAX_CHART_CATEGORIES);
      }
      return series;
    }
    return series.map((s: any) => {
      if (s && Array.isArray(s.data) && s.data.length > MAX_CHART_POINTS) {
        // إشارة تشخيصية: الـQuery كان يجب أن يجمّع ويحدّ قبل الوصول إلى الرسم
        console.warn(`UNEXPECTED DATASET SIZE: points=${s.data.length} > ${MAX_CHART_POINTS} (clipped) in series "${s?.name ?? '?'}"`);
        return { ...s, data: s.data.slice(0, MAX_CHART_POINTS) };
      }
      return s;
    });
  }
  return series;
}

function capChartOptions(options?: ApexOptions): ApexOptions | undefined {
  if (!options) return options;
  const capped: ApexOptions = { ...options };
  if (Array.isArray(capped.labels) && capped.labels.length > MAX_CHART_CATEGORIES) {
    console.warn(`UNEXPECTED DATASET SIZE: labels=${capped.labels.length} > ${MAX_CHART_CATEGORIES} (clipped)`);
    capped.labels = capped.labels.slice(0, MAX_CHART_CATEGORIES);
  }
  if (capped.xaxis && Array.isArray((capped.xaxis as any).categories) && (capped.xaxis as any).categories.length > MAX_CHART_POINTS) {
    console.warn(`UNEXPECTED DATASET SIZE: xaxis categories=${(capped.xaxis as any).categories.length} > ${MAX_CHART_POINTS} (clipped)`);
    capped.xaxis = {
      ...capped.xaxis,
      categories: (capped.xaxis as any).categories.slice(0, MAX_CHART_POINTS)
    } as any;
  }
  return capped;
}

export function ApexHost({ type, series, options, height = 280 }: HostProps) {
  const { brandColors, muted, border } = useThemeInk();
  const opts = useMemo(
    () => mergeOptions(capChartOptions(options), brandColors, muted, border),
    [options, brandColors, muted, border]
  );
  const safeSeries = capSeriesData(series);
  // الارتفاع ثابت بحد أقصى — لا يزيد مع حجم البيانات
  const safeHeight = Math.min(height, MAX_CHART_HEIGHT);
  return (
    <div className="apex-host" dir="ltr">
      <Chart type={type} series={safeSeries as any} options={opts} height={safeHeight} width="100%" />
    </div>
  );
}

function truncateAxisLabel(label: string, maxChars: number): string {
  const text = String(label ?? '').trim();
  if (text.length <= maxChars) return text;
  return `${text.slice(0, Math.max(1, maxChars - 1))}…`;
}

/** عرض عمود تسميات الفئات (عربي) — كافٍ بدون اقتطاع حاد */
function horizontalCategoryLabelWidth(categories: string[]): number {
  const maxLen = categories.reduce((m, c) => Math.max(m, String(c ?? '').trim().length), 0);
  return Math.min(176, Math.max(84, Math.ceil(maxLen * 9.5) + 18));
}

function horizontalBarChartHeight(categoryCount: number, seriesCount: number): number {
  const n = Math.max(1, categoryCount);
  const perRow = seriesCount > 1 ? 34 : n <= 2 ? 34 : 28;
  const legend = seriesCount > 1 ? 26 : 0;
  const valueAxis = 22;
  return Math.max(68, n * perRow + legend + valueAxis);
}

export function ApexBarChart({
  categories,
  series,
  horizontal = false,
  height = 280,
  valueFormatter,
  integerAxis = false,
  distributed = false
}: {
  categories: string[];
  series: { name: string; data: number[]; color?: string }[];
  horizontal?: boolean;
  height?: number;
  valueFormatter?: (v: number) => string;
  /** محور أعداد صحيحة (عدادات) — يمنع كسورًا مثل 0.2 على مخطط السجلات */
  integerAxis?: boolean;
  /** ألوان متدرجة لكل فئة في سلسلة واحدة */
  distributed?: boolean;
}) {
  const { brandColors, primary, ink, muted, border, onPrimary } = useThemeInk();

  const colors = distributed && series.length === 1
    ? categories.map((_, i) => brandColors[i % brandColors.length])
    : series.map((s, i) => s.color || brandColors[i % brandColors.length]);

  const flatValues = series.flatMap((s) => s.data.map((v) => Number(v) || 0));
  const maxVal = flatValues.length ? Math.max(...flatValues) : 0;
  const intMax = Math.max(1, Math.ceil(maxVal));
  /** محور أعداد صحيحة بمقياس «نظيف» — يتجنب تكات كسرية تظهر فارغة */
  const intAxisNice = (() => {
    if (intMax <= 1) return { max: 2, tickAmount: 2 };
    if (intMax <= 5) return { max: intMax + 1, tickAmount: intMax + 1 };
    const step = intMax <= 20 ? 5 : intMax <= 50 ? 10 : intMax <= 100 ? 20 : 50;
    const max = Math.ceil(intMax / step) * step;
    return { max, tickAmount: Math.max(2, Math.round(max / step)) };
  })();
  const intAxisMax = intAxisNice.max;
  const intTickAmount = intAxisNice.tickAmount;
  /** مخططات أفقية بقيم صغيرة/صحيحة — محور أعداد صحيحة يمنع تكرار 0 0 1 1 */
  const useIntegerValueAxis =
    integerAxis || (horizontal && maxVal > 0 && maxVal <= 20 && Math.abs(maxVal - Math.round(maxVal)) < 0.001);
  const compactValueAxis = horizontal && !useIntegerValueAxis && maxVal >= 1000;

  const maxCategoryLen = categories.reduce((m, c) => Math.max(m, String(c || '').length), 0);
  /** عمودي مزدحم: فئات كثيرة أو تسميات طويلة أو مجموعات متعددة — لا نرسم تسميات فوق الأعمدة */
  const verticalCrowded =
    !horizontal && (categories.length > 4 || maxCategoryLen > 8 || series.length > 1);
  const showDataLabels = horizontal
    ? false
    : !verticalCrowded && categories.length <= 5;

  const formatTick = (v: number) => {
    if (valueFormatter) return valueFormatter(Number(v));
    if (integerAxis) return Math.round(Number(v)).toLocaleString('en-US');
    return Number(v).toLocaleString('en-US', { maximumFractionDigits: v >= 100 ? 0 : 1 });
  };

  /** محور القيم السفلي — مختصر عند الآلاف؛ أخفِ الكسور لتجنب تكرار 0/1 */
  const formatValueAxisTick = (v: number) => {
    const n = Number(v);
    if (!Number.isFinite(n)) return '';
    if (useIntegerValueAxis) {
      if (Math.abs(n - Math.round(n)) > 0.001) return '';
      return Math.round(n).toLocaleString('en-US');
    }
    if (compactValueAxis) {
      const abs = Math.abs(n);
      if (abs >= 1_000_000) {
        const m = n / 1_000_000;
        return `${m % 1 === 0 ? m.toFixed(0) : m.toFixed(1)}M`;
      }
      if (abs >= 1000) {
        const k = n / 1000;
        return `${k % 1 === 0 ? k.toFixed(0) : k.toFixed(1)}k`;
      }
    }
    if (horizontal && Math.abs(n - Math.round(n)) > 0.001) return '';
    return n.toLocaleString('en-US', { maximumFractionDigits: n >= 100 ? 0 : 1 });
  };

  const formatBarLabel = (v: number) => {
    if (integerAxis && !horizontal) return Math.round(Number(v)).toLocaleString('en-US');
    return formatTick(Number(v));
  };

  const labelFont = horizontal ? '11px' : '9px';
  const tickFont = horizontal ? '10px' : '9px';
  const categoryLabelWidth = horizontal ? horizontalCategoryLabelWidth(categories) : 0;
  /** احترم height الممرَّر لاستواء البطاقات؛ لا تصغّر المخطط تحت ارتفاع الصف */
  const resolvedHeight = horizontal
    ? Math.max(height, horizontalBarChartHeight(categories.length, series.length))
    : height;

  return (
    <ApexHost
      type="bar"
      height={resolvedHeight}
      series={series.map((s) => ({ name: s.name, data: s.data }))}
      options={{
        colors,
        chart: {
          parentHeightOffset: 0,
          offsetX: 0,
          offsetY: 0,
          sparkline: { enabled: false },
          // الظل يوسّع حدود الرسم ويخلق فراغاً بصرياً — عطّله للأفقي
          dropShadow: horizontal
            ? { enabled: false }
            : {
                enabled: true,
                top: 2,
                left: 0,
                blur: 4,
                opacity: 0.12,
                color: primary
              }
        },
        fill: {
          type: 'gradient',
          gradient: {
            shade: 'light',
            type: horizontal ? 'horizontal' : 'vertical',
            shadeIntensity: 0.28,
            opacityFrom: 1,
            opacityTo: 0.82,
            stops: [0, 88, 100]
          }
        },
        plotOptions: {
          bar: {
            horizontal,
            borderRadius: 3,
            borderRadiusApplication: 'end',
            columnWidth: series.length > 1 ? '72%' : categories.length <= 4 ? '52%' : '64%',
            barHeight: horizontal
              ? (series.length > 1 ? '82%' : '78%')
              : undefined,
            distributed: distributed && series.length === 1,
            dataLabels: { position: 'center' }
          }
        },
        dataLabels: {
          enabled: showDataLabels,
          formatter: (val: number) => formatBarLabel(Number(val)),
          style: {
            fontSize: labelFont,
            fontWeight: 700,
            colors: horizontal ? [onPrimary] : [ink],
            fontFamily: '"Segoe UI Variable", "Segoe UI", Tahoma, Arial, sans-serif'
          },
          background: { enabled: false },
          offsetY: horizontal ? 0 : -4,
          offsetX: 0,
          dropShadow: { enabled: false }
        },
        grid: {
          borderColor: border,
          strokeDashArray: 3,
          xaxis: { lines: { show: horizontal } },
          yaxis: { lines: { show: !horizontal } },
          // left صغير: Apex يحجز عرض yaxis من maxWidth — لا نضاعف الهامش
          // top/bottom سالب يقلّص الفراغ الشاسع داخل البطاقة
          padding: {
            left: horizontal ? 2 : 2,
            right: horizontal ? 12 : 4,
            top: horizontal ? -12 : showDataLabels ? 14 : 4,
            bottom: horizontal ? -8 : verticalCrowded ? 12 : 2
          }
        },
        stroke: {
          show: true,
          width: 1,
          colors: [border]
        },
        xaxis: {
          type: 'category',
          categories,
          tickPlacement: 'on',
          ...(horizontal && useIntegerValueAxis
            ? {
                min: 0,
                max: intAxisMax,
                tickAmount: intTickAmount,
                decimalsInFloat: 0
              }
            : horizontal && compactValueAxis
              ? {
                  tickAmount: 4,
                  decimalsInFloat: 0
                }
              : {}),
          labels: horizontal
            ? {
                show: true,
                rotate: 0,
                hideOverlappingLabels: true,
                trim: false,
                maxHeight: 28,
                style: { colors: muted, fontWeight: 600, fontSize: tickFont },
                formatter: (v: string) => formatValueAxisTick(Number(v))
              }
            : {
                show: true,
                rotate: verticalCrowded ? -28 : 0,
                rotateAlways: verticalCrowded,
                hideOverlappingLabels: true,
                trim: false,
                maxHeight: verticalCrowded ? 56 : 36,
                style: {
                  colors: categories.map(() => muted),
                  fontWeight: 600,
                  fontSize: tickFont
                },
                formatter: (v: string) => truncateAxisLabel(v, verticalCrowded ? 12 : 16)
              },
          axisBorder: { show: true, color: border, height: 1 },
          axisTicks: { show: true, color: border }
        },
        yaxis: horizontal
          ? {
              // align right يضع التسمية بجانب العمود؛ العرض الكافي + offset يمنع التداخل العربي
              labels: {
                show: true,
                align: 'right',
                maxWidth: categoryLabelWidth,
                offsetX: -8,
                style: {
                  colors: ink,
                  fontWeight: 600,
                  fontSize: labelFont,
                  cssClass: 'apex-hbar-ylabel'
                },
                formatter: (v: string | number) => String(v ?? '').trim()
              }
            }
          : integerAxis
            ? {
                min: 0,
                max: intAxisMax,
                tickAmount: intTickAmount,
                forceNiceScale: false,
                decimalsInFloat: 0,
                labels: {
                  show: true,
                  formatter: (v: number) => {
                    const n = Number(v);
                    if (!Number.isFinite(n)) return '';
                    if (Math.abs(n - Math.round(n)) > 0.001) return '';
                    return Math.round(n).toLocaleString('en-US');
                  },
                  style: { colors: muted, fontWeight: 600, fontSize: tickFont }
                }
              }
            : {
                labels: {
                  formatter: (v: number) => formatTick(Number(v)),
                  style: { colors: muted, fontWeight: 600, fontSize: tickFont }
                }
              },
        legend: {
          show: series.length > 1,
          position: 'bottom',
          horizontalAlign: 'center',
          fontSize: '10px',
          fontWeight: 600,
          itemMargin: { horizontal: 14, vertical: 4 },
          markers: { size: 5, offsetX: -2 },
          offsetY: 0
        },
        tooltip: {
          shared: series.length > 1,
          intersect: series.length === 1,
          style: { fontSize: '11px' },
          y: {
            formatter: (v: number) => formatTick(Number(v))
          }
        }
      }}
    />
  );
}

export function ApexAreaChart({
  categories,
  series,
  height = 280,
  valueFormatter
}: {
  categories: string[];
  series: { name: string; data: number[]; color?: string }[];
  height?: number;
  valueFormatter?: (v: number) => string;
}) {
  const { brandColors, primary, muted, border, card } = useThemeInk();
  const colors = series.map((s, i) => s.color || brandColors[i % brandColors.length]);
  // محور زمني بتسميات محدودة: 8–12 تسمية ظاهرة كحد أقصى مهما طال الفترة.
  // يمكن أن يحتوي الرسم نقاطاً أكثر من التسميات — لكن لا نطبع 30/60/90 تاريخاً فوق بعضها.
  const timeTickAmount = categories.length <= 12
    ? undefined
    : Math.min(12, Math.max(4, Math.ceil(categories.length / 8)));
  return (
    <ApexHost
      type="area"
      height={height}
      series={series.map((s) => ({ name: s.name, data: s.data }))}
      options={{
        colors,
        chart: {
          dropShadow: {
            enabled: true,
            top: 1,
            left: 0,
            blur: 3,
            opacity: 0.1,
            color: primary
          }
        },
        stroke: { curve: 'smooth', width: 2.8 },
        fill: {
          type: 'gradient',
          gradient: {
            shadeIntensity: 0.4,
            opacityFrom: 0.42,
            opacityTo: 0.05,
            stops: [0, 85, 100]
          }
        },
        markers: {
          size: 4,
          strokeWidth: 2,
          strokeColors: card,
          colors,
          hover: { size: 6 }
        },
        grid: {
          borderColor: border,
          strokeDashArray: 3,
          // مساحة للمحور الرقمي حتى لا يتداخل مع الخط/النقاط
          padding: { left: 14, right: 14, top: 10, bottom: 6 }
        },
        xaxis: {
          categories,
          ...(timeTickAmount ? { tickAmount: timeTickAmount } : {}),
          labels: {
            style: { colors: muted, fontWeight: 600, fontSize: '10px' },
            rotate: 0,
            rotateAlways: false,
            hideOverlappingLabels: true,
            trim: false,
            maxHeight: 32
          },
          axisBorder: { color: border },
          axisTicks: { color: border }
        },
        yaxis: {
          labels: {
            minWidth: 40,
            maxWidth: 64,
            offsetX: -2,
            style: { colors: muted, fontWeight: 600, fontSize: '10px' },
            // المحور مختصر؛ التفاصيل (مثل SAR) في التلميح فقط
            formatter: (v: number) => {
              const n = Number(v);
              if (!Number.isFinite(n)) return '';
              if (Math.abs(n) >= 1000) {
                const k = n / 1000;
                return `${k % 1 === 0 ? k.toFixed(0) : k.toFixed(1)}k`;
              }
              return n.toLocaleString('en-US', { maximumFractionDigits: 0 });
            }
          }
        },
        tooltip: {
          y: {
            formatter: (v: number) => (valueFormatter ? valueFormatter(Number(v)) : Number(v).toLocaleString('en-US'))
          }
        }
      }}
    />
  );
}

export function ApexDonutChart({
  labels,
  values,
  centerLabel,
  height = 280
}: {
  labels: string[];
  values: number[];
  centerLabel?: string;
  height?: number;
}) {
  const { ink, muted, card } = useThemeInk();
  const total = values.reduce((a, b) => a + b, 0);
  return (
    <ApexHost
      type="donut"
      height={height}
      series={values}
      options={{
        labels,
        plotOptions: {
          pie: {
            donut: {
              size: '68%',
              labels: {
                show: true,
                name: { show: true, fontSize: '12px', fontWeight: 600, color: muted },
                value: {
                  show: true,
                  fontSize: '22px',
                  fontWeight: 800,
                  color: ink,
                  formatter: (v: string) => Number(v).toLocaleString('en-US')
                },
                total: {
                  show: true,
                  label: centerLabel || 'الإجمالي',
                  fontSize: '11px',
                  fontWeight: 700,
                  color: muted,
                  formatter: () => total.toLocaleString('en-US')
                }
              }
            }
          }
        },
        stroke: { width: 2, colors: [card] },
        legend: { position: 'bottom' },
        dataLabels: { enabled: false },
        tooltip: {
          y: { formatter: (v: number) => Number(v).toLocaleString('en-US') }
        }
      }}
    />
  );
}

export function ApexRadialSet({
  items,
  height = 220
}: {
  items: { label: string; percent: number; color?: string }[];
  height?: number;
}) {
  const { brandColors, ink, muted, track } = useThemeInk();
  return (
    <ApexHost
      type="radialBar"
      height={height}
      series={items.map((i) => Math.max(0, Math.min(100, i.percent)))}
      options={{
        colors: items.map((i, idx) => i.color || brandColors[idx % brandColors.length]),
        plotOptions: {
          radialBar: {
            hollow: { size: '28%' },
            track: { background: track, margin: 8 },
            dataLabels: {
              name: { fontSize: '12px', fontWeight: 700, color: muted },
              value: {
                fontSize: '16px',
                fontWeight: 800,
                color: ink,
                formatter: (v: number) => `${Math.round(v)}%`
              },
              total: {
                show: items.length > 1,
                label: 'متوسط',
                formatter: (w: any) => {
                  const arr: number[] = w.globals.seriesTotals || [];
                  const avg = arr.length ? arr.reduce((a, b) => a + b, 0) / arr.length : 0;
                  return `${Math.round(avg)}%`;
                }
              }
            }
          }
        },
        labels: items.map((i) => i.label),
        legend: { show: true, position: 'bottom' },
        stroke: { lineCap: 'round' }
      }}
    />
  );
}

export function ApexSparkline({
  points,
  color,
  height = 42
}: {
  points: number[];
  color?: string;
  height?: number;
}) {
  const { accent } = useThemeInk();
  if (!points.length) return null;
  return (
    <ApexHost
      type="area"
      height={height}
      series={[{ name: 'trend', data: points }]}
      options={{
        colors: [color || accent],
        chart: { sparkline: { enabled: true } },
        stroke: { curve: 'smooth', width: 2 },
        fill: {
          type: 'gradient',
          gradient: { opacityFrom: 0.35, opacityTo: 0.02, stops: [0, 100] }
        },
        tooltip: { enabled: false },
        xaxis: { labels: { show: false }, axisBorder: { show: false }, axisTicks: { show: false } },
        yaxis: { show: false },
        grid: { show: false },
        legend: { show: false }
      }}
    />
  );
}

export function useChartToneColors() {
  const brandColors = useThemeBrandColors();
  return {
    primary: brandColors[0],
    accent: brandColors[1],
    palette: brandColors
  };
}

export { FALLBACK_COLORS as BRAND_COLORS };
