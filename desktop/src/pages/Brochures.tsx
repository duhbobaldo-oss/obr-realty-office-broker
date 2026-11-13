import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { FileText, Pencil, Copy, Trash2, Printer, FolderOpen } from 'lucide-react';
import { api } from '../api';
import { getPageSize, type PageSizeId } from '../offer-design/sizes';
import { templateDisplayName } from '../offer-design/templates-v2';
import { getTemplate, type TemplateId } from '../offer-design/templates';
import { pageCount, migrateToV2 } from '../offer-design/model';
import PaginationBar, { usePagedItems } from '../components/PaginationBar';
import { TABLE_PAGE_SIZE } from '../pagination';
import SectionGuideBanner from '../components/SectionGuide';
import { useLocale } from '../hooks/useLocale';
import { t as tLocale } from '../domain/locale';

type LibraryItem = {
  id: string;
  title?: string | null;
  template_id?: string | null;
  property_id?: string | null;
  property_name?: string | null;
  created_at?: string;
  updated_at?: string;
  content?: any;
  kind: 'design' | 'brochure';
  pageSizeLabel?: string;
  templateLabel?: string;
  pagesLabel?: string;
};

function parseContent(raw: any): any | null {
  if (!raw) return null;
  if (typeof raw === 'object') return raw;
  try {
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

function formatWhen(value: string | null | undefined, lang: string): string {
  if (!value) return '—';
  const d = new Date(value);
  if (Number.isNaN(d.getTime())) return value;
  return d.toLocaleString(lang === 'en' ? 'en-US' : 'ar-SA');
}

function isOfferDesign(content: any): boolean {
  if (!content || typeof content !== 'object') return false;
  if (content.version === 2 && Array.isArray(content.pages)) return true;
  if (content.version === 1 && content.templateId) return true;
  return false;
}

function templateLabelFor(content: any, templateId?: string | null): string {
  const origin = content?.templateOrigin || content?.templateId || templateId || '';
  const named = templateDisplayName(origin);
  if (named !== origin) return named;
  return getTemplate(origin as TemplateId)?.id === origin
    ? getTemplate(origin as TemplateId).name
    : (origin || '—');
}

export default function Brochures() {
  const navigate = useNavigate();
  const { t, tf, lang } = useLocale();
  const [items, setItems] = useState<LibraryItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [message, setMessage] = useState('');
  const [q, setQ] = useState('');

  const load = useCallback(async () => {
    setLoading(true);
    setError('');
    try {
      const [brochData, propsData] = await Promise.all([
        api('/api/brochures'),
        api('/api/properties?limit=500').catch(() => ({ items: [] }))
      ]);
      const propMap = new Map<string, string>();
      for (const p of propsData.items || []) propMap.set(p.id, p.name);

      const mapped: LibraryItem[] = (brochData || []).map((b: any) => {
        const content = parseContent(b.content);
        const design = isOfferDesign(content);
        const v2 = design ? migrateToV2(content) : null;
        const pageSize = (v2?.pageSize || content?.pageSize || 'a4-portrait') as PageSizeId;
        return {
          id: b.id,
          title: b.title || v2?.title || content?.fields?.title || tLocale('untitled'),
          template_id: b.template_id,
          property_id: b.property_id,
          property_name: b.property_name || (b.property_id ? (propMap.get(b.property_id) || null) : null),
          created_at: b.created_at,
          updated_at: b.updated_at || b.created_at,
          content: v2 || content,
          kind: design ? 'design' : 'brochure',
          pageSizeLabel: getPageSize(pageSize).label,
          templateLabel: design ? templateLabelFor(content, b.template_id) : (b.template_id || tLocale('bro.kind.brochure')),
          pagesLabel: v2 ? String(pageCount(v2)) : '—'
        };
      });
      mapped.sort((a, b) => String(b.updated_at || '').localeCompare(String(a.updated_at || '')));
      setItems(mapped);
    } catch (err: any) {
      setError(err?.message || tLocale('bro.load-fail'));
      setItems([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  const filtered = useMemo(() => {
    const term = q.trim();
    if (!term) return items;
    return items.filter((it) =>
      [it.title, it.property_name, it.templateLabel, it.pageSizeLabel, it.kind === 'design' ? t('bro.kind.design-short') : t('bro.kind.brochure')]
        .filter(Boolean)
        .some((v) => String(v).includes(term))
    );
  }, [items, q, lang, t]);

  const paged = usePagedItems(filtered, TABLE_PAGE_SIZE, q);

  const openItem = (item: LibraryItem) => {
    if (item.kind === 'design') {
      navigate(`/offer-design?id=${encodeURIComponent(item.id)}`);
      return;
    }
    navigate('/offer-design');
  };

  const duplicateItem = async (item: LibraryItem) => {
    try {
      const content = item.content;
      await api('/api/brochures', {
        method: 'POST',
        body: {
          property_id: item.property_id || undefined,
          template_id: item.template_id || content?.templateOrigin || content?.templateId || 'royal-sheet',
          title: tf('bro.copy-suffix', { title: item.title ?? t('listing-tag') }),
          content: content || undefined
        }
      });
      setMessage(t('bro.copied'));
      await load();
    } catch (err: any) {
      setError(err?.message || t('bro.copy-fail'));
    }
  };

  const deleteItem = async (item: LibraryItem) => {
    if (!confirm(tf('bro.delete-confirm', { title: item.title ?? t('untitled') }))) return;
    try {
      await api(`/api/brochures/${item.id}`, { method: 'DELETE' });
      setMessage(t('bro.deleted'));
      await load();
    } catch (err: any) {
      setError(err?.message || t('delete_failed'));
    }
  };

  if (loading) {
    return <div className="content-card">{t('bro.loading')}</div>;
  }

  return (
    <div>
      <div className="card-header">
        <div className="card-title"><FolderOpen size={17} strokeWidth={1.8} /> {t('bro.title')} <SectionGuideBanner id="brochures" /></div>
        <button type="button" className="btn btn-primary" onClick={() => navigate('/offer-design')}>
          {t('bro.new-design')}
        </button>
      </div>

      <p className="text-secondary" style={{ marginBottom: 14 }}>
        {t('bro.hint')}
      </p>

      {message && <div className="success-alert">{message}</div>}
      {error && <div className="error-alert">{error}</div>}

      <div className="content-card obr-panel">
        <div className="toolbar">
          <input
            className="search-input"
            placeholder={t('bro.search-ph')}
            value={q}
            onChange={(e) => setQ(e.target.value)}
          />
        </div>

        {filtered.length === 0 ? (
          <div className="empty-state">
            <div className="icon"><FileText size={48} strokeWidth={1.4} /></div>
            <h3>{t('bro.empty')}</h3>
            <p>{q.trim() ? t('bro.no-match') : t('bro.empty-hint')}</p>
          </div>
        ) : (
          <div className="table-container">
            <table className="data-table">
              <thead>
                <tr>
                  <th>{t('bro.col.name')}</th>
                  <th>{t('bro.col.kind')}</th>
                  <th>{t('bro.col.property')}</th>
                  <th>{t('bro.col.pages')}</th>
                  <th>{t('bro.col.template')}</th>
                  <th>{t('bro.col.size')}</th>
                  <th>{t('bro.col.created')}</th>
                  <th>{t('bro.col.updated')}</th>
                  <th>{t('actions')}</th>
                </tr>
              </thead>
              <tbody>
                {paged.slice.map((it) => (
                  <tr key={it.id}>
                    <td>{it.title}</td>
                    <td>{it.kind === 'design' ? t('bro.kind.design') : t('bro.kind.brochure')}</td>
                    <td>{it.property_name || '—'}</td>
                    <td className="num">{it.pagesLabel || '—'}</td>
                    <td>{it.templateLabel || '—'}</td>
                    <td>{it.pageSizeLabel || '—'}</td>
                    <td className="num">{formatWhen(it.created_at, lang)}</td>
                    <td className="num">{formatWhen(it.updated_at, lang)}</td>
                    <td>
                      <button type="button" className="btn btn-sm btn-outline" title={t('open')} onClick={() => openItem(it)}>
                        <Pencil size={14} strokeWidth={1.8} /> {t('open')}
                      </button>{' '}
                      <button type="button" className="btn btn-sm btn-outline" title={t('copy')} onClick={() => duplicateItem(it)}>
                        <Copy size={14} strokeWidth={1.8} />
                      </button>{' '}
                      <button
                        type="button"
                        className="btn btn-sm btn-outline"
                        title={t('print')}
                        onClick={() => navigate(`/offer-design?id=${encodeURIComponent(it.id)}&print=1`)}
                      >
                        <Printer size={14} strokeWidth={1.8} />
                      </button>{' '}
                      <button type="button" className="btn btn-sm btn-danger" title={t('delete')} onClick={() => deleteItem(it)}>
                        <Trash2 size={14} strokeWidth={1.8} />
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
        <PaginationBar total={paged.total} page={paged.page} onPage={paged.setPage} label={t('item_label')} />
      </div>
    </div>
  );
}
