import React, { useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { api } from '../api';
import { MessageSquare, Building2, UserRoundPen, UserRound } from 'lucide-react';
import SendOfferModal from '../components/SendOfferModal';
import {
  CLIENT_CATEGORIES, CLIENT_ROLES, CLIENT_SOURCES, CLIENT_STATUSES, formatMoney
} from '../domain/labels';
import {
  clientRecordPhone,
  communicationTypeLabel,
  describeWhatsAppFailure,
  openClientWhatsAppConversation
} from '../whatsapp';
import PaginationBar, { usePagedItems } from '../components/PaginationBar';
import { TABLE_PAGE_SIZE } from '../pagination';
import { noAutofill } from '../noAutofill';
import { useLocale } from '../hooks/useLocale';

const EMPTY_FORM = {
  name: '',
  phone: '',
  phone_secondary: '',
  whatsapp: '',
  email: '',
  city: '',
  company: '',
  source: '',
  status: 'active',
  category: '',
  interests: '',
  required_property_type: '',
  budget: '',
  notes: '',
  follow_up_date: ''
};

export default function ClientForm() {
  const { id } = useParams();
  const navigate = useNavigate();
  const { t, tf } = useLocale();
  const [form, setForm] = useState(EMPTY_FORM);
  const [roles, setRoles] = useState<string[]>([]);
  const [communications, setCommunications] = useState<any[]>([]);
  const [linkedProperties, setLinkedProperties] = useState<any[]>([]);
  const [meta, setMeta] = useState({ created_at: '', last_contact: '' });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [commType, setCommType] = useState('whatsapp');
  const [commNotes, setCommNotes] = useState('');
  const [sendClient, setSendClient] = useState<any | null>(null);
  const [waBusy, setWaBusy] = useState(false);
  const [waNotice, setWaNotice] = useState('');
  const isEdit = Boolean(id);
  const pagedComms = usePagedItems(communications, TABLE_PAGE_SIZE, id);
  const pagedLinked = usePagedItems(linkedProperties, TABLE_PAGE_SIZE, id);

  const COMM_TYPES = [
    { value: 'call', label: t('comm.call') },
    { value: 'whatsapp', label: t('comm.whatsapp') },
    { value: 'message', label: t('comm.message') },
    { value: 'meeting', label: t('comm.meeting') },
    { value: 'email', label: t('comm.email') },
    { value: 'note', label: t('comm.note') }
  ];

  useEffect(() => {
    if (id) {
      loadClient();
      return;
    }
    setForm(EMPTY_FORM);
    setRoles([]);
    setCommunications([]);
    setLinkedProperties([]);
    setMeta({ created_at: '', last_contact: '' });
    setError('');
    setCommNotes('');
    setCommType('whatsapp');
    setSendClient(null);
    setWaNotice('');
  }, [id]);

  const loadClient = async () => {
    setLoading(true);
    setError('');
    try {
      const data = await api(`/api/clients/${id}`);
      setForm({
        name: data.name || '',
        phone: data.phone || '',
        phone_secondary: data.phone_secondary || '',
        whatsapp: data.whatsapp || '',
        email: data.email || '',
        city: data.city || '',
        company: data.company || '',
        source: data.source || '',
        status: data.status || '',
        category: data.category || '',
        interests: data.interests || '',
        required_property_type: data.required_property_type || '',
        budget: data.budget || '',
        notes: data.notes || '',
        follow_up_date: data.follow_up_date || ''
      });
      setRoles(Array.isArray(data.roles) ? data.roles : []);
      setCommunications(data.communications || []);
      setLinkedProperties(data.linked_properties || []);
      setMeta({ created_at: data.created_at || '', last_contact: data.last_contact || '' });
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>) => {
    const { name, value } = e.target;
    setForm((prev) => ({ ...prev, [name]: value }));
  };

  const toggleRole = (value: string) => {
    setRoles((prev) => prev.includes(value) ? prev.filter((r) => r !== value) : [...prev, value]);
  };

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setLoading(true);
    setError('');
    try {
      const fromDom: Record<string, string> = {};
      new FormData(e.currentTarget).forEach((value, key) => {
        if (typeof value === 'string') fromDom[key] = value;
      });
      const body = { ...form, ...fromDom, roles };
      if (id) {
        await api(`/api/clients/${id}`, { method: 'PUT', body });
      } else {
        await api('/api/clients', { method: 'POST', body });
      }
      navigate('/clients');
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  const addCommunication = async () => {
    if (!commNotes.trim()) return;
    try {
      await api(`/api/clients/${id}/communications`, {
        method: 'POST',
        body: { type: commType, notes: commNotes }
      });
      setCommNotes('');
      loadClient();
    } catch (err: any) {
      setError(err.message);
    }
  };

  const openWhatsApp = async () => {
    if (!id) return;
    setError('');
    setWaNotice('');
    setWaBusy(true);
    try {
      await openClientWhatsAppConversation(id, clientRecordPhone(form));
      setWaNotice(t('client.form.wa-opened'));
      await loadClient();
    } catch (err: unknown) {
      setError(describeWhatsAppFailure(err));
    } finally {
      setWaBusy(false);
    }
  };

  return (
    <div>
      <div className="card-header">
        <div className="card-title"><UserRoundPen size={17} strokeWidth={1.8} style={{ verticalAlign: '-3px' }} /> {isEdit ? t('client.form.edit') : t('client.form.new')}</div>
        <div style={{ display: 'flex', gap: 8 }}>
          {isEdit && (
            <button
              type="button"
              className="btn btn-primary"
              onClick={() => setSendClient({ id, name: form.name, whatsapp: form.whatsapp, phone: form.phone })}
            >
              {t('client.form.share-wa')}
            </button>
          )}
          {isEdit && (
            <button type="button" className="btn btn-success" disabled={waBusy} onClick={openWhatsApp}>
              {waBusy ? t('client.form.wa-opening') : t('whatsapp')}
            </button>
          )}
          <button className="btn btn-outline" onClick={() => navigate('/clients')}>{t('go_back')}</button>
        </div>
      </div>

      {error && <div className="error-alert">{error}</div>}
      {waNotice && <div className="success-alert">{waNotice}</div>}

      <div className="content-card">
        <form {...noAutofill} onSubmit={handleSubmit}>
          <h3><UserRound size={16} strokeWidth={1.8} style={{ verticalAlign: '-2px' }} /> {t('client.form.section-basic')}</h3>
          <div className="form-grid">
            <div className="form-group">
              <label htmlFor="client-name">{t('client.form.name')}</label>
              <input id="client-name" className="form-input" autoComplete="off" name="name" onChange={handleChange} value={form.name} required />
            </div>
            <div className="form-group">
              <label htmlFor="client-phone">{t('client.form.phone')}</label>
              <input id="client-phone" className="form-input" autoComplete="off" name="phone" onChange={handleChange} value={form.phone} />
              {isEdit && (
                <button
                  type="button"
                  className="btn btn-success"
                  style={{ marginTop: 8, width: '100%' }}
                  disabled={waBusy}
                  onClick={openWhatsApp}
                >
                  {waBusy ? t('client.form.wa-opening') : t('whatsapp')}
                </button>
              )}
            </div>
            <div className="form-group">
              <label htmlFor="client-phone2">{t('client.form.phone2')}</label>
              <input id="client-phone2" className="form-input" autoComplete="off" name="phone_secondary" onChange={handleChange} value={form.phone_secondary} />
            </div>
            <div className="form-group">
              <label htmlFor="client-wa">{t('client.form.wa')}</label>
              <input id="client-wa" className="form-input" autoComplete="off" name="whatsapp" onChange={handleChange} value={form.whatsapp} />
            </div>
            <div className="form-group">
              <label htmlFor="client-email">{t('client.form.email')}</label>
              <input id="client-email" className="form-input" autoComplete="off" name="email" type="email" onChange={handleChange} value={form.email} />
            </div>
            <div className="form-group">
              <label htmlFor="client-city">{t('city')}</label>
              <input id="client-city" className="form-input" autoComplete="off" name="city" onChange={handleChange} value={form.city} />
            </div>
            <div className="form-group">
              <label htmlFor="client-company">{t('client.form.company')}</label>
              <input id="client-company" className="form-input" autoComplete="off" name="company" onChange={handleChange} value={form.company} />
            </div>
            <div className="form-group">
              <label htmlFor="client-source">{t('client.form.source')}</label>
              <select id="client-source" className="form-input" autoComplete="off" name="source" onChange={handleChange} value={form.source}>
                <option value="">—</option>
                {CLIENT_SOURCES.map((s) => <option key={s.value} value={s.value}>{s.label}</option>)}
              </select>
            </div>
            <div className="form-group">
              <label htmlFor="client-status">{t('client.form.status')}</label>
              <select id="client-status" className="form-input" autoComplete="off" name="status" onChange={handleChange} value={form.status}>
                <option value="">—</option>
                {CLIENT_STATUSES.map((s) => <option key={s.value} value={s.value}>{s.label}</option>)}
              </select>
            </div>
            <div className="form-group">
              <label htmlFor="client-category">{t('client.form.category')}</label>
              <select id="client-category" className="form-input" autoComplete="off" name="category" onChange={handleChange} value={form.category}>
                <option value="">—</option>
                {CLIENT_CATEGORIES.map((s) => <option key={s.value} value={s.value}>{s.label}</option>)}
              </select>
            </div>
            <div className="form-group">
              <label htmlFor="client-follow">{t('client.form.followup')}</label>
              <input id="client-follow" className="form-input" autoComplete="off" name="follow_up_date" type="date" onChange={handleChange} value={form.follow_up_date} />
            </div>
            <div className="form-group full-row">
              <span className="form-label-text">{t('client.form.roles')}</span>
              <div className="role-checkboxes">
                {CLIENT_ROLES.map((r) => (
                  <label key={r.value} className="role-check">
                    <input
                      type="checkbox"
                      checked={roles.includes(r.value)}
                      onChange={() => toggleRole(r.value)}
                    />
                    {r.label}
                  </label>
                ))}
              </div>
            </div>
            <div className="form-group">
              <label htmlFor="client-interest">{t('client.form.interests')}</label>
              <input id="client-interest" className="form-input" autoComplete="off" name="interests" onChange={handleChange} value={form.interests} />
            </div>
            <div className="form-group">
              <label htmlFor="client-req">{t('client.form.req-type')}</label>
              <input id="client-req" className="form-input" autoComplete="off" name="required_property_type" onChange={handleChange} value={form.required_property_type} />
            </div>
            <div className="form-group">
              <label htmlFor="client-budget">{t('client.form.budget')}</label>
              <input id="client-budget" className="form-input" autoComplete="off" name="budget" onChange={handleChange} value={form.budget} />
            </div>
            <div className="form-group full-row">
              <label htmlFor="client-notes">{t('notes')}</label>
              <textarea id="client-notes" className="form-input" autoComplete="off" name="notes" rows={3} onChange={handleChange} value={form.notes} />
            </div>
          </div>

          {isEdit && (
            <p className="text-secondary" style={{ marginTop: 12 }}>
              {tf('client.form.meta', { created: meta.created_at || '—', last: meta.last_contact || '—' })}
            </p>
          )}

          <div style={{ marginTop: 20, display: 'flex', gap: 12 }}>
            <button className="btn btn-primary" disabled={loading}>{loading ? t('saving') : t('save')}</button>
            <button type="button" className="btn btn-outline" onClick={() => navigate('/clients')}>{t('cancel')}</button>
          </div>
        </form>
      </div>

      {isEdit && (
        <>
          <div className="content-card">
            <div className="card-title" style={{ marginBottom: 16 }}>
              <MessageSquare size={16} strokeWidth={1.8} style={{ verticalAlign: '-2px' }} /> {t('client.form.comms')}
            </div>
            <div style={{ display: 'flex', gap: 8, marginBottom: 16 }}>
              <select className="select-input" value={commType} onChange={(e) => setCommType(e.target.value)}>
                {COMM_TYPES.map((ct) => <option key={ct.value} value={ct.value}>{ct.label}</option>)}
              </select>
              <input
                className="search-input"
                autoComplete="off"
                placeholder={t('client.form.comm-ph')}
                value={commNotes}
                onChange={(e) => setCommNotes(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && addCommunication()}
              />
              <button className="btn btn-primary" onClick={addCommunication}>{t('add')}</button>
            </div>
            {communications.length === 0 ? (
              <div className="empty-state"><p>{t('client.form.no-comms')}</p></div>
            ) : (
              <>
              <div className="timeline">
                {pagedComms.slice.map((c) => (
                  <div key={c.id} className="timeline-item">
                    <div className="time">{c.created_at} — {communicationTypeLabel(c.type, COMM_TYPES)}</div>
                    {c.subject && <div className="title">{c.subject}</div>}
                    {c.notes && <div className="desc">{c.notes}</div>}
                  </div>
                ))}
              </div>
              <PaginationBar total={pagedComms.total} page={pagedComms.page} onPage={pagedComms.setPage} label={t('comm_label')} />
              </>
            )}
          </div>

          <div className="content-card">
            <div className="card-title" style={{ marginBottom: 16 }}>
              <Building2 size={16} strokeWidth={1.8} style={{ verticalAlign: '-2px' }} /> {t('client.form.linked')}
            </div>
            {linkedProperties.length === 0 ? (
              <div className="empty-state"><p>{t('client.form.no-linked')}</p></div>
            ) : (
              <>
              <div className="table-container">
                <table className="data-table">
                  <thead>
                    <tr><th>{t('listing-tag')}</th><th>{t('price')}</th><th>{t('client.form.location')}</th><th>{t('client.form.link-type')}</th></tr>
                  </thead>
                  <tbody>
                    {pagedLinked.slice.map((lp) => (
                      <tr key={lp.id || lp.property_id} style={{ cursor: lp.property_id ? 'pointer' : undefined }} onDoubleClick={() => lp.property_id && navigate(`/properties/${lp.property_id}`)}>
                        <td>{lp.property_name}</td>
                        <td className="num">{formatMoney(lp.price, lp.currency)}</td>
                        <td>{lp.city || '—'} {lp.district || ''}</td>
                        <td>{lp.link_type === 'offer_share' || lp.link_type === 'whatsapp_opened' ? t('client.share-offer') : (lp.link_type || '—')}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              <PaginationBar total={pagedLinked.total} page={pagedLinked.page} onPage={pagedLinked.setPage} label={t('property_label_short')} />
              </>
            )}
          </div>
        </>
      )}
      {sendClient && (
        <SendOfferModal client={sendClient} onClose={() => setSendClient(null)} />
      )}
    </div>
  );
}
