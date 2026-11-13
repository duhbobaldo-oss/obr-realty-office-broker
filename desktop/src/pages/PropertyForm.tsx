import { countryPlatforms, getCountry } from '../domain/countryProfile';
import React, { useEffect, useRef, useState } from 'react';
import { useNavigate, useParams, useSearchParams } from 'react-router-dom';
import { api } from '../api';
import { parseQueryPoint } from '../map/style';
import { Building2, ImagePlus, Trash2, Pencil, Camera, Link2, Phone, Megaphone, History } from 'lucide-react';
import { PROPERTY_PURPOSES, PROPERTY_SOURCE_TYPES, PROPERTY_STATUSES } from '../domain/labels';
import { MAX_PROPERTY_IMAGES, LISTING_CARD_IMAGE, fileToPropertyImage, uploadPropertyImage } from '../propertyImages';
import PaginationBar, { usePagedItems } from '../components/PaginationBar';
import { TABLE_PAGE_SIZE } from '../pagination';
import { noAutofill } from '../noAutofill';
import { useLocale } from '../hooks/useLocale';
import { currencySymbol, t as tLocale } from '../domain/locale';

const EMPTY_FORM = {
  name: '',
  city: '',
  district: '',
  location: '',
  property_type: '',
  purpose: '',
  usage_type: '',
  area: '',
  price: '',
  price_per_meter: '',
  frontage: '',
  street_width: '',
  streets_count: '',
  latitude: '',
  longitude: '',
  description: '',
  source_type: '',
  owner_client_id: '',
  owner_name: '',
  owner_phone: '',
  owner_notes: '',
  listing_expiry_date: '',
  commission_agreement: '',
  fal_ad_license: '',
  internal_price: '',
  internal_notes: '',
  ad_number: '',
  ad_link: '',
  marketing_platform: '',
  marketing_link: '',
  published_date: '',
  status: ''
};

