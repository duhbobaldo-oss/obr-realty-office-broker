import { brokerLicenseLabel, getCountry } from '../domain/countryProfile';
import React, { useEffect, useMemo, useRef, useState } from 'react';
import { NavLink, useLocation, useNavigate } from 'react-router-dom';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { api } from '../api';
import { getAppTitle, setAppTitle } from '../title';
import SysIcon, { SysIconName } from './SysIcon';
import AppContextMenu, { type AppMenuItem } from './AppContextMenu';
import BrandLogo from './BrandLogo';
import PageCopyrightFooter from './PageCopyrightFooter';
import OfficialSiteLink from './OfficialSiteLink';
import {
  applyDocumentLocale,
  currencySymbol,
  GCC_CURRENCIES,
  getDisplayCurrency,
  getUiLanguage,
  hydrateLocaleFromSettings,
  navLabelForPath,
  onLocaleChange,
  setLocalePrefs,
  t,
  tf,
  UI_LANGUAGES,
  type GccCurrencyCode,
  type UiLanguage
} from '../domain/locale';
import { applyTheme, currentTheme, THEMES, type ThemeId } from '../theme';
import { falTypeArLabel } from '../domain/labels';
import { Palette, Coins } from 'lucide-react';
import LangIcon from './LangIcon';

interface LayoutProps {
  auth: any;
  children: React.ReactNode;
}

const NAV_ITEMS: { path: string; icon: SysIconName; labelKey: string }[] = [
  { path: '/', icon: 'home', labelKey: 'dashboard' },
  { path: '/market', icon: 'graph-increase', labelKey: 'market' },
  { path: '/properties', icon: 'briefcase', labelKey: 'properties' },
  { path: '/maps', icon: 'location', labelKey: 'maps' },
  { path: '/clients', icon: 'users', labelKey: 'clients' },
  { path: '/campaigns', icon: 'target', labelKey: 'campaigns' },
  { path: '/finance', icon: 'wallet', labelKey: 'finance' },
  { path: '/brochures', icon: 'document-stack', labelKey: 'brochures' },
  { path: '/offer-design', icon: 'pen', labelKey: 'offer-design' },
  { path: '/database', icon: 'database', labelKey: 'database' },
  { path: '/settings', icon: 'settings', labelKey: 'settings' },
  { path: '/help', icon: 'info-circle', labelKey: 'help' }
];

function pageTitle(pathname: string): string {
  return navLabelForPath(pathname);
}

