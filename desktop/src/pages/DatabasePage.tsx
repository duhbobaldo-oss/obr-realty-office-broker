import React, { useEffect, useRef, useState } from 'react';
import {
  HardDrive, Database as DbIcon, Download, Archive, Upload, RefreshCw, RotateCcw, Trash2
} from 'lucide-react';
import { save, open } from '@tauri-apps/plugin-dialog';
import { writeFile, readFile } from '@tauri-apps/plugin-fs';
import { api } from '../api';
import SectionGuideBanner from '../components/SectionGuide';
import { useLocale } from '../hooks/useLocale';
import { t as tLocale, tf as tfLocale } from '../domain/locale';

type BackupItem = {
  folder_name: string;
  path: string;
  sequence: number;
  stamp: string;
  created_at?: string;
  schema_version?: number;
  db_bytes?: number;
  restorable?: boolean;
};

function stampNow(): string {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}${p(d.getMonth() + 1)}${p(d.getDate())}-${p(d.getHours())}${p(d.getMinutes())}${p(d.getSeconds())}`;
}

function formatBytes(n?: number): string {
  if (n == null || !Number.isFinite(n) || n <= 0) return '—';
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(2)} MB`;
}

const AR_MONTHS = ['يناير', 'فبراير', 'مارس', 'أبريل', 'مايو', 'يونيو', 'يوليو', 'أغسطس', 'سبتمبر', 'أكتوبر', 'نوفمبر', 'ديسمبر'];

/** تحويل التاريخ المخزن إلى صيغة عربية مقروءة للعرض فقط: 20 سبتمبر 2026 - 5:43 م */
function formatDateTimeAr(value?: string): string {
  if (!value) return '—';
  const d = new Date(value);
  if (isNaN(d.getTime())) return value;
  const day = d.getDate();
  const month = AR_MONTHS[d.getMonth()];
  const year = d.getFullYear();
  let hours = d.getHours();
  const ampm = hours >= 12 ? 'م' : 'ص';
  hours = hours % 12 || 12;
  const minutes = String(d.getMinutes()).padStart(2, '0');
  return `${day} ${month} ${year} - ${hours}:${minutes} ${ampm}`;
}

