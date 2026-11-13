import React, { useEffect, useId, useLayoutEffect, useRef, useState } from 'react';
import { CircleHelp } from 'lucide-react';
import { guideById, type SectionGuide } from '../domain/sectionGuides';
import { getUiLanguage } from '../domain/locale';

type Props = {
  id: string;
  guide?: SectionGuide;
  compact?: boolean;
};

type PopPos = { top: number; left: number; placeAbove: boolean };

/** علامة استفهام بجانب العنوان؛ النافذة تُثبَّت بـ fixed لتبتعد عن الشريط الجانبي. */
export default function SectionGuideBanner({ id, guide }: Props) {
  const g = guide || guideById(id);
  const tipId = useId();
  const rootRef = useRef<HTMLSpanElement>(null);
  const popRef = useRef<HTMLDivElement>(null);
  const closeTimer = useRef<number | null>(null);
  const [open, setOpen] = useState(false);
  const [pos, setPos] = useState<PopPos | null>(null);
  const isLtr = getUiLanguage() === 'en';

  const clearCloseTimer = () => {
    if (closeTimer.current != null) {
      window.clearTimeout(closeTimer.current);
      closeTimer.current = null;
    }
  };

  const computePos = (): PopPos | null => {
    const el = rootRef.current;
    if (!el) return null;
    const rect = el.getBoundingClientRect();
    const spaceBelow = window.innerHeight - rect.bottom;
    const placeAbove = spaceBelow < 240 && rect.top > spaceBelow;
    const popW = Math.min(380, window.innerWidth * 0.86);
    const gap = 10;
    let left: number;
    if (isLtr) {
      // ابتعد عن الشريط الجانبي الأيسر: ابدأ من الزر وادفع لليمين إن لزم
      const sidebarSafe = 248;
      left = Math.max(rect.left, sidebarSafe);
      if (left + popW > window.innerWidth - 12) {
        left = Math.max(12, window.innerWidth - popW - 12);
      }
    } else {
      left = rect.right - popW;
      if (left < 12) left = 12;
    }
    return {
      top: placeAbove ? rect.top - gap : rect.bottom + gap,
      left,
      placeAbove
    };
  };

  const openTip = () => {
    clearCloseTimer();
    setPos(computePos());
    setOpen(true);
  };

  const scheduleClose = () => {
    clearCloseTimer();
    closeTimer.current = window.setTimeout(() => setOpen(false), 140);
  };

  useEffect(() => () => clearCloseTimer(), []);

  useLayoutEffect(() => {
    if (!open) return;
    setPos(computePos());
    const onWin = () => setPos(computePos());
    window.addEventListener('resize', onWin);
    window.addEventListener('scroll', onWin, true);
    return () => {
      window.removeEventListener('resize', onWin);
      window.removeEventListener('scroll', onWin, true);
    };
  }, [open, isLtr]);

  useEffect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      const t = e.target as Node;
      if (rootRef.current?.contains(t) || popRef.current?.contains(t)) return;
      setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setOpen(false);
    };
    document.addEventListener('mousedown', onDoc);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('mousedown', onDoc);
      document.removeEventListener('keydown', onKey);
    };
  }, [open]);

  if (!g) return null;

  const helpAria =
    getUiLanguage() === 'en' ? `Help for section ${g.title}` : `تعليمات قسم ${g.title}`;

  const popStyle: React.CSSProperties | undefined = pos
    ? {
        position: 'fixed',
        left: pos.left,
        top: pos.top,
        width: Math.min(380, window.innerWidth * 0.86),
        zIndex: 6000,
        transform: pos.placeAbove ? 'translateY(-100%)' : undefined
      }
    : undefined;

  return (
    <span
      ref={rootRef}
      className={`section-guide-hint${open ? ' is-open' : ''}`}
      onMouseEnter={openTip}
      onMouseLeave={scheduleClose}
    >
      <button
        type="button"
        className="section-guide-hint-btn"
        aria-label={helpAria}
        aria-describedby={open ? tipId : undefined}
        aria-expanded={open}
        onClick={(e) => {
          e.preventDefault();
          e.stopPropagation();
          if (open) setOpen(false);
          else openTip();
        }}
        onFocus={openTip}
        onBlur={scheduleClose}
      >
        <CircleHelp size={16} strokeWidth={1.75} aria-hidden />
      </button>
      {open && pos && (
        <div
          ref={popRef}
          id={tipId}
          className="section-guide-hint-pop is-fixed"
          role="tooltip"
          dir={isLtr ? 'ltr' : 'rtl'}
          style={popStyle}
          onMouseEnter={openTip}
          onMouseLeave={scheduleClose}
        >
          <div className="section-guide-hint-title">{g.title}</div>
          {g.summary ? <p className="section-guide-hint-summary">{g.summary}</p> : null}
          {g.blocks && g.blocks.length > 0 && (
            <div className="section-guide-hint-blocks">
              {g.blocks.map((b) => (
                <div key={b.title} className="section-guide-hint-block">
                  <div className="section-guide-hint-block-title">{b.title}</div>
                  <p className="section-guide-hint-block-body">{b.body}</p>
                </div>
              ))}
            </div>
          )}
          {g.steps.length > 0 && (
            <ol className="section-guide-hint-steps">
              {g.steps.map((step, i) => (
                <li key={step}>
                  <span className="section-guide-hint-num" aria-hidden>{i + 1}</span>
                  <span>{step}</span>
                </li>
              ))}
            </ol>
          )}
        </div>
      )}
    </span>
  );
}