export default function Layout({ auth: _auth, children }: LayoutProps) {
  const location = useLocation();
  const navigate = useNavigate();
  const win = getCurrentWindow();
  const [officeName, setOfficeName] = useState(getAppTitle());
  const [falLicense, setFalLicense] = useState('');
  const [collapsed, setCollapsed] = useState(() => localStorage.getItem('obr_sidebar_collapsed') === '1');
  const [settingsError, setSettingsError] = useState('');
  const [followups, setFollowups] = useState(0);
  const [shellDataError, setShellDataError] = useState('');
  const [q, setQ] = useState('');
  const [searchOpen, setSearchOpen] = useState(false);
  const [searching, setSearching] = useState(false);
  const [searchErr, setSearchErr] = useState('');
  const [results, setResults] = useState<{ clients: any[]; properties: any[] }>({ clients: [], properties: [] });
  const [quickOpen, setQuickOpen] = useState(false);
  const [prefsOpen, setPrefsOpen] = useState<'theme' | 'lang' | 'currency' | null>(null);
  const [themeId, setThemeId] = useState<ThemeId>(() => currentTheme());
  const [uiLang, setUiLang] = useState<UiLanguage>(() => getUiLanguage());
  const [currency, setCurrency] = useState<GccCurrencyCode>(() => getDisplayCurrency());
  const [appMenu, setAppMenu] = useState<{ x: number; y: number; target: HTMLElement } | null>(null);
  const searchRef = useRef<HTMLDivElement>(null);
  const quickRef = useRef<HTMLDivElement>(null);
  const prefsRef = useRef<HTMLDivElement>(null);
  const mainContentRef = useRef<HTMLDivElement>(null);
  const pageScrollRef = useRef<HTMLElement>(null);
  const [chromeScrolled, setChromeScrolled] = useState(false);
  const [langTick, setLangTick] = useState(0);

  useEffect(() => {
    applyDocumentLocale();
    return onLocaleChange(() => {
      setLangTick((n) => n + 1);
      setUiLang(getUiLanguage());
      setCurrency(getDisplayCurrency());
    });
  }, []);

  const createBackupFromMenu = async () => {
    try {
      const data = await api('/api/backup/create', { method: 'POST' });
      window.alert(data?.message || t('ctx.backup-ok'));
    } catch (err: any) {
      window.alert(err?.message || t('ctx.backup-fail'));
    }
  };

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'F5') {
        const tag = (e.target as HTMLElement | null)?.tagName;
        if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return;
        e.preventDefault();
        window.location.reload();
        return;
      }
      if (!(e.ctrlKey || e.metaKey) || !e.shiftKey) return;
      const key = e.key.toLowerCase();
      if (key === 'd') {
        e.preventDefault();
        navigate('/offer-design');
      } else if (key === 'm') {
        e.preventDefault();
        navigate('/?appointment=new');
      } else if (key === 'l') {
        e.preventDefault();
        navigate('/properties/new');
      } else if (key === 'u') {
        e.preventDefault();
        navigate('/clients/new');
      } else if (key === 'o') {
        e.preventDefault();
        navigate('/finance', { state: { tab: 'contracts', newDraft: true } });
      } else if (key === 'b') {
        e.preventDefault();
        void createBackupFromMenu();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [navigate, langTick]);

  const currentTitle = useMemo(
    () => pageTitle(location.pathname),
    [location.pathname, langTick]
  );

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const d = await api('/api/settings');
        if (cancelled) return;
        const name = d.office_name || getAppTitle();
        setOfficeName(name);
        setAppTitle(name);
        hydrateLocaleFromSettings(d);
        setUiLang(getUiLanguage());
        setCurrency(getDisplayCurrency());
        const num = d.fal_license_number;
        const typ = brokerLicenseLabel(d.country || getCountry(), d.fal_license_type, d.broker_license_name);
        setFalLicense(num || typ ? [num, typ].filter(Boolean).join(' — ') : '');
        setSettingsError('');
      } catch (err: any) {
        if (!cancelled) setSettingsError(err?.message || 'تعذر تحميل إعدادات المكتب');
      }
    })();

    const onTitle = (e: Event) => {
      const t = (e as CustomEvent).detail;
      if (typeof t === 'string' && t.trim()) setOfficeName(t.trim());
    };
    const onOffice = (e: Event) => {
      const d = (e as CustomEvent).detail || {};
      if (typeof d.office_name === 'string') {
        const name = d.office_name.trim() || getAppTitle();
        setOfficeName(name);
      }
      const num = d.fal_license_number;
      const typ = brokerLicenseLabel(d.country || getCountry(), d.fal_license_type, d.broker_license_name);
      if (num !== undefined || typ !== undefined) {
        setFalLicense(num || typ ? [num, typ].filter(Boolean).join(' — ') : '');
      }
    };
    window.addEventListener('mawasib:title', onTitle);
    window.addEventListener('mawasib:office-settings', onOffice);
    return () => {
      cancelled = true;
      window.removeEventListener('mawasib:title', onTitle);
      window.removeEventListener('mawasib:office-settings', onOffice);
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const stats = await api('/api/dashboard/stats');
        if (cancelled) return;
        setFollowups(stats?.clients?.needs_followup || 0);
        setShellDataError('');
      } catch (err: any) {
        if (!cancelled) {
          setShellDataError(err?.message || 'تعذر تحديث حالة الشريط العلوي');
        }
      }
    })();
    return () => { cancelled = true; };
  }, [location.pathname]);

  useEffect(() => {
    localStorage.setItem('obr_sidebar_collapsed', collapsed ? '1' : '0');
  }, [collapsed]);

  useEffect(() => {
    setChromeScrolled(false);
    if (mainContentRef.current) mainContentRef.current.scrollTop = 0;
    if (pageScrollRef.current) pageScrollRef.current.scrollTop = 0;
  }, [location.pathname]);

  const syncChromeScroll = () => {
    const shellTop = mainContentRef.current?.scrollTop ?? 0;
    const pageTop = pageScrollRef.current?.scrollTop ?? 0;
    setChromeScrolled(Math.max(shellTop, pageTop) > 8);
  };

  useEffect(() => {
    const onDocClick = (e: MouseEvent) => {
      const t = e.target as Node;
      if (searchRef.current && !searchRef.current.contains(t)) setSearchOpen(false);
      if (quickRef.current && !quickRef.current.contains(t)) setQuickOpen(false);
      if (prefsRef.current && !prefsRef.current.contains(t)) setPrefsOpen(null);
    };
    document.addEventListener('mousedown', onDocClick);
    return () => document.removeEventListener('mousedown', onDocClick);
  }, []);

  const persistLocale = async (next: { language?: UiLanguage }) => {
    setLocalePrefs(next);
    if (next.language) setUiLang(next.language);
    setPrefsOpen(null);
    try {
      await api('/api/settings', {
        method: 'PUT',
        body: {
          ui_language: next.language ?? getUiLanguage()
        }
      });
    } catch {
      /* التطبيق المحلي تم؛ الحفظ على الخادم يُعاد من الإعدادات إن لزم */
    }
  };

  const pickTheme = (id: ThemeId) => {
    applyTheme(id);
    setThemeId(id);
    setPrefsOpen(null);
  };

  const runSearch = async (value: string) => {
    setQ(value);
    if (!value.trim()) {
      setResults({ clients: [], properties: [] });
      setSearchErr('');
      return;
    }
    setSearching(true);
    setSearchErr('');
    try {
      const [c, p] = await Promise.all([
        api(`/api/clients?search=${encodeURIComponent(value)}&limit=8`),
        api(`/api/properties?search=${encodeURIComponent(value)}&limit=8`)
      ]);
      setResults({
        clients: c.items || c || [],
        properties: p.items || p || []
      });
    } catch (err: any) {
      setSearchErr(err?.message || 'فشل البحث');
      setResults({ clients: [], properties: [] });
    } finally {
      setSearching(false);
    }
  };

  const resultCount = useMemo(
    () => (results.clients?.length || 0) + (results.properties?.length || 0),
    [results]
  );

  const appMenuItems = useMemo((): AppMenuItem[] => {
    const target = appMenu?.target || null;
    const field =
      target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement
        ? target
        : (target?.closest('input, textarea') as HTMLInputElement | HTMLTextAreaElement | null);
    const editable = !!field && !field.readOnly && !field.disabled;
    return [
      {
        id: 'copy',
        label: t('copy'),
        shortcut: 'Ctrl+C',
        onClick: async () => {
          const selected = window.getSelection()?.toString()
            || (field ? field.value.slice(field.selectionStart ?? 0, field.selectionEnd ?? 0) : '');
          if (selected) await navigator.clipboard.writeText(selected);
        }
      },
      {
        id: 'paste',
        label: t('paste'),
        shortcut: 'Ctrl+V',
        disabled: !editable,
        onClick: async () => {
          if (!field || !editable) return;
          const text = await navigator.clipboard.readText();
          const start = field.selectionStart ?? field.value.length;
          const end = field.selectionEnd ?? start;
          field.setRangeText(text, start, end, 'end');
          field.dispatchEvent(new Event('input', { bubbles: true }));
        }
      },
      {
        id: 'select',
        label: t('select_all'),
        shortcut: 'Ctrl+A',
        disabled: !field,
        onClick: () => field?.select()
      },
      { id: 'sep-edit', separator: true },
      {
        id: 'refresh-app',
        label: t('ctx.refresh-app'),
        shortcut: 'F5',
        onClick: () => {
          window.location.reload();
        }
      },
      { id: 'sep-actions', separator: true },
      {
        id: 'offer-pdf',
        label: t('ctx.offer-pdf'),
        shortcut: 'Ctrl+Shift+D',
        onClick: () => navigate('/offer-design')
      },
      {
        id: 'add-appointment',
        label: t('ctx.add-appointment'),
        shortcut: 'Ctrl+Shift+M',
        onClick: () => navigate('/?appointment=new')
      },
      {
        id: 'add-listing',
        label: t('ctx.add-listing'),
        shortcut: 'Ctrl+Shift+L',
        onClick: () => navigate('/properties/new')
      },
      {
        id: 'add-client',
        label: t('ctx.add-client'),
        shortcut: 'Ctrl+Shift+U',
        onClick: () => navigate('/clients/new')
      },
      {
        id: 'add-contract',
        label: t('ctx.add-contract'),
        shortcut: 'Ctrl+Shift+O',
        onClick: () => navigate('/finance', { state: { tab: 'contracts', newDraft: true } })
      },
      { id: 'sep-backup', separator: true },
      {
        id: 'backup',
        label: t('ctx.backup'),
        shortcut: 'Ctrl+Shift+B',
        onClick: () => {
          void createBackupFromMenu();
        }
      }
    ];
  }, [appMenu, navigate, langTick]);

  return (
    <div
      className={`app-layout app-layout-sidebar ${collapsed ? 'sidebar-collapsed' : ''}`}
      onContextMenu={(e) => {
        e.preventDefault();
        const t = e.target as HTMLElement;
        if (t.closest('.ods-studio, .app-ctx-menu, .modal-overlay, .mail-compose-modal')) return;
        setAppMenu({ x: e.clientX, y: e.clientY, target: t });
      }}
    >
      <aside className="sidebar" aria-label={t('main-nav')}>
        <div className="sidebar-header">
          <div className="sidebar-brand" title={officeName}>
            <span className="sidebar-logo">
              <BrandLogo />
            </span>
            {!collapsed && (
              <div className="sidebar-brand-text">
                <div className="sidebar-title">{officeName}</div>
                {falLicense ? <div className="sidebar-fal">{t('fal')}: {falLicense}</div> : null}
              </div>
            )}
          </div>
          <button
            type="button"
            className="sidebar-collapse-btn"
            title={collapsed ? t('expand-sidebar') : t('collapse-sidebar')}
            onClick={() => setCollapsed((v) => !v)}
          >
            <SysIcon name={collapsed ? 'chevron-left' : 'chevron-right'} size={16} />
          </button>
        </div>

        <nav className="sidebar-nav">
          {NAV_ITEMS.map((item) => (
            <NavLink
              key={item.path}
              to={item.path}
              end={item.path === '/'}
              className={({ isActive }) => `nav-item ${isActive ? 'active' : ''}`}
              title={t(item.labelKey)}
            >
              <span className="nav-icon"><SysIcon name={item.icon} size={18} /></span>
              {!collapsed && <span className="nav-label">{t(item.labelKey)}</span>}
            </NavLink>
          ))}
        </nav>

        <div className="sidebar-footer">
          <div className="sf-brand">
            <BrandLogo className="sf-logo" />
            {!collapsed && (
              <div className="sf-legal">
                <div className="sf-copy">
                  <OfficialSiteLink className="sf-email" />
                </div>
              </div>
            )}
          </div>
          {!collapsed && (
            <div className="sf-meta">
              <div className="sf-name">{t('app-name')}</div>
              <div className="sf-version">{t('version')}</div>
            </div>
          )}
        </div>
      </aside>

      <div className="main-content scroll-main" ref={mainContentRef} onScroll={syncChromeScroll}>
        <header className={`app-chrome top-action-bar${chromeScrolled ? ' is-scrolled' : ''}`}>
          <div className="chrome-start" data-tauri-drag-region>
            <div className="workspace-title chrome-page-title">{currentTitle}</div>
          </div>

          <div className="chrome-actions">
            <div className="top-search" ref={searchRef}>
              <SysIcon name="search" size={15} />
              <input
                value={q}
                onChange={(e) => runSearch(e.target.value)}
                onFocus={() => setSearchOpen(true)}
                placeholder={t('search')}
                aria-label={t('search')}
              />
              {searchOpen && (q.trim() || searching || searchErr) && (
                <div className="top-search-panel">
                  {searching && <div className="search-hint">{t('searching')}</div>}
                  {searchErr && <div className="search-hint error">{searchErr}</div>}
                  {!searching && !searchErr && q.trim() && resultCount === 0 && (
                    <div className="search-hint">{t('no-results')}</div>
                  )}
                  {results.clients?.map((c: any) => (
                    <button
                      key={`c-${c.id}`}
                      type="button"
                      className="search-hit"
                      onClick={() => { navigate(`/clients/${c.id}`); setSearchOpen(false); setQ(''); }}
                    >
                      <SysIcon name="user" size={14} />
                      <span>{c.name}</span>
                      <em>{t('client-tag')}</em>
                    </button>
                  ))}
                  {results.properties?.map((p: any) => (
                    <button
                      key={`p-${p.id}`}
                      type="button"
                      className="search-hit"
                      onClick={() => { navigate(`/properties/${p.id}`); setSearchOpen(false); setQ(''); }}
                    >
                      <SysIcon name="briefcase" size={14} />
                      <span>{p.name}</span>
                      <em>{t('listing-tag')}</em>
                    </button>
                  ))}
                </div>
              )}
            </div>

            <div className="top-quick" ref={quickRef}>
              <button type="button" className="chrome-action-btn" title={t('quick-add')} onClick={() => setQuickOpen((v) => !v)}>
                <SysIcon name="plus" size={16} />
              </button>
              {quickOpen && (
                <div className="quick-menu" role="menu">
                  <button type="button" role="menuitem" onClick={() => { navigate('/clients/new'); setQuickOpen(false); }}>
                    <span style={{ display: 'inline-flex', color: 'var(--primary)' }} aria-hidden><SysIcon name="user" size={15} /></span>
                    <span>{t('new-client')}</span>
                  </button>
                  <button type="button" role="menuitem" onClick={() => { navigate('/properties/new'); setQuickOpen(false); }}>
                    <span style={{ display: 'inline-flex', color: 'var(--primary)' }} aria-hidden><SysIcon name="briefcase" size={15} /></span>
                    <span>{t('new-listing')}</span>
                  </button>
                  <button type="button" role="menuitem" onClick={() => { navigate('/finance', { state: { tab: 'contracts', newDraft: true } }); setQuickOpen(false); }}>
                    <span style={{ display: 'inline-flex', color: 'var(--primary)' }} aria-hidden><SysIcon name="document" size={15} /></span>
                    <span>{t('new-contract')}</span>
                  </button>
                </div>
              )}
            </div>

            <button
              type="button"
              className="chrome-action-btn"
              title={followups > 0 ? tf('followups-n', { n: followups }) : t('followups-none')}
              onClick={() => navigate('/clients')}
            >
              <SysIcon name="bell" size={16} />
              {followups > 0 && <span className="chrome-badge">{followups}</span>}
            </button>

            <button
              type="button"
              className="chrome-action-btn"
              title={t('storage-status')}
              onClick={() => navigate('/database')}
            >
              <SysIcon name="archive" size={16} />
            </button>
          </div>

          <div className="chrome-end">
            <div className="chrome-prefs" ref={prefsRef}>
              <div className="chrome-pref">
                <button
                  type="button"
                  className={`chrome-action-btn${prefsOpen === 'theme' ? ' is-open' : ''}`}
                  title={t('theme')}
                  aria-expanded={prefsOpen === 'theme'}
                  onClick={() => setPrefsOpen((v) => (v === 'theme' ? null : 'theme'))}
                >
                  <Palette size={15} strokeWidth={1.9} />
                </button>
                {prefsOpen === 'theme' && (
                  <div className="quick-menu chrome-pref-menu" role="menu">
                    {THEMES.map((th) => (
                      <button
                        key={th.id}
                        type="button"
                        role="menuitemradio"
                        aria-checked={themeId === th.id}
                        className={themeId === th.id ? 'is-active' : undefined}
                        onClick={() => pickTheme(th.id)}
                      >
                        <span className="chrome-pref-swatches" aria-hidden>
                          {th.swatches.map((c) => (
                            <i key={c} style={{ background: c }} />
                          ))}
                        </span>
                        {t(`theme_${th.id}`)}
                      </button>
                    ))}
                  </div>
                )}
              </div>

              <div className="chrome-pref">
                <button
                  type="button"
                  className={`chrome-action-btn${prefsOpen === 'lang' ? ' is-open' : ''}`}
                  title={t('language')}
                  aria-expanded={prefsOpen === 'lang'}
                  onClick={() => setPrefsOpen((v) => (v === 'lang' ? null : 'lang'))}
                >
                  <LangIcon size={16} />
                </button>
                {prefsOpen === 'lang' && (
                  <div className="quick-menu chrome-pref-menu" role="menu">
                    {UI_LANGUAGES.map((l) => (
                      <button
                        key={l.value}
                        type="button"
                        role="menuitemradio"
                        aria-checked={uiLang === l.value}
                        className={uiLang === l.value ? 'is-active' : undefined}
                        onClick={() => void persistLocale({ language: l.value })}
                      >
                        {uiLang === 'en' ? l.labelEn : l.labelAr}
                      </button>
                    ))}
                  </div>
                )}
              </div>

              <div className="chrome-pref" title="تُحدد العملة من الدولة في الإعدادات">
                <span className="chrome-action-btn chrome-currency-btn"><Coins size={15} strokeWidth={1.9} /><span className="chrome-currency-code">{currencySymbol()}</span></span>
              </div>
            </div>

            <div className="titlebar-controls chrome-window">
              <button type="button" className="tb-btn" title={t('minimize')} onClick={() => win.minimize()}>
                <svg width="12" height="12" viewBox="0 0 12 12"><path d="M1 6h10" stroke="currentColor" strokeWidth="1.4" /></svg>
              </button>
              <button type="button" className="tb-btn" title={t('maximize')} onClick={() => win.toggleMaximize()}>
                <svg width="12" height="12" viewBox="0 0 12 12"><rect x="1.5" y="1.5" width="9" height="9" fill="none" stroke="currentColor" strokeWidth="1.4" rx="1" /></svg>
              </button>
              <button type="button" className="tb-btn tb-close" title={t('close')} onClick={() => win.close()}>
                <svg width="12" height="12" viewBox="0 0 12 12"><path d="M2 2l8 8M10 2l-8 8" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" /></svg>
              </button>
            </div>
          </div>
        </header>

        {(settingsError || shellDataError) && (
          <div className="error-alert" style={{ margin: '8px 16px 0' }}>
            {[settingsError, shellDataError].filter(Boolean).join(' — ')}
          </div>
        )}

        <main
          ref={pageScrollRef}
          className={`workspace-body page-content scroll-main${location.pathname.startsWith('/offer-design') ? ' is-ods-page' : ''}${location.pathname.startsWith('/maps') ? ' is-map-page' : ''}`}
          onScroll={syncChromeScroll}
        >
          {children}
          <PageCopyrightFooter />
        </main>
      </div>
      {appMenu && (
        <AppContextMenu
          x={appMenu.x}
          y={appMenu.y}
          items={appMenuItems}
          onClose={() => setAppMenu(null)}
        />
      )}
    </div>
  );
}
