import React, { useEffect, useState } from 'react';
import { api } from '../api';
import { COUNTRY_PROFILES, countryProfile, getCountry } from '../domain/countryProfile';
import { useLocale } from '../hooks/useLocale';

type Integration = ReturnType<typeof blank>;
const blank = () => ({ id: '', country: getCountry() as string, jurisdiction: '', authorityName: '', integrationName: '', mode: 'official_url', serviceUrl: '', baseUrl: '', endpoint: '', httpMethod: 'GET', authType: 'none', apiKeyHeader: '', clientId: '', tokenUrl: '', scope: '', enabled: false, status: 'untested', lastTestedAt: null as string | null, hasSecrets: false });
const blankSecrets = () => ({ apiKey: '', token: '', username: '', password: '', clientSecret: '', customHeaders: '' });
export const integrationRequest = (body: Record<string, unknown>) => api('/api/official-integrations', { method: 'POST', body });

export default function OfficialIntegrations() {
  const { t, lang } = useLocale();
  const [items, setItems] = useState<Integration[]>([]);
  const [form, setForm] = useState<Integration | null>(null);
  const [secrets, setSecrets] = useState(blankSecrets);
  const [replaceSecrets, setReplaceSecrets] = useState(false);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState('');
  const [error, setError] = useState('');
  const refresh = async () => setItems((await integrationRequest({ action: 'list' })).items);
  useEffect(() => { refresh().catch(() => setError(t('integration.storage'))); }, []);
  const run = async (work: () => Promise<void>) => {
    setBusy(true); setError(''); setMessage('');
    try { await work(); } catch (e) { const key = e instanceof Error ? e.message : 'integration.failed'; setError(t(key.startsWith('integration.') ? key : 'integration.failed')); }
    finally { setBusy(false); }
  };
  const edit = (item: Integration) => { setForm({ ...item }); setSecrets(blankSecrets()); setReplaceSecrets(!item.hasSecrets); setError(''); setMessage(''); };
  const update = (key: keyof Integration, value: string | boolean) => setForm(f => f ? { ...f, [key]: value } : null);
  const field = (key: keyof Integration, type = 'text') => form && <label className="form-group" key={key}>{t(`integration.${key}`)}<input className="form-input" type={type} value={String(form[key] ?? '')} onChange={e => update(key, e.target.value)} /></label>;
  const secretField = (key: keyof ReturnType<typeof blankSecrets>) => <label className="form-group" key={key}>{t(`integration.${key}`)}<input className="form-input" type="password" autoComplete="new-password" value={secrets[key]} onChange={e => setSecrets(s => ({ ...s, [key]: e.target.value }))} /></label>;
  const select = (key: keyof Integration, values: string[], translate = false) => form && <label className="form-group">{t(`integration.${key}`)}<select className="form-input" value={String(form[key])} onChange={e => { update(key,e.target.value); if(key === 'authType') { setReplaceSecrets(true); setSecrets(blankSecrets()); } }}>{values.map(v => <option key={v} value={v}>{translate ? t(`integration.${v}`) : v}</option>)}</select></label>;
  const jurisdictions = [...new Set(countryProfile('AE').licenses.map(l => 'jurisdiction' in l ? String(l.jurisdiction) : '').filter(Boolean))];
  const save = async () => {
    if (!form) return;
    let secretPayload: Record<string, unknown> | undefined;
    if (replaceSecrets && form.mode === 'api') {
      let customHeaders = {};
      if (secrets.customHeaders.trim()) {
        try { customHeaders = JSON.parse(secrets.customHeaders); } catch { throw new Error('integration.headers_json'); }
        if (!customHeaders || Array.isArray(customHeaders) || typeof customHeaders !== 'object' || Object.values(customHeaders).some(v => typeof v !== 'string')) throw new Error('integration.headers_json');
      }
      secretPayload = { ...secrets, customHeaders };
    }
    const saved = await integrationRequest({ action: 'save', integration: form, ...(secretPayload ? { secrets: secretPayload } : {}) });
    setSecrets(blankSecrets()); setReplaceSecrets(false); setForm(saved); await refresh(); setMessage(t('integration.saved'));
  };
  return <section className="content-card" aria-label={t('integration.title')}>
    <h3>{t('integration.title')}</h3><p className="muted">{t('integration.description')}</p>
    {error && <p className="error-alert" role="alert">{error}</p>}{message && <p role="status">{message}</p>}
    <button type="button" className="btn btn-outline" disabled={busy} onClick={() => edit(blank())}>{t('integration.add')}</button>
    <table><thead><tr><th>{t('integration.integrationName')}</th><th>{t('integration.country')}</th><th>{t('integration.status')}</th><th>{t('integration.actions')}</th></tr></thead><tbody>
      {items.map(item => <tr key={item.id}><td>{item.integrationName}</td><td>{item.country}{item.jurisdiction ? ` / ${item.jurisdiction}` : ''}</td><td>{t(`integration.${item.status}`)}<br />{item.lastTestedAt || ''}</td><td>
        <button type="button" className="btn btn-sm btn-outline" disabled={busy} onClick={() => edit(item)}>{t('integration.edit')}</button>
        <button type="button" className="btn btn-sm btn-outline" disabled={busy} onClick={() => run(async () => { await integrationRequest({ action: 'save', integration: { ...item, enabled: !item.enabled } }); await refresh(); })}>{t(item.enabled ? 'integration.disable' : 'integration.enable')}</button>
        <button type="button" className="btn btn-sm btn-outline" disabled={busy} onClick={() => run(async () => { const result = await integrationRequest({ action: 'test', id: item.id }); setMessage(t(result.message)); await refresh(); if (form?.id === item.id) setForm(result.integration); })}>{t('integration.test')}</button>
        <button type="button" className="btn btn-sm btn-outline" disabled={busy} onClick={() => { if (confirm(t('integration.deleteConfirm'))) run(async () => { await integrationRequest({ action: 'delete', id: item.id }); if (form?.id === item.id) setForm(null); await refresh(); }); }}>{t('integration.delete')}</button>
      </td></tr>)}
    </tbody></table>
    {form && <form onSubmit={e => { e.preventDefault(); void run(save); }}><div className="form-grid">
      <label className="form-group">{t('integration.country')}<select className="form-input" value={form.country} onChange={e => setForm({ ...form, country: e.target.value, jurisdiction: '' })}>{COUNTRY_PROFILES.map(p => <option key={p.code} value={p.code}>{lang === 'en' ? p.nameEn : p.nameAr}</option>)}</select></label>
      {form.country === 'AE' && <label className="form-group">{t('integration.jurisdiction')}<select className="form-input" value={form.jurisdiction} onChange={e => update('jurisdiction', e.target.value)}><option value="">{t('integration.countryWide')}</option>{jurisdictions.map(j => <option key={j} value={j}>{j}</option>)}</select></label>}
      {field('authorityName')}{field('integrationName')}{select('mode',['official_url','api'],true)}
      <label><input type="checkbox" checked={form.enabled} onChange={e => update('enabled', e.target.checked)} /> {t('integration.enabled')}</label>
      {form.mode === 'official_url' ? field('serviceUrl','url') : <>
        {field('baseUrl','url')}{field('endpoint')}{select('httpMethod',['GET','HEAD','POST','PUT','PATCH','DELETE','OPTIONS'])}{select('authType',['none','api_key','bearer','basic','oauth2'],true)}
        {form.authType === 'api_key' && field('apiKeyHeader')}{form.authType === 'oauth2' && <>{field('clientId')}{field('tokenUrl','url')}{field('scope')}</>}
        {form.hasSecrets && <label><span>•••••••• — {t('integration.protected')}</span><input type="checkbox" checked={replaceSecrets} onChange={e => { setReplaceSecrets(e.target.checked); setSecrets(blankSecrets()); }} />{t('integration.replaceSecrets')}</label>}
        {replaceSecrets && <>
          {form.authType === 'api_key' && secretField('apiKey')}{form.authType === 'bearer' && secretField('token')}
          {form.authType === 'basic' && <>{secretField('username')}{secretField('password')}</>}{form.authType === 'oauth2' && secretField('clientSecret')}
          <label className="form-group">{t('integration.customHeaders')}<textarea className="form-input" dir="ltr" autoComplete="off" value={secrets.customHeaders} onChange={e => setSecrets(s => ({ ...s, customHeaders: e.target.value }))} placeholder={'{"X-Custom-Header":"value"}'} /></label>
        </>}
        <p className="muted">{t('integration.probeHint')}</p><p className="muted">{t('integration.mapping_required')}</p>
      </>}
    </div><button className="btn btn-primary" disabled={busy}>{t('integration.save')}</button><button type="button" className="btn btn-outline" disabled={busy} onClick={() => { setForm(null); setSecrets(blankSecrets()); }}>{t('integration.close')}</button></form>}
  </section>;
}