export default function PropertyForm() {
  const { id } = useParams();
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const { t, tf } = useLocale();
  const [form, setForm] = useState(EMPTY_FORM);
  const [recordCountry, setRecordCountry] = useState(getCountry());
  const [recordCurrency, setRecordCurrency] = useState(currencySymbol());
  const [contacts, setContacts] = useState<any[]>([]);
  const [updates, setUpdates] = useState<any[]>([]);
  const [images, setImages] = useState<{ id?: string; url: string }[]>([]);
  const [removedImageIds, setRemovedImageIds] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const imageInputRef = useRef<HTMLInputElement>(null);
  const isEdit = Boolean(id);
  const suggestedPricePerMeter = Number(form.area) > 0 && Number(form.price) >= 0 && form.price !== ''
    ? Number(form.price) / Number(form.area)
    : null;
  const pagedUpdates = usePagedItems(updates, TABLE_PAGE_SIZE, id);

  useEffect(() => {
    loadContacts();
    if (id) {
      loadProperty();
      return;
    }
    const point = parseQueryPoint(searchParams);
    setForm(point
      ? { ...EMPTY_FORM, latitude: String(point.lat), longitude: String(point.lng) }
      : EMPTY_FORM);
    setUpdates([]);
    setImages([]);
    setRemovedImageIds([]);
    setError('');
  }, [id, searchParams]);

  const loadContacts = async () => {
    try {
      const list = await api('/api/clients/options');
      setContacts(Array.isArray(list) ? list : []);
    } catch (err: any) {
      setError(err.message || tLocale('prop.form.contacts-fail'));
    }
  };

  const loadProperty = async () => {
    setLoading(true);
    setError('');
    try {
      const data = await api(`/api/properties/${id}`);
      setRecordCountry(data.country);
      setRecordCurrency(data.currency);
      setForm({
        name: data.name || '',
        city: data.city || '',
        district: data.district || '',
        location: data.location || '',
        property_type: data.property_type || '',
        purpose: data.purpose || '',
        usage_type: data.usage_type || '',
        area: data.area ?? '',
        price: data.price ?? '',
        price_per_meter: data.price_per_meter ?? '',
        frontage: data.frontage || '',
        street_width: data.street_width ?? '',
        streets_count: data.streets_count ?? '',
        latitude: data.latitude ?? '',
        longitude: data.longitude ?? '',
        description: data.description || '',
        source_type: data.source_type || '',
        owner_client_id: data.owner_client_id || '',
        owner_name: data.owner_name || '',
        owner_phone: data.owner_phone || '',
        owner_notes: data.owner_notes || '',
        listing_expiry_date: data.listing_expiry_date || '',
        commission_agreement: data.commission_agreement || '',
        fal_ad_license: data.fal_ad_license || '',
        internal_price: data.internal_price ?? '',
        internal_notes: data.internal_notes || '',
        ad_number: data.ad_number || '',
        ad_link: data.ad_link || '',
        marketing_platform: data.marketing_platform || '',
        marketing_link: data.marketing_link || '',
        published_date: data.published_date || '',
        status: data.status || 'available'
      });
      setUpdates(data.updates || []);
      const loaded = Array.isArray(data.images)
        ? data.images
            .map((img: any) => ({ id: String(img.id || ''), url: String(img.image_url || '') }))
            .filter((img: { id: string; url: string }) => img.url)
        : [];
      setImages(loaded);
      setRemovedImageIds([]);
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>) => {
    const { name, value } = e.target;
    if (name === 'owner_client_id') {
      const contact = contacts.find((c) => c.id === value);
      setForm({
        ...form,
        owner_client_id: value,
        owner_name: contact?.name || (value ? form.owner_name : ''),
        owner_phone: contact?.phone || (value ? form.owner_phone : '')
      });
      return;
    }
    setForm({ ...form, [name]: value });
  };

  const addImageFiles = async (files: FileList | null) => {
    if (!files?.length) return;
    const room = MAX_PROPERTY_IMAGES - images.length;
    if (room <= 0) {
      setError(tLocale('prop.max-images'));
      return;
    }
    setError('');
    try {
      const picked = Array.from(files).slice(0, room);
      const next: { url: string }[] = [];
      for (const file of picked) {
        next.push({ url: await fileToPropertyImage(file) });
      }
      setImages((prev) => [...prev, ...next]);
    } catch (err: any) {
      setError(err.message || tLocale('prop.form.image-read-fail'));
    } finally {
      if (imageInputRef.current) imageInputRef.current.value = '';
    }
  };

  const removeFormImage = (index: number) => {
    const img = images[index];
    if (img?.id) setRemovedImageIds((ids) => [...ids, img.id as string]);
    setImages((prev) => prev.filter((_, i) => i !== index));
  };

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setError('');
    setLoading(true);
    try {
      const fromDom: Record<string, string> = {};
      new FormData(e.currentTarget).forEach((value, key) => {
        if (typeof value === 'string') fromDom[key] = value;
      });
      const body = { ...form, ...fromDom };
      let propertyId = id || '';
      if (id) {
        await api(`/api/properties/${id}`, { method: 'PUT', body });
      } else {
        const created = await api('/api/properties', { method: 'POST', body });
        propertyId = created?.id || '';
        if (!propertyId) throw new Error(tLocale('prop.form.saved-no-id'));
      }
      for (const imageId of removedImageIds) {
        await api(`/api/properties/${propertyId}/images/${imageId}`, { method: 'DELETE' });
      }
      const pending = images.filter((img) => !img.id);
      for (const img of pending) {
        await uploadPropertyImage(propertyId, img.url, false);
      }
      navigate('/properties');
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  const addUpdate = async () => {
    const title = prompt(t('prop.form.update-title-ph'));
    if (!title) return;
    const description = prompt(t('prop.form.update-desc-ph'));
    try {
      await api(`/api/properties/${id}/updates`, {
        method: 'POST',
        body: { title, description: description || '' }
      });
      loadProperty();
    } catch (err: any) {
      setError(err.message);
    }
  };

  return (
    <div>
      <div className="card-header">
        <div className="card-title"><Pencil size={17} strokeWidth={1.8} style={{ verticalAlign: '-3px' }} /> {isEdit ? t('prop.form.edit') : t('prop.form.new')}</div>
        <button className="btn btn-outline" onClick={() => navigate('/properties')}>{t('go_back')}</button>
      </div>

      {error && <div className="error-alert">{error}</div>}

      <div className="content-card">
        <form {...noAutofill} onSubmit={handleSubmit}>
          <h3><Building2 size={16} strokeWidth={1.8} style={{ verticalAlign: '-2px' }} /> {t('prop.form.section-basic')}</h3>
          <div className="form-grid">
            <div className="form-group full-row">
              <label htmlFor="prop-name">{t('prop.form.name')}</label>
              <input id="prop-name" className="form-input" autoComplete="off" name="name" value={form.name} onChange={handleChange} required />
            </div>
            <div className="form-group">
              <label htmlFor="prop-city">{t('city')}</label>
              <input id="prop-city" className="form-input" autoComplete="off" name="city" value={form.city} onChange={handleChange} />
            </div>
            <div className="form-group">
              <label htmlFor="prop-district">{t('district')}</label>
              <input id="prop-district" className="form-input" autoComplete="off" name="district" value={form.district} onChange={handleChange} />
            </div>
            <div className="form-group">
              <label htmlFor="prop-type">{t('prop.form.type')}</label>
              <select id="prop-type" className="form-input" autoComplete="off" name="property_type" value={form.property_type} onChange={handleChange}>
                <option value="">{t('choose_type')}</option>
                <option value="فيلا">{t('ptype.villa')}</option>
                <option value="شقة">{t('ptype.apartment')}</option>
                <option value="أرض">{t('ptype.land')}</option>
                <option value="مكتب">{t('ptype.office')}</option>
                <option value="محل تجاري">{t('ptype.shop')}</option>
                <option value="مستودع">{t('ptype.warehouse')}</option>
                <option value="عمارة">{t('ptype.building')}</option>
                <option value="استراحة">{t('ptype.rest')}</option>
                <option value="مزرعة">{t('ptype.farm')}</option>
                <option value="أخرى">{t('ptype.other')}</option>
              </select>
            </div>
            <div className="form-group">
              <label htmlFor="prop-purpose">{t('purpose')}</label>
              <select id="prop-purpose" className="form-input" autoComplete="off" name="purpose" value={form.purpose} onChange={handleChange}>
                <option value="">—</option>
                {PROPERTY_PURPOSES.map((p) => <option key={p.value} value={p.value}>{p.label}</option>)}
              </select>
            </div>
            <div className="form-group">
              <label htmlFor="prop-usage">{t('prop.form.usage')}</label>
              <input id="prop-usage" className="form-input" autoComplete="off" name="usage_type" value={form.usage_type} onChange={handleChange} />
            </div>
            <div className="form-group">
              <label htmlFor="prop-area">{t('area_sqm')}</label>
              <input id="prop-area" className="form-input num" autoComplete="off" name="area" type="number" value={form.area} onChange={handleChange} />
            </div>
            <div className="form-group">
              <label htmlFor="prop-price">{tf('prop.form.price-cur', { cur: recordCurrency })}</label>
              <input id="prop-price" className="form-input num" autoComplete="off" name="price" type="number" value={form.price} onChange={handleChange} />
            </div>
            <div className="form-group">
              <label htmlFor="prop-price-per-meter">سعر المتر ({recordCurrency}/م²)</label>
              <input id="prop-price-per-meter" className="form-input num" autoComplete="off" name="price_per_meter" type="number" min="0" step="0.01" value={form.price_per_meter} onChange={handleChange} />
              {suggestedPricePerMeter != null && (
                <small className="muted">اقتراح محسوب فقط: {suggestedPricePerMeter.toLocaleString('ar-SA', { maximumFractionDigits: 2 })} {recordCurrency}/م² — لا يُحفظ تلقائيًا</small>
              )}
            </div>
            <div className="form-group">
              <label htmlFor="prop-front">{t('prop.form.frontage')}</label>
              <input id="prop-front" className="form-input" autoComplete="off" name="frontage" value={form.frontage} onChange={handleChange} />
            </div>
            <div className="form-group">
              <label htmlFor="prop-sw">{t('prop.form.street-w')}</label>
              <input id="prop-sw" className="form-input num" autoComplete="off" name="street_width" type="number" value={form.street_width} onChange={handleChange} />
            </div>
            <div className="form-group">
              <label htmlFor="prop-sc">{t('prop.form.streets-n')}</label>
              <input id="prop-sc" className="form-input num" autoComplete="off" name="streets_count" type="number" value={form.streets_count} onChange={handleChange} />
            </div>
            <div className="form-group">
              <label htmlFor="prop-lat">{t('prop.form.lat')}</label>
              <input id="prop-lat" className="form-input num" autoComplete="off" name="latitude" value={form.latitude} onChange={handleChange} />
            </div>
            <div className="form-group">
              <label htmlFor="prop-lon">{t('prop.form.lng')}</label>
              <input id="prop-lon" className="form-input num" autoComplete="off" name="longitude" value={form.longitude} onChange={handleChange} />
            </div>
            <div className="form-group full-row">
              <button
                type="button"
                className="btn btn-outline"
                onClick={() => {
                  if (isEdit && id) {
                    navigate(`/maps?property=${id}`);
                    return;
                  }
                  const lat = String(form.latitude || '').trim();
                  const lng = String(form.longitude || '').trim();
                  const q = lat && lng ? `?lat=${encodeURIComponent(lat)}&lng=${encodeURIComponent(lng)}` : '';
                  navigate(`/maps${q}`);
                }}
              >
                {t('prop.form.map-pick')}
              </button>
            </div>
            <div className="form-group">
              <label htmlFor="prop-status">{t('prop.form.status')}</label>
              <select id="prop-status" className="form-input" autoComplete="off" name="status" value={form.status} onChange={handleChange}>
                <option value="">—</option>
                {PROPERTY_STATUSES.map((s) => <option key={s.value} value={s.value}>{s.label}</option>)}
              </select>
            </div>
            <div className="form-group">
              <label htmlFor="prop-expiry">{t('prop.form.expiry')}</label>
              <input id="prop-expiry" className="form-input" autoComplete="off" name="listing_expiry_date" type="date" value={form.listing_expiry_date} onChange={handleChange} />
            </div>
            <div className="form-group">
              <label htmlFor="prop-fal">{t('prop.form.fal')}</label>
              <input id="prop-fal" className="form-input" autoComplete="off" name="fal_ad_license" value={form.fal_ad_license} onChange={handleChange} />
            </div>
            <div className="form-group full-row">
              <label htmlFor="prop-desc">{t('prop.form.desc')}</label>
              <textarea id="prop-desc" className="form-input" autoComplete="off" name="description" rows={3} value={form.description} onChange={handleChange} />
            </div>
          </div>

          <hr style={{ margin: '24px 0', border: 'none', borderTop: '1px solid var(--gray-border)' }} />

          <h3><Camera size={16} strokeWidth={1.8} style={{ verticalAlign: '-2px' }} /> {t('prop.form.photos')}</h3>
          <p className="text-secondary" style={{ marginBottom: 12 }}>
            {tf('prop.form.photos-hint', {
              aspect: `${LISTING_CARD_IMAGE.aspectW}:${LISTING_CARD_IMAGE.aspectH}`,
              px: `${LISTING_CARD_IMAGE.widthPx}×${LISTING_CARD_IMAGE.heightPx}`
            })}
          </p>
          <input
            ref={imageInputRef}
            type="file"
            accept="image/*"
            multiple
            hidden
            onChange={(e) => addImageFiles(e.target.files)}
          />
          <div className="prop-image-slots">
            {images.map((img, index) => (
              <div key={img.id || `pending-${index}`} className="prop-image-slot is-filled">
                <img src={img.url} alt={tf('prop.image-n', { n: index + 1 })} />
                <span className="prop-image-num">{index + 1}</span>
                <button type="button" className="prop-image-remove" title={t('prop.form.remove-photo')} onClick={() => removeFormImage(index)}>
                  <Trash2 size={14} />
                </button>
              </div>
            ))}
            {images.length < MAX_PROPERTY_IMAGES && (
              <button
                type="button"
                className="prop-image-slot is-add"
                onClick={() => imageInputRef.current?.click()}
              >
                <ImagePlus size={22} strokeWidth={1.7} />
                <span>{tf('prop.form.add-photo-n', { n: images.length + 1, max: MAX_PROPERTY_IMAGES })}</span>
              </button>
            )}
          </div>

          <hr style={{ margin: '24px 0', border: 'none', borderTop: '1px solid var(--gray-border)' }} />

          <h3><Link2 size={16} strokeWidth={1.8} style={{ verticalAlign: '-2px' }} /> {t('prop.form.section-source')}</h3>
          <p className="text-secondary" style={{ marginBottom: 12 }}>
            {t('prop.form.source-hint')}
          </p>
          <div className="form-grid">
            <div className="form-group">
              <label htmlFor="prop-source">{t('prop.form.source-type')}</label>
              <select id="prop-source" className="form-input" autoComplete="off" name="source_type" value={form.source_type} onChange={handleChange}>
                <option value="">—</option>
                {PROPERTY_SOURCE_TYPES.map((s) => <option key={s.value} value={s.value}>{s.label}</option>)}
              </select>
            </div>
            <div className="form-group">
              <label htmlFor="prop-owner">{t('prop.form.owner-contact')}</label>
              <select id="prop-owner" className="form-input" autoComplete="off" name="owner_client_id" value={form.owner_client_id} onChange={handleChange}>
                <option value="">{t('unlink_contact')}</option>
                {contacts.map((c) => (
                  <option key={c.id} value={c.id}>{c.name}{c.phone ? ` — ${c.phone}` : ''}</option>
                ))}
              </select>
            </div>
            <div className="form-group">
              <label htmlFor="prop-oname">{t('prop.form.owner-name')}</label>
              <input id="prop-oname" className="form-input" autoComplete="off" name="owner_name" onChange={handleChange} value={form.owner_name} />
            </div>
            <div className="form-group">
              <label htmlFor="prop-ophone">{t('prop.form.owner-phone')}</label>
              <input id="prop-ophone" className="form-input" autoComplete="off" name="owner_phone" onChange={handleChange} value={form.owner_phone} />
            </div>
            <div className="form-group full-row">
              <label htmlFor="prop-onotes">{t('prop.form.owner-notes')}</label>
              <input id="prop-onotes" className="form-input" autoComplete="off" name="owner_notes" onChange={handleChange} value={form.owner_notes} />
            </div>
          </div>

          <hr style={{ margin: '24px 0', border: 'none', borderTop: '1px solid var(--gray-border)' }} />

          <h3><Phone size={16} strokeWidth={1.8} style={{ verticalAlign: '-2px' }} /> {t('prop.form.section-comm')}</h3>
          <div className="form-grid">
            <div className="form-group">
              <label htmlFor="prop-comm">{t('prop.form.comm')}</label>
              <input id="prop-comm" className="form-input" autoComplete="off" name="commission_agreement" value={form.commission_agreement} onChange={handleChange} />
            </div>
            <div className="form-group">
              <label htmlFor="prop-iprice">{t('prop.form.internal-price')}</label>
              <input id="prop-iprice" className="form-input num" autoComplete="off" name="internal_price" type="number" value={form.internal_price} onChange={handleChange} />
            </div>
            <div className="form-group full-row">
              <label htmlFor="prop-inotes">{t('prop.form.internal-notes')}</label>
              <textarea id="prop-inotes" className="form-input" autoComplete="off" name="internal_notes" rows={2} value={form.internal_notes} onChange={handleChange} />
            </div>
          </div>

          <hr style={{ margin: '24px 0', border: 'none', borderTop: '1px solid var(--gray-border)' }} />

          <h3><Megaphone size={16} strokeWidth={1.8} style={{ verticalAlign: '-2px' }} /> {t('prop.form.section-mkt')}</h3>
          <div className="form-grid">
            <div className="form-group"><label htmlFor="prop-plat">{t('prop.form.mkt-platform')}</label><select id="prop-plat" className="form-input" name="marketing_platform" onChange={handleChange} value={form.marketing_platform}><option value="">—</option>{form.marketing_platform && !countryPlatforms(recordCountry).some(p => p.id === form.marketing_platform) && <option value={form.marketing_platform}>{form.marketing_platform}</option>}{countryPlatforms(recordCountry).map(p => <option key={p.id} value={p.id}>{p.name}</option>)}</select></div>
            <div className="form-group"><label htmlFor="prop-mlink">{t('prop.form.mkt-link')}</label><input id="prop-mlink" className="form-input" autoComplete="off" name="marketing_link" onChange={handleChange} value={form.marketing_link} /></div>
            <div className="form-group"><label htmlFor="prop-ad">{t('prop.form.ad-number')}</label><input id="prop-ad" className="form-input" autoComplete="off" name="ad_number" onChange={handleChange} value={form.ad_number} /></div>
            <div className="form-group"><label htmlFor="prop-pub">{t('prop.form.published')}</label><input id="prop-pub" className="form-input" autoComplete="off" name="published_date" type="date" onChange={handleChange} value={form.published_date} /></div>
          </div>

          <div style={{ marginTop: 24, display: 'flex', gap: 12 }}>
            <button className="btn btn-primary" disabled={loading}>{loading ? t('saving') : t('prop.form.save')}</button>
            <button type="button" className="btn btn-outline" onClick={() => navigate('/properties')}>{t('cancel')}</button>
          </div>
        </form>
      </div>

      {isEdit && (
        <div className="content-card" style={{ marginTop: 16 }}>
          <div className="card-header">
            <div className="card-title"><History size={17} strokeWidth={1.8} style={{ verticalAlign: '-3px' }} /> {t('prop.form.updates')}</div>
            <button className="btn btn-primary btn-sm" onClick={addUpdate}>{t('prop.form.add-update')}</button>
          </div>
          {updates.length === 0 ? (
            <div className="empty-state"><p>{t('prop.form.no-updates')}</p></div>
          ) : (
            <>
            <div className="timeline">
              {pagedUpdates.slice.map((u) => (
                <div key={u.id} className="timeline-item">
                  <div className="time">{u.created_at}</div>
                  <div className="title">{u.title}</div>
                  {u.description && <div className="desc">{u.description}</div>}
                </div>
              ))}
            </div>
            <PaginationBar total={pagedUpdates.total} page={pagedUpdates.page} onPage={pagedUpdates.setPage} label={t('update_label')} />
            </>
          )}
        </div>
      )}
    </div>
  );
}
