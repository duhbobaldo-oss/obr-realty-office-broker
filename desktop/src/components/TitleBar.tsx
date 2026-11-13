import React, { useEffect, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import BrandLogo from './BrandLogo';
import { getAppTitle } from '../title';

/** شريط علوي مخصص — يعرض اسم المكتب من الإعدادات. */
export default function TitleBar() {
  const win = getCurrentWindow();
  const [title, setTitle] = useState(getAppTitle());

  useEffect(() => {
    const onTitle = (e: Event) => {
      const t = (e as CustomEvent).detail;
      if (typeof t === 'string' && t.trim()) setTitle(t.trim());
    };
    window.addEventListener('mawasib:title', onTitle);
    return () => window.removeEventListener('mawasib:title', onTitle);
  }, []);

  return (
    <div className="titlebar" data-tauri-drag-region>
      <div className="titlebar-brand" data-tauri-drag-region>
        <span className="titlebar-logo">
          <BrandLogo />
        </span>
        <span className="titlebar-text">{title}</span>
      </div>
      <div className="titlebar-controls">
        <button className="tb-btn" title="تصغير" onClick={() => win.minimize()}>
          <svg width="12" height="12" viewBox="0 0 12 12"><path d="M1 6h10" stroke="currentColor" strokeWidth="1.4" /></svg>
        </button>
        <button className="tb-btn" title="تكبير / استعادة" onClick={() => win.toggleMaximize()}>
          <svg width="12" height="12" viewBox="0 0 12 12"><rect x="1.5" y="1.5" width="9" height="9" fill="none" stroke="currentColor" strokeWidth="1.4" rx="1" /></svg>
        </button>
        <button className="tb-btn tb-close" title="إغلاق" onClick={() => win.close()}>
          <svg width="12" height="12" viewBox="0 0 12 12"><path d="M2 2l8 8M10 2l-8 8" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" /></svg>
        </button>
      </div>
    </div>
  );
}
