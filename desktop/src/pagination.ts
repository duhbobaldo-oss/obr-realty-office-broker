/** حجم صفحة الجداول المعروضة — بدون سقف على عدد الصفحات. */
export const TABLE_PAGE_SIZE = 20;

export type PageToken = { kind: 'page'; n: number } | { kind: 'gap' };

export function totalPages(total: number, pageSize: number = TABLE_PAGE_SIZE): number {
  if (!Number.isFinite(total) || total <= 0) return 0;
  if (!Number.isFinite(pageSize) || pageSize <= 0) return 0;
  return Math.ceil(total / pageSize);
}

export function clampPage(page: number, total: number, pageSize: number = TABLE_PAGE_SIZE): number {
  const pages = totalPages(total, pageSize);
  if (pages <= 0) return 1;
  if (!Number.isFinite(page) || page < 1) return 1;
  return Math.min(page, pages);
}

export function slicePage<T>(items: T[], page: number, pageSize: number = TABLE_PAGE_SIZE): T[] {
  const safe = clampPage(page, items.length, pageSize);
  const start = (safe - 1) * pageSize;
  return items.slice(start, start + pageSize);
}

/**
 * ترقيم مرتب بلا حد أقصى لعدد الصفحات:
 * صفحات قليلة → كلها.
 * صفحات كثيرة → الأولى، نافذة حول الحالية، الأخيرة، مع فجوات.
 */
export function arrangedPageItems(current: number, pages: number, windowSize = 5): PageToken[] {
  if (pages <= 0) return [];
  const cur = Math.min(Math.max(1, current), pages);
  const span = Math.max(3, windowSize);

  if (pages <= span + 2) {
    return Array.from({ length: pages }, (_, i) => ({ kind: 'page' as const, n: i + 1 }));
  }

  const half = Math.floor(span / 2);
  let start = Math.max(1, cur - half);
  let end = Math.min(pages, start + span - 1);
  start = Math.max(1, end - span + 1);

  const tokens: PageToken[] = [];
  if (start > 1) {
    tokens.push({ kind: 'page', n: 1 });
    if (start > 2) tokens.push({ kind: 'gap' });
  }
  for (let n = start; n <= end; n += 1) {
    tokens.push({ kind: 'page', n });
  }
  if (end < pages) {
    if (end < pages - 1) tokens.push({ kind: 'gap' });
    tokens.push({ kind: 'page', n: pages });
  }
  return tokens;
}