export default function DatabasePage() {
  const { t, tf } = useLocale();
  const [exporting, setExporting] = useState(false);
  const [importing, setImporting] = useState(false);
  const [backingUp, setBackingUp] = useState(false);
  const [restoring, setRestoring] = useState(false);
  const [loadingBackups, setLoadingBackups] = useState(false);
  const [backupListError, setBackupListError] = useState(false);
  const [message, setMessage] = useState('');
  const [error, setError] = useState('');
  const [info, setInfo] = useState<any>(null);
  const [backups, setBackups] = useState<BackupItem[]>([]);
  const [backupPage, setBackupPage] = useState(1);
  const [backupPageSize, setBackupPageSize] = useState(10);
  const [deleting, setDeleting] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const restoreInFlight = useRef(false);
  const deleteInFlight = useRef(false);

  const restoreUiAvailable = info?.restore_ui_available === true;

  const loadInfo = async () => {
    try {
      const data = await api('/api/system/info');
      setInfo(data);
      setError('');
    } catch (err: any) {
      setError(err.message || tLocale('db.info-fail'));
    }
  };

  const loadBackups = async () => {
    setLoadingBackups(true);
    try {
      const data = await api('/api/backup/list');
      const items = Array.isArray(data?.items) ? data.items : [];
      setBackups(items);
      setBackupListError(false);
      setError((current) => current === tLocale('db.backup-list-fail') ? '' : current);
    } catch (err: any) {
      setBackups([]);
      setBackupListError(true);
      setError(tLocale('db.backup-list-fail'));
    } finally {
      setLoadingBackups(false);
    }
  };

  useEffect(() => {
    void (async () => {
      await loadInfo();
    })();
  }, []);

  useEffect(() => {
    if (restoreUiAvailable) {
      void loadBackups();
      setBackupPage(1);
    }
  }, [restoreUiAvailable]);

  const deleteBackup = async (item: BackupItem) => {
    if (deleteInFlight.current || deleting) return;
    if (!item?.path) return;
    const ok = window.confirm(tLocale('db.backup-delete-confirm'));
    if (!ok) return;
    deleteInFlight.current = true;
    setDeleting(true);
    setMessage('');
    setError('');
    try {
      await api('/api/backup/delete', { method: 'POST', body: { package_path: item.path } });
      setMessage(tLocale('db.backup-delete-done'));
      // تحديث القائمة بعد الحذف — فشل التحديث لا يعني فشل الحذف
      try {
        await loadBackups();
        setBackupPage(1);
      } catch {
        setError(tLocale('db.backup-refresh-fail'));
      }
    } catch (err: any) {
      setError(err?.message || tLocale('db.backup-delete-fail'));
    } finally {
      deleteInFlight.current = false;
      setDeleting(false);
    }
  };

  const exportJson = async (section: 'all' | 'properties' | 'clients' | 'market') => {
    setExporting(true);
    setMessage('');
    setError('');
    try {
      const data = section === 'all'
        ? await api('/api/export/all')
        : await api(`/api/export/${section}`);

      const defaultName = section === 'all'
        ? `obr-export-all-${stampNow()}.json`
        : `obr-export-${section}-${stampNow()}.json`;

      const selected = await save({
        defaultPath: defaultName,
        filters: [{ name: 'JSON', extensions: ['json'] }]
      });
      if (!selected) {
        setMessage(t('db.export-cancel'));
        return;
      }
      const path = Array.isArray(selected) ? selected[0] : selected;
      const finalPath = /\.json$/i.test(path) ? path : `${path}.json`;
      const bytes = new TextEncoder().encode(JSON.stringify(data, null, 2));
      await writeFile(finalPath, bytes);
      setMessage(tf('db.export-done', { file: finalPath }));
    } catch (err: any) {
      setError(err.message || tLocale('db.export-fail'));
    } finally {
      setExporting(false);
    }
  };

  const importFromBytes = async (bytes: Uint8Array) => {
    setImporting(true);
    setMessage('');
    setError('');
    try {
      const text = new TextDecoder().decode(bytes);
      const payload = JSON.parse(text);
      if (!payload || (typeof payload !== 'object')) {
        throw new Error(t('db.import-invalid'));
      }
      let body = payload;
      if (Array.isArray(payload)) {
        const sample = payload[0] || {};
        if (sample.sale_price != null || sample.price_per_meter_sale != null || sample.rent_value != null || sample.entry_date != null) {
          body = { data: { market_reference_data: payload } };
        } else if (sample.purpose != null || sample.property_type != null || (sample.name != null && sample.area != null)) {
          body = { data: { properties: payload } };
        } else {
          body = { data: { clients: payload } };
        }
      }
      const result = await api('/api/import/json', {
        method: 'POST',
        body: { payload: body }
      });
      const parts = Object.entries(result?.imported || {})
        .map(([k, v]) => `${k}: ${v}`)
        .join(' · ');
      setMessage(parts ? `${t('db.import-done')} — ${parts}` : t('db.import-done'));
      if (Array.isArray(result?.errors) && result.errors.length > 0) {
        setError(result.errors.slice(0, 5).join('\n'));
      }
    } catch (err: any) {
      setError(err.message || tLocale('db.import-fail'));
    } finally {
      setImporting(false);
    }
  };

  const pickImportFile = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: 'JSON', extensions: ['json'] }]
      });
      if (!selected) return;
      const path = Array.isArray(selected) ? selected[0] : selected;
      const bytes = await readFile(path);
      await importFromBytes(bytes instanceof Uint8Array ? bytes : Uint8Array.from(bytes as any));
    } catch (err: any) {
      if (String(err?.message || err).includes('not allowed') || String(err?.message || '').includes('plugin')) {
        fileInputRef.current?.click();
        return;
      }
      setError(err.message || tLocale('db.import-fail'));
    }
  };

  const onHtmlFile = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    e.target.value = '';
    if (!file) return;
    try {
      const buf = await file.arrayBuffer();
      await importFromBytes(new Uint8Array(buf));
    } catch (err: any) {
      setError(err.message || tLocale('db.import-fail'));
    }
  };

  const createBackup = async () => {
    setBackingUp(true);
    setMessage('');
    setError('');
    try {
      const data = await api('/api/backup/create', { method: 'POST' });
      setMessage(data.message || tfLocale('db.backup-save-hint', { msg: tLocale('db.backup-done') }));
      await loadInfo();
      if (restoreUiAvailable) await loadBackups();
    } catch (err: any) {
      setError(err.message || tLocale('db.backup-fail'));
    } finally {
      setBackingUp(false);
    }
  };

  const restoreBackup = async (item: BackupItem) => {
    if (restoreInFlight.current || restoring) {
      setError(tLocale('db.backup-restore-busy'));
      return;
    }
    if (!item?.path || item.restorable === false) {
      setError(tLocale('db.backup-unavailable'));
      return;
    }
    const label = item.folder_name || item.stamp || item.path;
    const ok = window.confirm(tfLocale('db.backup-restore-confirm', { name: label }));
    if (!ok) {
      setMessage(tLocale('db.backup-restore-cancel'));
      setError('');
      return;
    }

    restoreInFlight.current = true;
    setRestoring(true);
    setMessage('');
    setError('');
    try {
      const data = await api('/api/backup/restore', {
        method: 'POST',
        body: { package_path: item.path }
      });
      setMessage(data?.message || tLocale('db.backup-restore-done'));
      // الاستعادة تُبطل الجلسات المخزّنة في القاعدة المستعادة عمدًا (أمنيًا)؛
      // لذلك نُنشئ جلسة محلية جديدة لنفس المالك قبل أي استدعاء مصادق عليه.
      try {
        const fresh = await api('/api/auth/ensure-local', { method: 'POST', auth: false });
        localStorage.setItem('mawasib_auth', JSON.stringify(fresh));
      } catch (authErr: any) {
        setError(authErr?.message || tLocale('db.backup-refresh-fail'));
      }
      // نجاح الاستعادة مستقل عن تحديث القائمة — فشل التحديث لا يجعل الاستعادة فاشلة
      try {
        await loadInfo();
      } catch {
        setError(tLocale('db.info-fail'));
      }
      try {
        await loadBackups();
        setBackupPage(1);
      } catch {
        setError(tLocale('db.backup-refresh-fail'));
      }
    } catch (err: any) {
      setError(err?.message || tLocale('db.backup-restore-fail'));
    } finally {
      restoreInFlight.current = false;
      setRestoring(false);
    }
  };

  const busy = exporting || importing || backingUp || restoring;

  return (
    <div>
      <div className="card-header">
        <div className="card-title"><DbIcon size={17} strokeWidth={1.8} style={{ verticalAlign: '-3px' }} /> {t('db.title')} <SectionGuideBanner id="database" /></div>
      </div>

      {message && <div className="success-alert" style={{ marginBottom: 16 }}>{message}</div>}
      {error && <div className="error-alert" style={{ marginBottom: 16, whiteSpace: 'pre-wrap' }}>{error}</div>}

      <div className="content-card obr-panel">
        <h3 style={{ marginBottom: 0 }}><HardDrive size={16} strokeWidth={1.8} style={{ verticalAlign: '-2px' }} /> {t('db.storage')}</h3>
        {info ? (
          <div className="obr-panel-body" style={{ fontSize: 13, lineHeight: 1.9, color: 'var(--foreground)' }}>
            <div>{t('db.schema')}: <strong>{info.schema_version}</strong></div>
            <div>{t('db.integrity')}: <strong>{info.integrity_ok ? t('db.ok') : t('db.bad')}</strong></div>
            <div className="text-secondary">{t('db.local-only')}</div>
          </div>
        ) : (
          <div className="obr-panel-body" style={{ color: 'var(--muted-foreground)' }}>{t('db.reading')}</div>
        )}
      </div>

      <div className="content-card obr-panel">
        <h3 style={{ marginBottom: 0 }}><Download size={16} strokeWidth={1.8} style={{ verticalAlign: '-2px' }} /> {t('db.export-title')}</h3>
        <div className="obr-panel-body">
          <p style={{ margin: '0 0 16px', color: 'var(--muted-foreground)', lineHeight: 1.6 }}>
            {t('db.export-hint')}
          </p>
          <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap' }}>
            <button className="btn btn-primary" onClick={() => exportJson('all')} disabled={busy}>
              {exporting ? t('db.exporting') : <><Download size={15} strokeWidth={1.8} /> {t('db.export-all')}</>}
            </button>
            <button className="btn btn-outline" disabled={busy} onClick={() => exportJson('properties')}>
              {t('db.export-properties')}
            </button>
            <button className="btn btn-outline" disabled={busy} onClick={() => exportJson('clients')}>
              {t('db.export-clients')}
            </button>
            <button className="btn btn-outline" disabled={busy} onClick={() => exportJson('market')}>
              {t('db.export-market')}
            </button>
          </div>
        </div>
      </div>

      <div className="content-card obr-panel">
        <h3 style={{ marginBottom: 0 }}><Upload size={16} strokeWidth={1.8} style={{ verticalAlign: '-2px' }} /> {t('db.import-title')}</h3>
        <div className="obr-panel-body">
          <p style={{ margin: '0 0 16px', color: 'var(--muted-foreground)', lineHeight: 1.6 }}>
            {t('db.import-hint')}
          </p>
          <input ref={fileInputRef} type="file" accept=".json,application/json" hidden onChange={onHtmlFile} />
          <button className="btn btn-primary" disabled={busy} onClick={() => void pickImportFile()}>
            {importing ? t('db.importing') : <><Upload size={15} strokeWidth={1.8} /> {t('db.import-pick')}</>}
          </button>
        </div>
      </div>

      <div className="content-card obr-panel">
        <h3 style={{ marginBottom: 0 }}><Archive size={16} strokeWidth={1.8} style={{ verticalAlign: '-2px' }} /> {t('db.backup')}</h3>
        <div className="obr-panel-body">
          <p style={{ margin: '0 0 14px', color: 'var(--muted-foreground)', lineHeight: 1.6 }}>
            {t('db.backup-hint')}
          </p>
          <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap', marginBottom: restoreUiAvailable ? 18 : 0 }}>
            <button className="btn btn-primary" disabled={busy} onClick={() => void createBackup()}>
              {backingUp ? t('db.backing-up') : <><Archive size={15} strokeWidth={1.8} /> {t('db.backup-create')}</>}
            </button>
            {restoreUiAvailable ? (
              <button
                className="btn btn-outline"
                type="button"
                disabled={busy || loadingBackups}
                onClick={() => void loadBackups()}
              >
                <RefreshCw size={15} strokeWidth={1.8} /> {t('db.backup-refresh')}
              </button>
            ) : null}
          </div>

          {restoreUiAvailable ? (
            <>
              <h4 style={{ margin: '0 0 10px', fontSize: 14 }}>{t('db.backup-list')}</h4>
              {loadingBackups && backups.length === 0 ? (
                <p className="text-secondary" style={{ margin: 0 }}>{t('db.reading')}</p>
              ) : backupListError ? (
                <button type="button" className="btn btn-outline" onClick={() => void loadBackups()}>{t('retry')}</button>
              ) : backups.length === 0 ? (
                <p className="text-secondary" style={{ margin: 0 }}>{t('db.backup-empty')}</p>
              ) : (
                <div className="table-container">
                  <table className="data-table">
                    <thead>
                      <tr>
                        <th scope="col">{t('db.backup-col-num')}</th>
                        <th scope="col">{t('db.backup-col-date')}</th>
                        <th scope="col">{t('db.backup-col-name')}</th>
                        <th scope="col">{t('db.backup-col-size')}</th>
                        <th scope="col">{t('db.backup-col-schema')}</th>
                        <th scope="col">{t('db.backup-col-actions')}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {backups
                        .slice((backupPage - 1) * backupPageSize, backupPage * backupPageSize)
                        .map((item) => {
                        const canRestore = item.restorable !== false && Boolean(item.path);
                        return (
                          <tr key={item.path || item.folder_name}>
                            <td className="num">{item.sequence || '—'}</td>
                            <td>{formatDateTimeAr(item.created_at)}</td>
                            <td>{item.folder_name || '—'}</td>
                            <td className="num" dir="ltr">{formatBytes(item.db_bytes)}</td>
                            <td className="num">{item.schema_version ?? '—'}</td>
                            <td>
                              <button
                                type="button"
                                className="btn btn-outline btn-sm"
                                disabled={busy || !canRestore}
                                title={!canRestore ? t('db.backup-unavailable') : undefined}
                                onClick={() => void restoreBackup(item)}
                              >
                                {restoring
                                  ? t('db.backup-restoring')
                                  : <><RotateCcw size={14} strokeWidth={1.8} /> {t('db.backup-restore')}</>}
                              </button>{' '}
                              <button
                                type="button"
                                className="btn btn-outline btn-sm"
                                disabled={busy || deleting}
                                onClick={() => void deleteBackup(item)}
                              >
                                {deleting
                                  ? t('loading')
                                  : <><Trash2 size={14} strokeWidth={1.8} /> {t('db.backup-delete')}</>}
                              </button>
                            </td>
                          </tr>
                        );
                      })}
                    </tbody>
                  </table>
                  {backups.length > backupPageSize || backupPage > 1 ? (
                    <div style={{ display: 'flex', alignItems: 'center', gap: 10, flexWrap: 'wrap', marginTop: 12, fontSize: 13 }}>
                      <label style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                        <span className="text-secondary">{t('db.backup-page-size')}</span>
                        <select
                          className="form-input"
                          style={{ width: 'auto', padding: '4px 8px' }}
                          value={backupPageSize}
                          onChange={(e) => { setBackupPageSize(Number(e.target.value)); setBackupPage(1); }}
                        >
                          {[10, 25, 50].map((s) => <option key={s} value={s}>{s}</option>)}
                        </select>
                      </label>
                      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                        <button
                          type="button"
                          className="btn btn-outline btn-sm"
                          disabled={backupPage <= 1}
                          onClick={() => setBackupPage((p) => Math.max(1, p - 1))}
                        >
                          {t('db.backup-page-prev')}
                        </button>
                        <span className="num">
                          {tf('db.backup-page-indicator', { page: backupPage, total: Math.max(1, Math.ceil(backups.length / backupPageSize)) })}
                        </span>
                        <button
                          type="button"
                          className="btn btn-outline btn-sm"
                          disabled={backupPage >= Math.ceil(backups.length / backupPageSize)}
                          onClick={() => setBackupPage((p) => p + 1)}
                        >
                          {t('db.backup-page-next')}
                        </button>
                      </div>
                    </div>
                  ) : null}
                </div>
              )}
            </>
          ) : null}
        </div>
      </div>
    </div>
  );
}
