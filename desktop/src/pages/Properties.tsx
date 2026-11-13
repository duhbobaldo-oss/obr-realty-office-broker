import React, { useEffect, useState, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import { api } from '../api';
import { LayoutGrid, Table2, Building2, MapPin, Ruler,
  ChevronLeft, ChevronRight, Images, Pencil, Copy, Trash2, ImagePlus, MessageCircle } from 'lucide-react';
import SendOfferModal from '../components/SendOfferModal';
import {
  PROPERTY_PURPOSES, PROPERTY_STATUSES, formatMoney, purposeLabel, sourceTypeLabel
} from '../domain/labels';
import { MAX_PROPERTY_IMAGES, fileToPropertyImage, uploadPropertyImage, usePropertyImageSrc } from '../propertyImages';

/** صورة تُحلّ تلقائيًا: data URL مباشرة، والمسار النسبي يُجلب كـdata URL مؤقتًا. */
function PropertyImg({
  src,
  alt,
  className,
  onClick
}: {
  src?: string | null;
  alt?: string;
  className?: string;
  onClick?: () => void;
}) {
  const resolved = usePropertyImageSrc(src);
  if (!resolved) {
    return <div className={className} aria-label={alt} />;
  }
  return <img src={resolved} alt={alt} className={className} onClick={onClick} loading="lazy" />;
}
import PaginationBar from '../components/PaginationBar';
import { TABLE_PAGE_SIZE } from '../pagination';
import SectionGuideBanner from '../components/SectionGuide';
import { useLocale } from '../hooks/useLocale';
import { t as tLocale } from '../domain/locale';
import { propertyPricePerMeter } from '../domain/propertyPricing';

const STATUS_CLASS: Record<string, string> = {
  available: 'badge-available',
  negotiating: 'badge-negotiating',
  reserved: 'badge-reserved',
  sold: 'badge-sold',
  suspended: 'badge-suspended',
  expired: 'badge-expired'
};

const PAGE_SIZE = TABLE_PAGE_SIZE;

function GalleryModal({
  propertyId,
  name,
  startIndex = 0,
  onClose
}: {
  propertyId: string;
  name: string;
  startIndex?: number;
  onClose: () => void;
}) {
  const { t, tf } = useLocale();
  const [images, setImages] = useState<string[]>([]);
  const [idx, setIdx] = useState(startIndex);

  useEffect(() => {
    api(`/api/properties/${propertyId}/images`)
      .then((d) => {
        const urls = (d as any[]).map((x) => x.image_url).filter(Boolean);
        setImages(urls);
        setIdx(Math.min(Math.max(0, startIndex), Math.max(0, urls.length - 1)));
      })
      .catch(() => setImages([]));
  }, [propertyId, startIndex]);

  const prev = () => setIdx((i) => (i - 1 + images.length) % images.length);
  const next = () => setIdx((i) => (i + 1) % images.length);

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="gallery-box" onClick={(e) => e.stopPropagation()}>
        <div className="gallery-header">
          <span><Images size={16} strokeWidth={1.8} style={{ verticalAlign: '-3px' }} /> {tf('prop.gallery-title', { name })}</span>
          <button className="gallery-close" onClick={onClose}>✕</button>
        </div>
        {images.length === 0 ? (
          <div className="gallery-empty">{t('prop.gallery-empty')}</div>
        ) : (
          <>
            <div className="gallery-stage">
              <PropertyImg src={images[idx]} alt={`${name} ${idx + 1}`} />
              {images.length > 1 && (
                <>
                  <button className="gallery-nav right" onClick={prev}><ChevronRight size={22} /></button>
                  <button className="gallery-nav left" onClick={next}><ChevronLeft size={22} /></button>
                </>
              )}
              <span className="gallery-counter">{idx + 1} / {images.length}</span>
            </div>
            {images.length > 1 && (
              <div className="gallery-thumbs">
                {images.map((src, i) => (
                  <PropertyImg key={i} src={src} alt="" className={i === idx ? 'active' : ''} onClick={() => setIdx(i)} />
                ))}
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}

export default function Properties() {
  const { t, tf } = useLocale();
  const [items, setItems] = useState<any[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [search, setSearch] = useState('');
  const [status, setStatus] = useState('');
  const [purpose, setPurpose] = useState('');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [view, setView] = useState<'grid' | 'table'>('grid');
  const [gallery, setGallery] = useState<{ id: string; name: string; index?: number } | null>(null);
  const [shareProperty, setShareProperty] = useState<any | null>(null);
  const navigate = useNavigate();
  const fileRef = useRef<HTMLInputElement>(null);
  const [imgTarget, setImgTarget] = useState<string | null>(null);
  const [uploading, setUploading] = useState(false);

  useEffect(() => {
    loadData();
  }, [page, status, purpose]);

  const loadData = async () => {
    setLoading(true);
    setError('');
    try {
      const params = new URLSearchParams({ page: String(page), limit: String(PAGE_SIZE) });
      if (search.trim()) params.set('search', search.trim());
      if (status) params.set('status', status);
      if (purpose) params.set('purpose', purpose);
      const data = await api(`/api/properties?${params}`);
      setItems(data.items || []);
      setTotal(data.total || 0);
    } catch (err: any) {
      setError(tLocale('prop.load-fail'));
      setItems([]);
      setTotal(0);
    } finally {
      setLoading(false);
    }
  };

  const handleSearch = () => {
    if (page === 1) loadData();
    else setPage(1);
  };

  const handleDelete = async (id: string) => {
    if (!confirm(t('prop.delete-confirm'))) return;
    try {
      await api(`/api/properties/${id}`, { method: 'DELETE' });
      loadData();
    } catch (err: any) {
      setError(err.message || t('delete_failed'));
    }
  };

  const handleDuplicate = async (id: string) => {
    try {
      await api(`/api/properties/${id}/duplicate`, { method: 'POST' });
      loadData();
    } catch (err: any) {
      setError(err.message || t('duplicate_failed'));
    }
  };

  const handleImageSelected = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    const targetId = imgTarget;
    if (!file || !targetId) return;
    const prop = items.find((i) => i.id === targetId);
    const count = Number(prop?.image_count || 0);
    if (count >= MAX_PROPERTY_IMAGES) {
      alert(t('prop.max-images'));
      if (fileRef.current) fileRef.current.value = '';
      setImgTarget(null);
      return;
    }
    setUploading(true);
    try {
      const dataUrl = await fileToPropertyImage(file);
      await uploadPropertyImage(targetId, dataUrl, !prop?.main_image);
      await loadData();
    } catch (err: any) {
      alert(err.message || t('prop.image-fail'));
    } finally {
      setUploading(false);
      if (fileRef.current) fileRef.current.value = '';
      setImgTarget(null);
    }
  };

  return (
    <div>
      <div className="card-header">
        <div className="card-title"><Building2 size={17} strokeWidth={1.8} style={{ verticalAlign: '-3px' }} /> {t('properties')} <SectionGuideBanner id="properties" /></div>
        <button className="btn btn-primary" onClick={() => navigate('/properties/new')}>
          {t('prop.add')}
        </button>
      </div>

      {uploading && <div className="success-alert">{t('prop.uploading')}</div>}

      {error && <div className="error-alert" role="alert">{error} <button type="button" className="btn btn-outline" onClick={loadData}>{t('retry')}</button></div>}

      <div className="content-card obr-panel">
        <div className="toolbar">
          <input
            className="search-input"
            autoComplete="off"
            placeholder={t('prop.search-ph')}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
          />
          <select className="select-input" value={status} onChange={(e) => { setStatus(e.target.value); setPage(1); }}>
            <option value="">{t('all_statuses')}</option>
            {PROPERTY_STATUSES.map((s) => (
              <option key={s.value} value={s.value}>{s.label}</option>
            ))}
          </select>
          <select className="select-input" value={purpose} onChange={(e) => { setPurpose(e.target.value); setPage(1); }}>
            <option value="">{t('all_purposes')}</option>
            {PROPERTY_PURPOSES.map((s) => (
              <option key={s.value} value={s.value}>{s.label}</option>
            ))}
          </select>
          <button className="btn btn-outline" onClick={handleSearch}>{t('search_action')}</button>
          <div style={{ marginRight: 'auto' }}>
            <button className={`btn btn-sm ${view === 'grid' ? 'btn-primary' : 'btn-outline'}`} onClick={() => setView('grid')}><LayoutGrid size={14} strokeWidth={1.8} /> {t('cards_view')}</button>
            <button className={`btn btn-sm ${view === 'table' ? 'btn-primary' : 'btn-outline'}`} onClick={() => setView('table')}><Table2 size={14} strokeWidth={1.8} /> {t('table_view')}</button>
          </div>
        </div>

        {loading ? (
          <div className="obr-panel-body">{t('loading')}</div>
        ) : error ? null : items.length === 0 ? (
          <div className="empty-state">
            <div className="icon"><Building2 size={48} strokeWidth={1.4} /></div>
            <h3>{t('prop.empty')}</h3>
            <p>{search || status || purpose ? t('filter_no_match') : t('prop.empty-hint')}</p>
          </div>
        ) : view === 'grid' ? (
          <div className="properties-grid">
            {items.map((p: any) => {
              const ppm = propertyPricePerMeter(p);
              const definitions = [
                p.area != null ? { label: t('area'), value: tf('market.area-m2', { n: p.area }), icon: <Ruler size={12} strokeWidth={1.8} /> } : null,
                p.purpose ? { label: t('purpose'), value: purposeLabel(p.purpose) } : null,
                p.source_type ? { label: t('source'), value: sourceTypeLabel(p.source_type) } : null,
                p.owner_name ? { label: t('owner'), value: p.owner_name } : null,
                ppm.value != null ? { label: t('ppm'), value: `${ppm.value.toLocaleString('en-US', { maximumFractionDigits: 2 })}${ppm.source === 'calculated' ? ' (محسوب)' : ''}` } : null,
                p.property_type ? { label: t('type'), value: p.property_type } : null
              ].filter(Boolean) as { label: string; value: string; icon?: React.ReactNode }[];
              return <div className="mag-card" key={p.id}>
                <div className="mag-image" onClick={() => setGallery({ id: p.id, name: p.name })} title={t('prop.browse-images')}>
                  {p.main_image ? (
                    <PropertyImg src={p.main_image} alt={p.name} />
                  ) : (
                    <div className="mag-placeholder">
                      <Building2 size={44} strokeWidth={1.2} />
                      <span>{t('prop.no-image')}</span>
                    </div>
                  )}
                  {Number(p.image_count || 0) > 1 && (
                    <div className="mag-pips" onClick={(e) => e.stopPropagation()}>
                      {Array.from({ length: Math.min(MAX_PROPERTY_IMAGES, Number(p.image_count) || 0) }, (_, i) => (
                        <button
                          key={i}
                          type="button"
                          className="mag-pip"
                          title={tf('prop.image-n', { n: i + 1 })}
                          onClick={() => setGallery({ id: p.id, name: p.name, index: i })}
                        >
                          {i + 1}
                        </button>
                      ))}
                    </div>
                  )}
                  <span className={`badge ${STATUS_CLASS[p.status] || 'badge-expired'}`}>
                    {PROPERTY_STATUSES.find((s) => s.value === p.status)?.label || p.status}
                  </span>
                </div>

                <div className="mag-body">
                  <h3 className="mag-title">
                    {p.name}
                    {p.is_training ? <span className="badge training-badge">{t('training.badge')}</span> : null}
                  </h3>
                  {(p.district || p.city) && <div className="mag-location">
                    <MapPin size={13} strokeWidth={1.8} /> {[p.district, p.city].filter(Boolean).join('، ')}
                  </div>}

                  {definitions.length > 0 && <div className="def-grid">
                    {definitions.map((d, i) => (
                      <span className="def-item" key={i}>
                        {d.icon && <span className="def-icon">{d.icon}</span>}
                        <span className="def-text">
                          <span className="def-label">{d.label}</span>
                          <span className="def-value">{d.value}</span>
                        </span>
                      </span>
                    ))}
                  </div>}

                  <div className="mag-footer">
                    {p.price != null && <div className="mag-price-block">
                      <div className="mag-price-label">{t('price')}</div>
                      <div className="mag-price">{formatMoney(p.price, p.currency)}</div>
                    </div>}
                    <div className="mag-card-actions">
                      <button
                        type="button"
                        className="mag-action mag-action-wa"
                        title={t('whatsapp')}
                        aria-label={t('whatsapp')}
                        onClick={() => setShareProperty(p)}
                      >
                        <MessageCircle size={15} strokeWidth={1.8} />
                      </button>
                      <button
                        type="button"
                        className="mag-action"
                        title={t('edit')}
                        aria-label={t('edit')}
                        onClick={() => navigate(`/properties/${p.id}`)}
                      >
                        <Pencil size={15} strokeWidth={1.8} />
                      </button>
                      <button
                        type="button"
                        className="mag-action"
                        title={Number(p.image_count || 0) >= MAX_PROPERTY_IMAGES ? t('prop.images-limit') : t('prop.add-image')}
                        aria-label={t('prop.add-image')}
                        disabled={uploading || Number(p.image_count || 0) >= MAX_PROPERTY_IMAGES}
                        onClick={() => { setImgTarget(p.id); fileRef.current?.click(); }}
                      >
                        <ImagePlus size={15} strokeWidth={1.8} />
                      </button>
                      <button
                        type="button"
                        className="mag-action"
                        title={t('duplicate_action')}
                        aria-label={t('duplicate_action')}
                        onClick={() => handleDuplicate(p.id)}
                      >
                        <Copy size={15} strokeWidth={1.8} />
                      </button>
                      <button
                        type="button"
                        className="mag-action mag-action-danger"
                        title={t('delete')}
                        aria-label={t('delete')}
                        onClick={() => handleDelete(p.id)}
                      >
                        <Trash2 size={15} strokeWidth={1.8} />
                      </button>
                    </div>
                  </div>
                </div>
              </div>;
            })}
          </div>
        ) : (
          <div className="table-container">
            <table>
              <thead>
                <tr>
                  <th>{t('name')}</th>
                  <th>{t('city')}</th>
                  <th>{t('district')}</th>
                  <th>{t('purpose')}</th>
                  <th>{t('source')}</th>
                  <th>{t('owner')}</th>
                  <th>{t('area')}</th>
                  <th>{t('price')}</th>
                  <th>{t('status')}</th>
                  <th>{t('actions')}</th>
                </tr>
              </thead>
              <tbody>
                {items.map((p: any) => (
                  <tr key={p.id} onDoubleClick={() => navigate(`/properties/${p.id}`)} style={{ cursor: 'pointer' }} title={t('prop.dblclick-edit')}>
                    <td>
                      {p.name}
                      {p.is_training ? <span className="badge training-badge">{t('training.badge')}</span> : null}
                    </td>
                    <td>{p.city || '—'}</td>
                    <td>{p.district || '—'}</td>
                    <td>{purposeLabel(p.purpose)}</td>
                    <td>{sourceTypeLabel(p.source_type)}</td>
                    <td>{p.owner_name || '—'}</td>
                    <td className="num">{p.area ? tf('market.area-m2', { n: p.area }) : '—'}</td>
                    <td className="num" style={{ color: 'var(--gold-dark)' }}>{formatMoney(p.price, p.currency)}</td>
                    <td>
                      <span className={`badge ${STATUS_CLASS[p.status] || 'badge-expired'}`}>
                        {PROPERTY_STATUSES.find((s) => s.value === p.status)?.label || p.status}
                      </span>
                    </td>
                    <td>
                      <button
                        className="btn btn-sm btn-success"
                        title={t('share_whatsapp')}
                        onClick={() => setShareProperty(p)}
                      >
                        <MessageCircle size={14} strokeWidth={1.8} />
                      </button>{' '}
                      <button
                        className="btn btn-sm btn-outline"
                        title={Number(p.image_count || 0) >= MAX_PROPERTY_IMAGES ? t('prop.images-limit') : t('prop.add-image')}
                        disabled={uploading || Number(p.image_count || 0) >= MAX_PROPERTY_IMAGES}
                        onClick={() => { setImgTarget(p.id); fileRef.current?.click(); }}
                      >
                        <ImagePlus size={14} strokeWidth={1.8} />
                      </button>{' '}
                      <button className="btn btn-sm btn-outline" title={t('edit')} onClick={() => navigate(`/properties/${p.id}`)}><Pencil size={14} strokeWidth={1.8} /></button>{' '}
                      <button className="btn btn-sm btn-outline" title={t('duplicate_action')} onClick={() => handleDuplicate(p.id)}><Copy size={14} strokeWidth={1.8} /></button>{' '}
                      <button className="btn btn-sm btn-danger" title={t('delete')} onClick={() => handleDelete(p.id)}><Trash2 size={14} strokeWidth={1.8} /></button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}

      <PaginationBar total={total} page={page} onPage={setPage} pageSize={PAGE_SIZE} label={t('prop.label')} />

      <input ref={fileRef} type="file" accept="image/*" style={{ display: 'none' }} onChange={handleImageSelected} />

      {gallery && (
        <GalleryModal
          propertyId={gallery.id}
          name={gallery.name}
          startIndex={gallery.index || 0}
          onClose={() => setGallery(null)}
        />
      )}
      {shareProperty && (
        <SendOfferModal property={shareProperty} onClose={() => setShareProperty(null)} />
      )}
      </div>
    </div>
  );
}
