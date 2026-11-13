export const ARABIC_MONTHS = [
  'يناير', 'فبراير', 'مارس', 'أبريل', 'مايو', 'يونيو',
  'يوليو', 'أغسطس', 'سبتمبر', 'أكتوبر', 'نوفمبر', 'ديسمبر'
];

export const WEEKDAY_LABELS = ['س', 'ح', 'ن', 'ث', 'ر', 'خ', 'ج'];

export function isoDate(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}

export function parseIsoDate(value: string): Date {
  const [y, m, d] = value.split('-').map(Number);
  return new Date(y, (m || 1) - 1, d || 1);
}

export function monthLabel(year: number, monthIndex: number): string {
  return `${ARABIC_MONTHS[monthIndex]} ${year}`;
}

export function startOfMonth(year: number, monthIndex: number): Date {
  return new Date(year, monthIndex, 1);
}

export function addMonths(year: number, monthIndex: number, delta: number): { year: number; month: number } {
  const d = new Date(year, monthIndex + delta, 1);
  return { year: d.getFullYear(), month: d.getMonth() };
}

/** الأسبوع يبدأ السبت ليطابق تقويم المكتب، والعمود الأول يسارًا كما في النموذج */
export function monthCells(year: number, monthIndex: number): { iso: string; day: number; inMonth: boolean }[] {
  const first = startOfMonth(year, monthIndex);
  const saturdayIndex = (first.getDay() + 1) % 7;
  const daysInMonth = new Date(year, monthIndex + 1, 0).getDate();
  const prevMonthDays = new Date(year, monthIndex, 0).getDate();
  const cells: { iso: string; day: number; inMonth: boolean }[] = [];

  for (let i = 0; i < 42; i += 1) {
    const offset = i - saturdayIndex;
    if (offset < 0) {
      const day = prevMonthDays + offset + 1;
      const date = new Date(year, monthIndex - 1, day);
      cells.push({ iso: isoDate(date), day, inMonth: false });
    } else if (offset >= daysInMonth) {
      const day = offset - daysInMonth + 1;
      const date = new Date(year, monthIndex + 1, day);
      cells.push({ iso: isoDate(date), day, inMonth: false });
    } else {
      const day = offset + 1;
      const date = new Date(year, monthIndex, day);
      cells.push({ iso: isoDate(date), day, inMonth: true });
    }
  }
  return cells;
}
