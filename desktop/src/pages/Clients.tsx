import React, { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { api } from '../api';
import { Users } from 'lucide-react';
import SendOfferModal from '../components/SendOfferModal';
import PaginationBar from '../components/PaginationBar';
import { TABLE_PAGE_SIZE } from '../pagination';
import { CLIENT_ROLES, CLIENT_STATUSES, roleLabel } from '../domain/labels';
import {
  clientRecordPhone,
  describeWhatsAppFailure,
  openClientWhatsAppConversation
} from '../whatsapp';
import SectionGuideBanner from '../components/SectionGuide';
import { useLocale } from '../hooks/useLocale';
import { t as tLocale } from '../domain/locale';

export default function Clients() {
  const { t } = useLocale();
  const [items, setItems] = useState<any[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [search, setSearch] = useState('');
  const [role, setRole] = useState('');
  const [status, setStatus] = useState('');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [sendClient, setSendClient] = useState<any | null>(null);
  const navigate = useNavigate();

  useEffect(() => {
    loadData();
  }, [page, role, status]);

  const loadData = async () => {
    setLoading(true);
    setError('');
    try {
      const params = new URLSearchParams({ page: String(page), limit: String(TABLE_PAGE_SIZE) });
      if (search.trim()) params.set('search', search.trim());
      if (role) params.set('role', role);
      if (status) params.set('status', status);
      const data = await api(`/api/clients?${params}`);
      setItems(data.items || []);
      setTotal(data.total || 0);
    } catch (err: any) {
      setError(tLocale('client.load-fail'));
      setItems([]);
      setTotal(0);
    } finally {
      setLoading(false);
    }
  };

  const handleDelete = async (id: string) => {
    if (!confirm(t('client.delete-confirm'))) return;
    try {
      await api(`/api/clients/${id}`, { method: 'DELETE' });
      loadData();
    } catch (err: any) {
      setError(err.message || t('delete_failed'));
    }
  };

  const openWhatsApp = async (client: { id: string; whatsapp?: string; phone?: string; phone_secondary?: string }) => {
    setError('');
    try {
      await openClientWhatsAppConversation(client.id, clientRecordPhone(client));
    } catch (err: unknown) {
      setError(describeWhatsAppFailure(err));
    }
  };

  return (
    <div>
      <div className="card-header">
        <div className="card-title"><Users size={17} strokeWidth={1.8} style={{ verticalAlign: '-3px' }} /> {t('clients')} <SectionGuideBanner id="clients" /></div>
        <button className="btn btn-primary" onClick={() => navigate('/clients/new')}>
          {t('client.add')}
        </button>
      </div>

      {error && <div className="error-alert" role="alert">{error} <button type="button" className="btn btn-outline" onClick={loadData}>{t('retry')}</button></div>}

      <div className="content-card obr-panel">
        <div className="toolbar">
          <input
            className="search-input"
            autoComplete="off"
            placeholder={t('client.search-ph')}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && (page === 1 ? loadData() : setPage(1))}
          />
          <select className="select-input" value={role} onChange={(e) => { setRole(e.target.value); setPage(1); }}>
            <option value="">{t('all_roles')}</option>
            {CLIENT_ROLES.map((r) => <option key={r.value} value={r.value}>{r.label}</option>)}
          </select>
          <select className="select-input" value={status} onChange={(e) => { setStatus(e.target.value); setPage(1); }}>
            <option value="">{t('all_statuses')}</option>
            {CLIENT_STATUSES.map((s) => <option key={s.value} value={s.value}>{s.label}</option>)}
          </select>
          <button className="btn btn-outline" onClick={() => { if (page === 1) loadData(); else setPage(1); }}>{t('search_action')}</button>
        </div>

        {loading ? (
          <div className="obr-panel-body">{t('loading')}</div>
        ) : error ? null : items.length === 0 ? (
          <div className="empty-state">
            <div className="icon"><Users size={48} strokeWidth={1.4} /></div>
            <h3>{t('client.empty')}</h3>
            <p>{search || role || status ? t('filter_no_match') : t('client.empty-hint')}</p>
          </div>
        ) : (
          <div className="table-container">
            <table className="data-table">
              <thead>
                <tr>
                  <th>{t('name')}</th>
                  <th>{t('mobile')}</th>
                  <th>{t('roles')}</th>
                  <th>{t('city')}</th>
                  <th>{t('status')}</th>
                  <th>{t('last_contact')}</th>
                  <th>{t('actions')}</th>
                </tr>
              </thead>
              <tbody>
                {items.map((c: any) => {
                  const roles: string[] = Array.isArray(c.roles) && c.roles.length
                    ? c.roles
                    : (c.relationship_type ? [c.relationship_type] : []);
                  return (
                    <tr key={c.id}>
                      <td>
                        <div>
                          {c.name}
                          {c.is_training ? <span className="badge training-badge">{t('training.badge')}</span> : null}
                        </div>
                        {c.company ? <div className="text-secondary">{c.company}</div> : null}
                      </td>
                      <td className="num">{c.phone || '—'}</td>
                      <td>
                        {roles.length === 0 ? '—' : roles.map((r) => (
                          <span key={r} className="badge" style={{ marginInlineStart: 4, background: 'var(--primary-light)', color: 'var(--primary)' }}>
                            {roleLabel(r)}
                          </span>
                        ))}
                      </td>
                      <td>{c.city || '—'}</td>
                      <td>{CLIENT_STATUSES.find((s) => s.value === c.status)?.label || c.status || '—'}</td>
                      <td className="num">{c.last_contact || '—'}</td>
                      <td>
                        <button className="btn btn-sm btn-outline" onClick={() => navigate(`/clients/${c.id}`)}>{t('open')}</button>
                        <button className="btn btn-sm btn-primary" onClick={() => setSendClient(c)}>{t('client.share-offer')}</button>
                        <button type="button" className="btn btn-sm btn-success" onClick={() => openWhatsApp(c)}>{t('whatsapp')}</button>
                        <button className="btn btn-sm btn-danger" onClick={() => handleDelete(c.id)}>{t('delete')}</button>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}

        <PaginationBar total={total} page={page} onPage={setPage} label={t('client.label')} />
        {sendClient && (
          <SendOfferModal client={sendClient} onClose={() => setSendClient(null)} />
        )}
      </div>
    </div>
  );
}
