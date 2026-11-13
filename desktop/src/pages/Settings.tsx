import OfficialIntegrations from '../components/OfficialIntegrations';
import { COUNTRY_PROFILES, countryProfile, getCountry, type CountryCode } from '../domain/countryProfile';
import React, { useEffect, useRef, useState } from 'react';
import { api } from '../api';
import { broadcastOfficeSettings } from '../title';
import { THEMES, applyTheme, currentTheme, ThemeId, isSoftUiEnabled, applySoftUi } from '../theme';
import { Settings as SettingsIcon, Building2, Package, Palette, Sparkles, Database, Languages } from 'lucide-react';
import SectionGuideBanner from '../components/SectionGuide';
import {
  GCC_CURRENCIES,
  UI_LANGUAGES,
  getDisplayCurrency,
  getUiLanguage,
  hydrateLocaleFromSettings,
  setLocalePrefs,
  type GccCurrencyCode,
  type UiLanguage
} from '../domain/locale';
import { useLocale } from '../hooks/useLocale';

import pkg from '../../package.json';
const APP_VERSION: string = pkg.version;

interface SettingsProps {
  auth: any;
}


export default function Settings({ auth: _auth }: SettingsProps) {
  const { t, tf, lang } = useLocale();
  const [theme, setTheme] = useState<ThemeId>(currentTheme());
  const [softUi, setSoftUi] = useState(isSoftUiEnabled());
  const [uiLanguage, setUiLanguage] = useState<UiLanguage>(getUiLanguage());
  const [country, setCountry] = useState<CountryCode>(getCountry());
  const profile = countryProfile(country);
  const displayCurrency = profile.currency;
  const [form, setForm] = useState({
    office_name: '',
    office_phone: '',
    office_whatsapp: '',
    office_email: '',
    office_address: '',
    broker_name: '',
    broker_phone: '',
    broker_whatsapp: '',
    fal_license_number: '',
    fal_license_type: '',
    broker_license_name: ''
  });
  const [message, setMessage] = useState('');
  const [saveError, setSaveError] = useState('');
  const [saving, setSaving] = useState(false);
  const saveInFlight = useRef(false);
  const demoInFlight = useRef(false);
  const [loadError, setLoadError] = useState('');
  const [demoStatus, setDemoStatus] = useState<any>(null);
  const [demoBusy, setDemoBusy] = useState(false);
  const [demoMsg, setDemoMsg] = useState('');
  const [demoErr, setDemoErr] = useState('');

  const themeLabelKey = (id: ThemeId) => `theme_${id}` as const;
  const themeDescKey = (id: ThemeId) => `theme_${id}_desc` as const;

  const handleTheme = (id: ThemeId) => {
    applyTheme(id);
    setTheme(id);
  };

  const handleSoftUi = (enabled: boolean) => {
    applySoftUi(enabled);
    setSoftUi(enabled);
  };

  useEffect(() => {
    loadSettings();
    refreshDemoStatus();
  }, []);

  const loadSettings = async () => {
    try {
      const data = await api('/api/settings');
      setForm({
        office_name: data.office_name || '',
        office_phone: data.office_phone || '',
        office_whatsapp: data.office_whatsapp || '',
        office_email: data.office_email || '',
        office_address: data.office_address || '',
        broker_name: data.broker_name || '',
        broker_phone: data.broker_phone || '',
        broker_whatsapp: data.broker_whatsapp || '',
        fal_license_number: data.fal_license_number || '',
        fal_license_type: data.fal_license_type || '',
        broker_license_name: data.broker_license_name || ''
      });
      hydrateLocaleFromSettings(data);
      setUiLanguage(getUiLanguage());
      setCountry(getCountry());
      setLoadError('');
    } catch {
      setLoadError(t('settings.load-failed'));
    }
  };

  const refreshDemoStatus = async () => {
    try {
      const s = await api('/api/demo/status');
      setDemoStatus(s);
      setDemoErr('');
    } catch {
      setDemoErr(t('settings.demo-status-fail'));
    }
  };

  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) => {
    setForm({ ...form, [e.target.name]: e.target.value });
  };

  const handleSubmit = async (e?: React.FormEvent) => {
    e?.preventDefault();
    if (saveInFlight.current) return;
    saveInFlight.current = true;
    setSaving(true);
    setMessage('');
    setSaveError('');
    try {
      await api('/api/settings', {
        method: 'PUT',
        body: {
          office_name: form.office_name,
          office_phone: form.office_phone,
          office_whatsapp: form.office_whatsapp,
          office_email: form.office_email,
          office_address: form.office_address,
          broker_name: form.broker_name,
          broker_phone: form.broker_phone,
          broker_whatsapp: form.broker_whatsapp,
          fal_license_number: form.fal_license_number,
          fal_license_type: form.fal_license_type,
          ui_language: uiLanguage,
          country,
          broker_license_name: form.broker_license_name
        }
      });
      setLocalePrefs({ language: uiLanguage, country });
      setMessage(t('settings.saved'));
      broadcastOfficeSettings({ country, broker_license_name: form.broker_license_name,
        office_name: form.office_name,
        fal_license_number: form.fal_license_number,
        fal_license_type: form.fal_license_type,
        office_phone: form.office_phone,
        office_whatsapp: form.office_whatsapp
      });
      setTimeout(() => setMessage(''), 3000);
    } catch {
      setSaveError(t('error'));
    } finally {
      saveInFlight.current = false;
      setSaving(false);
    }
  };

  const confirmTrainingDelete = async () => {
    if (demoInFlight.current) return;
    const clients = demoStatus?.demo_counts?.clients ?? 0;
    const properties = demoStatus?.demo_counts?.properties ?? 0;
    const market = demoStatus?.demo_counts?.market_reference_data ?? 0;
    const ok = window.confirm(
      tf('settings.demo-delete-confirm', { clients, properties, market })
    );
    if (!ok) return;
    demoInFlight.current = true;
    setDemoBusy(true);
    setDemoMsg('');
    setDemoErr('');
    try {
      const res = await api('/api/demo/delete', { method: 'POST' });
      setDemoMsg(res.message || t('settings.demo-deleted'));
      setDemoStatus(res.after);
    } catch {
      setDemoErr(t('settings.demo-delete-fail'));
    } finally {
      demoInFlight.current = false;
      setDemoBusy(false);
    }
  };

  const confirmTrainingLoad = async () => {
    if (demoInFlight.current) return;
    const ok = window.confirm(
      demoStatus?.batch_exists
        ? t('settings.demo-reload-confirm')
        : t('settings.demo-load-confirm')
    );
    if (!ok) return;
    demoInFlight.current = true;
    setDemoBusy(true);
    setDemoMsg('');
    setDemoErr('');
    try {
      const res = await api('/api/demo/reseed', { method: 'POST' });
      setDemoMsg(res.message || t('settings.demo-loaded-msg'));
      setDemoStatus(res.status);
    } catch {
      setDemoErr(t('settings.demo-load-fail'));
    } finally {
      demoInFlight.current = false;
      setDemoBusy(false);
    }
  };


  return (
    <div>
      {loadError && <div className="error-alert" role="alert" style={{ marginBottom: 16 }}>{loadError} <button type="button" className="btn btn-outline" onClick={loadSettings}>{t('retry')}</button></div>}

      <div className="content-card">
        <h3 style={{ marginBottom: 16, color: 'var(--primary)', display: 'flex', alignItems: 'center', gap: 8 }}>
          <Palette size={16} strokeWidth={1.8} /> {t('settings.appearance')}
        </h3>
        <div className="theme-grid">
          {THEMES.map((themeDef) => (
            <button
              key={themeDef.id}
              type="button"
              className={`theme-card ${theme === themeDef.id ? 'active' : ''}`}
              onClick={() => handleTheme(themeDef.id)}
              aria-pressed={theme === themeDef.id}
            >
              <div className="theme-swatch">
                {themeDef.swatches.map((c, i) => <span key={i} style={{ background: c }} />)}
              </div>
              <div className="theme-card-body">
                <span className="theme-card-name">{t(themeLabelKey(themeDef.id))}</span>
                <span className="theme-card-desc">{t(themeDescKey(themeDef.id))}</span>
              </div>
              {theme === themeDef.id && (
                <span style={{ color: 'var(--gold)', fontSize: 14, fontWeight: 800 }} aria-hidden>✓</span>
              )}
            </button>
          ))}
        </div>
        <p style={{ marginTop: 14, fontSize: 12, color: 'var(--gray)' }}>
          {t('settings.theme-persist')}
        </p>

        <div className="soft-ui-toggle-card">
          <div className="soft-ui-toggle-text">
            <div className="soft-ui-toggle-title">
              <Sparkles size={15} strokeWidth={1.8} /> {t('settings.soft-ui')}
            </div>
            <div className="soft-ui-toggle-desc">
              {t('settings.soft-ui-desc')}
            </div>
          </div>
          <button
            type="button"
            className={`soft-ui-switch ${softUi ? 'on' : ''}`}
            role="switch"
            aria-checked={softUi}
            aria-label={t('settings.soft-ui')}
            onClick={() => handleSoftUi(!softUi)}
          >
            <span className="soft-ui-switch-knob" />
            <span className="soft-ui-switch-label">{softUi ? t('settings.enabled') : t('settings.disabled')}</span>
          </button>
        </div>
      </div>

      <div className="card-header">
        <div className="card-title"><SettingsIcon size={17} strokeWidth={1.8} style={{ verticalAlign: '-3px' }} /> {t('settings')} <SectionGuideBanner id="settings" /></div>
      </div>

      {message && <div className="success-alert" style={{ marginBottom: 16 }}>{message}</div>}

      <div className="content-card">
        <h3 style={{ marginBottom: 16, color: 'var(--primary)' }}><Building2 size={16} strokeWidth={1.8} style={{ verticalAlign: '-2px' }} /> {t('settings.office')}</h3>
        <form onSubmit={handleSubmit}>
          <div className="form-grid">
            <div className="form-group"><label>{t('settings.office-name')}</label><input className="form-input" name="office_name" value={form.office_name} onChange={handleChange} /></div>
            <div className="form-group"><label>{t('phone')}</label><input className="form-input" name="office_phone" value={form.office_phone} onChange={handleChange} /></div>
            <div className="form-group"><label>{t('settings.whatsapp')}</label><input className="form-input" name="office_whatsapp" value={form.office_whatsapp} onChange={handleChange} /></div>
            <div className="form-group"><label>{t('email')}</label><input className="form-input" name="office_email" value={form.office_email} onChange={handleChange} /></div>
            <div className="form-group full-row"><label>{t('settings.address')}</label><input className="form-input" name="office_address" value={form.office_address} onChange={handleChange} /></div>
            <div className="form-group"><label>{t('settings.fal-number')}</label><input className="form-input" name="fal_license_number" value={form.fal_license_number} onChange={handleChange} /></div>
            <div className="form-group">
              <label>{t('settings.fal-type')}</label>
              <select className="form-input" name="fal_license_type" value={form.fal_license_type} onChange={handleChange}>
                <option value="">{t('settings.choose-option')}</option>
                <option value="manual">إدخال يدوي</option>
                {profile.licenses.map((license) => (
                  <option key={license.id} value={license.id}>{license.name}</option>
                ))}
              </select>
            </div>
            <div className="form-group">{country === "AE" && <p className="muted">لم يثبت مسمى رسمي منشور للفجيرة وأم القيوين؛ استخدم الإدخال اليدوي. يتحدد رابط الخدمات من نوع الرخصة المعتمد، ولا تُستنتج الإمارة من النص الحر.</p>}<label htmlFor="broker-license-name">اسم الترخيص / السجل التجاري — إدخال يدوي</label><input id="broker-license-name" className="form-input" name="broker_license_name" value={form.broker_license_name} onChange={handleChange} /></div>
            <div className="form-group"><label>{t('settings.broker-name')}</label><input className="form-input" name="broker_name" value={form.broker_name} onChange={handleChange} /></div>
            <div className="form-group"><label>{t('settings.broker-phone')}</label><input className="form-input" name="broker_phone" value={form.broker_phone} onChange={handleChange} /></div>
            <div className="form-group"><label>{t('settings.broker-whatsapp')}</label><input className="form-input" name="broker_whatsapp" value={form.broker_whatsapp} onChange={handleChange} /></div>
          </div>
          {saveError && <div className="error-alert" role="alert">{saveError} <button type="button" className="btn btn-outline" onClick={() => void handleSubmit()}>{t('retry')}</button></div>}
          <button className="btn btn-primary" style={{ marginTop: 16 }} disabled={saving}>{saving ? t('saving') : t('settings.save-office')}</button>
        </form>
      </div>

      <OfficialIntegrations />

      <div className="content-card">
        <h3 style={{ marginBottom: 16, color: 'var(--primary)' }}><Package size={16} strokeWidth={1.8} style={{ verticalAlign: '-2px' }} /> معلومات النسخة</h3>
        <div className="license-details">
          <div><strong>المنتج</strong><span>OBR – مكتب الوسيط العقاري</span></div>
          <div><strong>الإصدار</strong><span dir="ltr">{APP_VERSION}</span></div>
          <div><strong>حالة النسخة</strong><span className="badge badge-available">كاملة — دون تفعيل</span></div>
        </div>
        <small style={{ display: 'block', marginTop: 12, lineHeight: 1.7, color: 'var(--muted-foreground)' }}>
          لا يوجد تفعيل عبر حساب OBR لهذه النسخة. يعمل البرنامج مباشرة، وبياناتك محفوظة محليًا على جهازك.
        </small>
      </div>

      <div className="content-card">
        <h3 style={{ marginBottom: 12, color: 'var(--primary)', display: 'flex', alignItems: 'center', gap: 8 }}>
          <Languages size={16} strokeWidth={1.8} /> {t('language')} / {t('currency')}
        </h3>
        <p style={{ fontSize: 13, lineHeight: 1.7, color: 'var(--muted-foreground)', marginBottom: 14 }}>
          {t('locale-hint')}
        </p>
        <p className="text-secondary" style={{ fontSize: 12, marginBottom: 12 }}>{t('templates-in-help')}</p>
        <div className="form-grid">
          <div className="form-group">
            <label>{t('language')}</label>
            <select
              className="form-input"
              value={uiLanguage}
              onChange={(e) => setUiLanguage(e.target.value as UiLanguage)}
            >
              {UI_LANGUAGES.map((l) => (
                <option key={l.value} value={l.value}>{lang === 'en' ? l.labelEn : l.labelAr}</option>
              ))}
            </select>
          </div>
          <div className="form-group">
            <label htmlFor="settings-country">الدولة</label>
            <select id="settings-country" className="form-input" value={country} onChange={(e) => {
              setCountry(e.target.value as CountryCode);
              setForm(f => ({ ...f, fal_license_type: '', fal_license_number: '', broker_license_name: '' }));
            }}>
              {COUNTRY_PROFILES.map(p => <option key={p.code} value={p.code}>{lang === 'en' ? p.nameEn : p.nameAr} — {p.currency}</option>)}
            </select>
            <small>عملة السجلات الجديدة: {displayCurrency}. السجلات السابقة تحتفظ بعملتها.</small>
          </div>
        </div>
        <button
          type="button"
          className="btn btn-primary"
          style={{ marginTop: 14 }}
          onClick={async () => {
            try {
              await api('/api/settings', {
                method: 'PUT',
                body: {
                  ...form,
                  ui_language: uiLanguage,
                  country,
          broker_license_name: form.broker_license_name
                }
              });
              setLocalePrefs({ language: uiLanguage, country });
              broadcastOfficeSettings({ ...form, country });
              setMessage(t('locale-saved'));
              setTimeout(() => setMessage(''), 3000);
            } catch {
              setSaveError(t('error'));
            }
          }}
        >
          {t('save-locale')}
        </button>
      </div>

      <div className="content-card">
        <h3 style={{ marginBottom: 12, color: 'var(--primary)', display: 'flex', alignItems: 'center', gap: 8 }}>
          <Database size={16} strokeWidth={1.8} /> {t('settings.demo')}
        </h3>
        <p style={{ fontSize: 13, lineHeight: 1.7, color: 'var(--muted-foreground)', marginBottom: 12 }}>
          {t('settings.demo-intro')}
        </p>
        {demoStatus && (
          <div style={{ fontSize: 14, lineHeight: 1.8, marginBottom: 14 }}>
            <div>
              {demoStatus.batch_exists
                ? t('settings.demo-loaded')
                : t('settings.demo-none')}
            </div>
            {demoStatus.batch_exists && (
              <>
                <div>{t('settings.demo-clients')}: <strong className="num">{demoStatus.demo_counts?.clients ?? 0}</strong></div>
                <div>{t('settings.demo-listings')}: <strong className="num">{demoStatus.demo_counts?.properties ?? 0}</strong></div>
                <div>{t('settings.demo-market')}: <strong className="num">{demoStatus.demo_counts?.market_reference_data ?? 0}</strong></div>
              </>
            )}
            <div className="text-secondary" style={{ marginTop: 6 }}>
              {tf('settings.demo-real', {
                clients: demoStatus.non_demo_clients ?? 0,
                listings: demoStatus.non_demo_properties ?? 0
              })}
            </div>
          </div>
        )}
        {demoMsg && <div className="success-alert" style={{ marginBottom: 12 }}>{demoMsg}</div>}
        {demoErr && <div className="error-alert" style={{ marginBottom: 12 }}>{demoErr}</div>}
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 10 }}>
          <button type="button" className="btn btn-outline" disabled={demoBusy} onClick={refreshDemoStatus}>
            {t('settings.demo-refresh')}
          </button>
          <button type="button" className="btn btn-primary" disabled={demoBusy} onClick={confirmTrainingLoad}>
            {demoStatus?.batch_exists ? t('settings.demo-reload') : t('settings.demo-load')}
          </button>
          <button
            type="button"
            className="btn btn-outline"
            disabled={demoBusy || !demoStatus?.batch_exists}
            onClick={confirmTrainingDelete}
          >
            {t('settings.demo-delete-all')}
          </button>
        </div>
        <p style={{ marginTop: 10, fontSize: 12, color: 'var(--muted-foreground)' }}>
          {t('settings.demo-delete-hint')}
        </p>
      </div>
    </div>
  );
}
