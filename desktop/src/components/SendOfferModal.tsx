import React, { useEffect, useState } from 'react';
import { api } from '../api';
import { Send, Search } from 'lucide-react';
import {
  clientRecordPhone,
  describeWhatsAppFailure,
  shareOfferToWhatsApp
} from '../whatsapp';
import PaginationBar, { usePagedItems } from './PaginationBar';
import { TABLE_PAGE_SIZE } from '../pagination';

type Offer = {
  country: string;
  currency: string;
  id: string;
  name: string;
  city?: string | null;
  district?: string | null;
  price?: number | null;
  area?: number | null;
};

type ClientRow = {
  id: string;
  name: string;
  phone?: string | null;
  whatsapp?: string | null;
  phone_secondary?: string | null;
  city?: string | null;
};

type Props =
  | { client: ClientRow; property?: undefined; onClose: () => void }
  | { property: Offer; client?: undefined; onClose: () => void };

export default function SendOfferModal(props: Props) {
  const { onClose } = props;
  const pickClient = Boolean(props.property);
  const [offers, setOffers] = useState<Offer[]>([]);
  const [clients, setClients] = useState<ClientRow[]>([]);
  const [q, setQ] = useState('');
  const [sending, setSending] = useState(false);
  const [loadError, setLoadError] = useState('');
  const [actionError, setActionError] = useState('');
  const [notice, setNotice] = useState('');

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        if (pickClient) {
          const list = await api('/api/clients/options');
          if (!cancelled) setClients(Array.isArray(list) ? list : []);
        } else {
          const propsData = await api('/api/properties?limit=200');
          if (!cancelled) setOffers(propsData.items || []);
        }
      } catch (err: any) {
        if (!cancelled) setLoadError(err?.message || 'تعذر تحميل القائمة');
      }
    })();
    return () => { cancelled = true; };
  }, [pickClient]);

  const fmtPrice = (p: any, currency: string) => (p && !isNaN(p) ? Number(p).toLocaleString('ar-SA') + ' ' + currency : '—');

  const filteredOffers = offers.filter((o) =>
    (o.name || '').includes(q) ||
    (o.city || '').includes(q) ||
    (o.district || '').includes(q)
  );

  const filteredClients = clients.filter((c) =>
    (c.name || '').includes(q) ||
    (c.phone || '').includes(q) ||
    (c.city || '').includes(q)
  );
  const pagedClients = usePagedItems(filteredClients, TABLE_PAGE_SIZE, q);
  const pagedOffers = usePagedItems(filteredOffers, TABLE_PAGE_SIZE, q);

  const share = async (client: ClientRow, offer: Offer) => {
    setActionError('');
    setNotice('');
    setSending(true);
    try {
      let phoneSource = client;
      try {
        const full = await api(`/api/clients/${client.id}`);
        phoneSource = { ...client, ...full };
      } catch {
        phoneSource = client;
      }
      await shareOfferToWhatsApp({
        clientId: client.id,
        phoneRaw: clientRecordPhone(phoneSource),
        clientName: client.name,
        propertyId: offer.id,
        property: offer
      });
      setNotice('تم فتح محادثة واتساب');
    } catch (err: unknown) {
      setActionError(describeWhatsAppFailure(err));
    } finally {
      setSending(false);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" style={{ width: 620, maxWidth: '94%' }} onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <div className="modal-title">
            {pickClient
              ? `مشاركة عبر واتساب — ${props.property?.name}`
              : `مشاركة عرض عبر واتساب — ${props.client?.name}`}
          </div>
          <button type="button" className="modal-close" onClick={onClose} aria-label="إغلاق">✕</button>
        </div>
        <div className="modal-body">
          <p style={{ fontSize: 12, color: 'var(--muted-foreground)', marginBottom: 12, lineHeight: 1.6 }}>
            {pickClient
              ? 'اختر العميل. سيُفتح واتساب بنص العرض فقط، دون إرفاق ملفات.'
              : 'اختر العرض. سيُفتح واتساب بنص العرض فقط، دون إرفاق ملفات.'}
          </p>
          {loadError && <div className="error-alert" style={{ marginBottom: 12 }}>{loadError}</div>}
          {actionError && <div className="error-alert" style={{ marginBottom: 12 }}>{actionError}</div>}
          {notice && <div className="success-alert" style={{ marginBottom: 12 }}>{notice}</div>}
          <input
            className="search-input"
            placeholder={pickClient ? 'ابحث عن العميل بالاسم أو الجوال...' : 'ابحث في العروض (الاسم، المدينة، الحي)...'}
            value={q}
            onChange={(e) => setQ(e.target.value)}
            style={{ marginBottom: 14 }}
          />
          <div>
            {pickClient ? (
              filteredClients.length === 0 ? (
                <div className="empty-state">
                  <div className="icon"><Search size={40} strokeWidth={1.4} /></div>
                  <h3>لا يوجد عملاء مطابقون</h3>
                  <p>أضف عميلًا من صفحة العملاء أولًا</p>
                </div>
              ) : (
                <>
                {pagedClients.slice.map((c) => (
                  <div
                    key={c.id}
                    style={{
                      display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 12,
                      padding: '10px 12px', border: '1px solid var(--gray-border)',
                      borderRadius: 10, marginBottom: 8, background: 'var(--card, #fff)'
                    }}
                  >
                    <div style={{ minWidth: 0 }}>
                      <div style={{ fontWeight: 700, color: 'var(--ink)' }}>{c.name}</div>
                      <div style={{ fontSize: 12, color: 'var(--gray)', marginTop: 2 }}>
                        {c.phone || 'بدون جوال'}{c.city ? ` • ${c.city}` : ''}
                      </div>
                    </div>
                    <button
                      type="button"
                      className="btn btn-primary btn-sm"
                      disabled={sending}
                      onClick={() => props.property && share(c, props.property)}
                    >
                      <Send size={13} strokeWidth={1.8} /> {sending ? 'جارٍ التجهيز...' : 'واتساب لهذا العميل'}
                    </button>
                  </div>
                ))}
                <PaginationBar total={pagedClients.total} page={pagedClients.page} onPage={pagedClients.setPage} label="عميل" />
                </>
              )
            ) : filteredOffers.length === 0 ? (
              <div className="empty-state">
                <div className="icon"><Search size={40} strokeWidth={1.4} /></div>
                <h3>لا توجد عروض مطابقة</h3>
                <p>أضف عروضًا من صفحة «العروض العقارية»</p>
              </div>
            ) : (
              <>
              {pagedOffers.slice.map((o) => (
                <div
                  key={o.id}
                  style={{
                    display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 12,
                    padding: '10px 12px', border: '1px solid var(--gray-border)',
                    borderRadius: 10, marginBottom: 8, background: 'var(--card, #fff)'
                  }}
                >
                  <div style={{ minWidth: 0 }}>
                    <div style={{ fontWeight: 700, color: 'var(--ink)' }}>{o.name}</div>
                    <div style={{ fontSize: 12, color: 'var(--gray)', marginTop: 2 }}>
                      {o.district || '—'}، {o.city || '—'} • {fmtPrice(o.price, o.currency)}
                    </div>
                  </div>
                  <button
                    type="button"
                    className="btn btn-primary btn-sm"
                    disabled={sending}
                    onClick={() => props.client && share(props.client, o)}
                  >
                    <Send size={13} strokeWidth={1.8} /> {sending ? 'جارٍ التجهيز...' : 'مشاركة عبر واتساب'}
                  </button>
                </div>
              ))}
              <PaginationBar total={pagedOffers.total} page={pagedOffers.page} onPage={pagedOffers.setPage} label="عرض" />
              </>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
