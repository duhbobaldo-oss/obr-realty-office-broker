import React, { useEffect, useLayoutEffect, useRef, useState } from 'react';

export type AppMenuItem =
  | {
      id: string;
      label: string;
      shortcut?: string;
      danger?: boolean;
      disabled?: boolean;
      icon?: React.ReactNode;
      onClick: () => void;
    }
  | { id: string; separator: true };

type Props = {
  x: number;
  y: number;
  items: AppMenuItem[];
  onClose: () => void;
};

export default function AppContextMenu({ x, y, items, onClose }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ left: x, top: y });

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const w = el.offsetWidth;
    const h = el.offsetHeight;
    const left = Math.max(8, Math.min(x, window.innerWidth - w - 8));
    const top = Math.max(8, Math.min(y, window.innerHeight - h - 8));
    setPos({ left, top });
  }, [x, y, items.length]);

  useEffect(() => {
    const onDown = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('mousedown', onDown);
    window.addEventListener('keydown', onKey);
    window.addEventListener('blur', onClose);
    return () => {
      window.removeEventListener('mousedown', onDown);
      window.removeEventListener('keydown', onKey);
      window.removeEventListener('blur', onClose);
    };
  }, [onClose]);

  return (
    <div
      ref={ref}
      className="app-ctx-menu"
      role="menu"
      style={{ left: pos.left, top: pos.top }}
      onContextMenu={(e) => e.preventDefault()}
    >
      {items.map((item) =>
        'separator' in item ? (
          <div key={item.id} className="app-ctx-sep" role="separator" />
        ) : (
          <button
            key={item.id}
            type="button"
            role="menuitem"
            className={`app-ctx-item${item.danger ? ' is-danger' : ''}`}
            disabled={item.disabled}
            onClick={() => {
              if (item.disabled) return;
              item.onClick();
              onClose();
            }}
          >
            <span style={{ display: 'inline-flex', alignItems: 'center', gap: 8 }}>
              {'icon' in item && item.icon ? <span style={{ display: 'inline-flex', color: 'var(--primary)' }} aria-hidden>{item.icon}</span> : null}
              <span>{item.label}</span>
            </span>
            {item.shortcut ? <kbd>{item.shortcut}</kbd> : null}
          </button>
        )
      )}
    </div>
  );
}
