import React, { useEffect, useState } from 'react';
import { ChevronLeft, ChevronRight } from 'lucide-react';
import {
  TABLE_PAGE_SIZE,
  arrangedPageItems,
  clampPage,
  slicePage,
  totalPages
} from '../pagination';

export function usePagedItems<T>(items: T[], pageSize: number = TABLE_PAGE_SIZE, resetKey?: unknown) {
  const [page, setPage] = useState(1);
  const total = items.length;
  const current = clampPage(page, total, pageSize);
  useEffect(() => {
    setPage(1);
  }, [resetKey]);
  useEffect(() => {
    if (page !== current) setPage(current);
  }, [page, current]);
  return {
    page: current,
    setPage,
    slice: slicePage(items, current, pageSize),
    total,
    pageSize
  };
}

type Props = {
  total: number;
  page: number;
  onPage: (page: number) => void;
  pageSize?: number;
  label?: string;
};

export default function PaginationBar({
  total,
  page,
  onPage,
  pageSize = TABLE_PAGE_SIZE,
  label = 'سجل'
}: Props) {
  const pages = totalPages(total, pageSize);
  const current = clampPage(page, total, pageSize);

  useEffect(() => {
    if (pages > 0 && page !== current) onPage(current);
  }, [pages, page, current, onPage]);

  if (pages <= 1) return null;

  const tokens = arrangedPageItems(current, pages);
  const from = (current - 1) * pageSize + 1;
  const to = Math.min(total, current * pageSize);

  return (
    <nav className="pagination" aria-label="ترقيم الصفحات">
      <button
        type="button"
        className="page-btn"
        disabled={current <= 1}
        onClick={() => onPage(current - 1)}
        aria-label="الصفحة السابقة"
      >
        <ChevronRight size={14} /> السابق
      </button>

      {tokens.map((token, index) => {
        if (token.kind === 'gap') {
          return (
            <span key={`gap-${index}`} className="page-ellipsis" aria-hidden="true">
              …
            </span>
          );
        }
        const prev = tokens[index - 1];
        const showDot = prev?.kind === 'page';
        return (
          <React.Fragment key={token.n}>
            {showDot ? <span className="page-dot" aria-hidden="true">·</span> : null}
            <button
              type="button"
              className={`page-btn ${token.n === current ? 'active' : ''}`}
              onClick={() => onPage(token.n)}
              aria-current={token.n === current ? 'page' : undefined}
              aria-label={`صفحة ${token.n}`}
            >
              {token.n}
            </button>
          </React.Fragment>
        );
      })}

      <button
        type="button"
        className="page-btn"
        disabled={current >= pages}
        onClick={() => onPage(current + 1)}
        aria-label="الصفحة التالية"
      >
        التالي <ChevronLeft size={14} />
      </button>

      <span className="page-info">
        {from}–{to} من {total} {label} — صفحة {current} من {pages}
      </span>
    </nav>
  );
}
