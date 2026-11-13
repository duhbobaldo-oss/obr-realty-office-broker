import React, { useEffect, useState } from 'react';
import classicLogo from '../assets/brand/logo-classic.png';
import archiveLogo from '../assets/brand/logo-archive.png';
import systemLogo from '../assets/brand/logo-system.png';
import { currentTheme, type ThemeId } from '../theme';

const LOGO_BY_THEME: Record<ThemeId, string> = {
  classic: classicLogo,
  archive: archiveLogo,
  system: systemLogo
};

/** شعار مخصص لكل ثيم (كلاسيكي / أرشيف / نظامي). */
export default function BrandLogo({ className = '' }: { className?: string }) {
  const [theme, setTheme] = useState<ThemeId>(() => currentTheme());

  useEffect(() => {
    const sync = () => setTheme(currentTheme());
    window.addEventListener('obr-theme-changed', sync);
    return () => window.removeEventListener('obr-theme-changed', sync);
  }, []);

  return <img className={className} src={LOGO_BY_THEME[theme]} alt="" />;
}
