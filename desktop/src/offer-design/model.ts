import { t as systemT } from '../domain/locale';
/** نموذج مستند تصميم العروض v2 — عناصر + صفحات + ربط بيانات */

import type { PageSizeId } from './sizes';
import { getPageSize } from './sizes';

export const DOC_VERSION = 2 as const;

export type ElementType =
  | 'text'
  | 'heading'
  | 'property_field'
  | 'office_field'
  | 'image'
  | 'shape'
  | 'divider'
  | 'qr'
  | 'specs_table';

export type BindingKey =
  | 'property.title'
  | 'property.price'
  | 'property.pricePerMeter'
  | 'property.area'
  | 'property.city'
  | 'property.district'
  | 'property.propertyType'
  | 'property.purpose'
  | 'property.frontage'
  | 'property.streetWidth'
  | 'property.description'
  | 'property.adNumber'
  | 'property.adLink'
  | 'property.falAdLicense'
  | 'property.image'
  | 'office.name'
  | 'office.phone'
  | 'office.whatsapp'
  | 'office.falLicense'
  | 'office.logo';

export interface ElementStyle {
  fontSize?: number;
  fontWeight?: number | string;
  align?: 'right' | 'center' | 'left';
  color?: string;
  background?: string;
  borderColor?: string;
  borderWidth?: number;
  radius?: number;
  opacity?: number;
  hideIfEmpty?: boolean;
  objectFit?: 'cover' | 'contain';
}

export interface DocElement {
  id: string;
  type: ElementType;
  x: number; // mm from page right (RTL visual) — stored as mm from left for stability; renderer flips for RTL display
  y: number; // mm from top
  width: number; // mm
  height: number; // mm
  rotation: number;
  zIndex: number;
  visible: boolean;
  binding?: BindingKey | null;
  content?: string;
  /** Explicit provenance for generated text; editing content clears this key. */
  systemTextKey?: string;
  style: ElementStyle;
}

export interface DocPage {
  id: string;
  name: string;
  systemName?: boolean;
  order: number;
  elements: DocElement[];
}

export interface OfferDocument {
  version: typeof DOC_VERSION;
  title: string;
  titleKey?: string;
  propertyId: string;
  pageSize: PageSizeId;
  templateOrigin?: string;
  theme: {
    primary: string;
    accent: string;
    surface: string;
    ink: string;
    muted: string;
  };
  pages: DocPage[];
  updatedAt: string;
}

/** كتالوج الربط — التسمية العربية فقط في الواجهة */
export const BINDING_CATALOG: { key: BindingKey; label: string; group: 'property' | 'office'; kind: 'text' | 'image' }[] = [
  { key: 'property.title', get label() { return systemT('design.ui.125'); }, group: 'property', kind: 'text' },
  { key: 'property.price', get label() { return systemT('design.ui.126'); }, group: 'property', kind: 'text' },
  { key: 'property.pricePerMeter', get label() { return systemT('design.ui.127'); }, group: 'property', kind: 'text' },
  { key: 'property.area', get label() { return systemT('design.ui.128'); }, group: 'property', kind: 'text' },
  { key: 'property.city', get label() { return systemT('design.ui.129'); }, group: 'property', kind: 'text' },
  { key: 'property.district', get label() { return systemT('design.ui.130'); }, group: 'property', kind: 'text' },
  { key: 'property.propertyType', get label() { return systemT('design.ui.131'); }, group: 'property', kind: 'text' },
  { key: 'property.purpose', get label() { return systemT('design.ui.132'); }, group: 'property', kind: 'text' },
  { key: 'property.frontage', get label() { return systemT('design.ui.133'); }, group: 'property', kind: 'text' },
  { key: 'property.streetWidth', get label() { return systemT('design.ui.134'); }, group: 'property', kind: 'text' },
  { key: 'property.description', get label() { return systemT('design.ui.135'); }, group: 'property', kind: 'text' },
  { key: 'property.adNumber', get label() { return systemT('design.ui.136'); }, group: 'property', kind: 'text' },
  { key: 'property.adLink', get label() { return systemT('design.ui.137'); }, group: 'property', kind: 'text' },
  { key: 'property.falAdLicense', get label() { return systemT('design.ui.138'); }, group: 'property', kind: 'text' },
  { key: 'property.image', get label() { return systemT('design.ui.139'); }, group: 'property', kind: 'image' },
  { key: 'office.name', get label() { return systemT('design.ui.140'); }, group: 'office', kind: 'text' },
  { key: 'office.phone', get label() { return systemT('design.ui.141'); }, group: 'office', kind: 'text' },
  { key: 'office.whatsapp', get label() { return systemT('design.ui.142'); }, group: 'office', kind: 'text' },
  { key: 'office.falLicense', get label() { return systemT('design.ui.143'); }, group: 'office', kind: 'text' },
  { key: 'office.logo', get label() { return systemT('design.ui.144'); }, group: 'office', kind: 'image' }
];

export function bindingLabel(key?: BindingKey | null): string {
  if (!key) return '—';
  return BINDING_CATALOG.find((b) => b.key === key)?.label || systemT('design.ui.145');
}

export function newId(prefix = 'el'): string {
  return `${prefix}_${Math.random().toString(36).slice(2, 10)}_${Date.now().toString(36)}`;
}

export function defaultTheme() {
  return {
    primary: '#23365f',
    accent: '#d97706',
    surface: '#ffffff',
    ink: '#23365f',
    muted: '#6f7380'
  };
}

export function emptyDocument(partial?: Partial<OfferDocument>): OfferDocument {
  const pageId = newId('page');
  return {
    version: DOC_VERSION,
    titleKey: partial?.titleKey,
    title: partial?.title || systemT('design.ui.146'),
    propertyId: partial?.propertyId || '',
    pageSize: partial?.pageSize || 'a4-portrait',
    templateOrigin: partial?.templateOrigin,
    theme: partial?.theme || defaultTheme(),
    pages: partial?.pages || [
      {
        id: pageId,
        name: systemT('design.ui.147'),
        order: 0,
        elements: []
      }
    ],
    updatedAt: new Date().toISOString()
  };
}

export function createElement(
  type: ElementType,
  overrides: Partial<DocElement> = {}
): DocElement {
  const defaults: Record<ElementType, Partial<DocElement>> = {
    text: { width: 80, height: 12, content: '', systemTextKey: 'design.text', style: { fontSize: 11, align: 'right', color: '#23365f' } },
    heading: { width: 120, height: 16, content: '', systemTextKey: 'design.heading', style: { fontSize: 18, fontWeight: 700, align: 'right', color: '#23365f' } },
    property_field: { width: 110, height: 14, binding: 'property.price', style: { fontSize: 13, align: 'right', hideIfEmpty: false } },
    office_field: { width: 110, height: 14, binding: 'office.name', style: { fontSize: 12, align: 'right', hideIfEmpty: false } },
    image: { width: 110, height: 82, binding: 'property.image', style: { objectFit: 'contain', background: '#eee5d8', hideIfEmpty: false } },
    shape: { width: 40, height: 30, style: { background: '#e9e0d2', radius: 2 } },
    divider: { width: 160, height: 1, style: { background: '#d97706' } },
    qr: {
      width: 28,
      height: 28,
      binding: 'property.adLink',
      content: '',
      style: { background: '#fff', borderColor: '#23365f', borderWidth: 1, hideIfEmpty: true }
    },
    specs_table: {
      width: 90,
      height: 55,
      style: { fontSize: 10, background: '#f9f6f1', borderColor: '#e9e0d2', borderWidth: 1, radius: 4 }
    }
  };
  const d = defaults[type] || {};
  const { style: overrideStyle, ...restOverrides } = overrides;
  return {
    id: newId('el'),
    type,
    x: 20,
    y: 20,
    width: 60,
    height: 14,
    rotation: 0,
    zIndex: 1,
    visible: true,
    binding: null,
    content: '',
    ...d,
    ...restOverrides,
    systemTextKey: overrides.binding || overrides.content !== undefined ? overrides.systemTextKey : (overrides.systemTextKey || d.systemTextKey),
    style: { ...(d.style || {}), ...(overrideStyle || {}) }
  };
}

/** ترقية محتوى v1 (لقطة حقول) إلى مستند عناصر v2 */
export function migrateToV2(raw: any): OfferDocument {
  if (!raw || typeof raw !== 'object') {
    return emptyDocument();
  }
  if (raw.version === 2 && Array.isArray(raw.pages)) {
    return {
      ...emptyDocument(),
      ...raw,
      version: DOC_VERSION,
      pages: raw.pages.map((p: any, i: number) => ({
        id: p.id || newId('page'),
        name: p.name || `صفحة ${i + 1}`,
        order: typeof p.order === 'number' ? p.order : i,
        elements: Array.isArray(p.elements) ? p.elements : []
      }))
    };
  }

  // v1 OfferDesignDoc
  if (raw.version === 1 && raw.fields) {
    const f = raw.fields || {};
    const theme = raw.theme || defaultTheme();
    const pageSize = (raw.pageSize || 'a4-portrait') as PageSizeId;
    const size = getPageSize(pageSize);
    const pageId = newId('page');
    const elements: DocElement[] = [];
    let z = 1;
    const push = (el: DocElement) => {
      el.zIndex = z++;
      elements.push(el);
    };

    push(createElement('heading', {
      x: 12, y: 12, width: size.widthMm - 24, height: 18,
      content: f.title || '',
      binding: 'property.title',
      style: { fontSize: 20, fontWeight: 700, color: theme.ink, align: 'right', hideIfEmpty: false }
    }));
    push(createElement('office_field', {
      x: 12, y: 32, width: size.widthMm - 24, height: 10,
      binding: 'office.name',
      content: f.officeName || '',
      style: { fontSize: 11, color: theme.muted, align: 'right' }
    }));
    push(createElement('image', {
      x: 12, y: 48, width: size.widthMm * 0.55, height: size.heightMm * 0.42,
      binding: 'property.image',
      content: f.imageUrl || '',
      style: { objectFit: 'contain', background: '#eee5d8', hideIfEmpty: false }
    }));
    push(createElement('property_field', {
      x: size.widthMm * 0.6, y: 48, width: size.widthMm * 0.35, height: 16,
      binding: 'property.price',
      content: f.price || '',
      style: { fontSize: 16, fontWeight: 700, color: theme.accent, align: 'right' }
    }));
    const specsY = 70;
    const specs: { binding: BindingKey; fallback: string }[] = [
      { binding: 'property.propertyType', fallback: f.propertyType || '' },
      { binding: 'property.city', fallback: f.city || '' },
      { binding: 'property.district', fallback: f.district || '' },
      { binding: 'property.area', fallback: f.area || '' },
      { binding: 'property.pricePerMeter', fallback: f.pricePerMeter || '' }
    ];
    specs.forEach((s, i) => {
      push(createElement('property_field', {
        x: size.widthMm * 0.6,
        y: specsY + i * 14,
        width: size.widthMm * 0.35,
        height: 12,
        binding: s.binding,
        content: s.fallback,
        style: { fontSize: 11, color: theme.ink, align: 'right', hideIfEmpty: true }
      }));
    });
    push(createElement('text', {
      x: 12, y: size.heightMm * 0.72, width: size.widthMm - 24, height: 28,
      content: f.description || '',
      binding: 'property.description',
      style: { fontSize: 10, color: theme.muted, align: 'right', hideIfEmpty: true }
    }));
    push(createElement('office_field', {
      x: 12, y: size.heightMm - 22, width: (size.widthMm - 24) / 3, height: 10,
      binding: 'office.phone',
      content: f.phone || '',
      style: { fontSize: 9, color: theme.muted, align: 'right', hideIfEmpty: true }
    }));
    push(createElement('office_field', {
      x: 12 + (size.widthMm - 24) / 3, y: size.heightMm - 22, width: (size.widthMm - 24) / 3, height: 10,
      binding: 'office.whatsapp',
      content: f.whatsapp || '',
      style: { fontSize: 9, color: theme.muted, align: 'right', hideIfEmpty: true }
    }));
    push(createElement('office_field', {
      x: 12 + (2 * (size.widthMm - 24)) / 3, y: size.heightMm - 22, width: (size.widthMm - 24) / 3, height: 10,
      binding: 'office.falLicense',
      content: f.falLicense || '',
      style: { fontSize: 9, color: theme.muted, align: 'right', hideIfEmpty: true }
    }));

    return {
      version: DOC_VERSION,
      title: f.title || raw.title || systemT('design.ui.24'),
      propertyId: raw.propertyId || '',
      pageSize,
      templateOrigin: raw.templateId,
      theme: {
        primary: theme.primary || defaultTheme().primary,
        accent: theme.accent || defaultTheme().accent,
        surface: theme.surface || defaultTheme().surface,
        ink: theme.ink || defaultTheme().ink,
        muted: theme.muted || defaultTheme().muted
      },
      pages: [{ id: pageId, name: systemT('design.ui.147'), order: 0, elements }],
      updatedAt: raw.updatedAt || new Date().toISOString()
    };
  }

  // محتوى قديم غير معروف
  return emptyDocument({ title: raw.title || systemT('design.ui.148') });
}

export function pageCount(doc: OfferDocument): number {
  return doc.pages?.length || 0;
}

export function sortedPages(doc: OfferDocument): DocPage[] {
  return [...(doc.pages || [])].sort((a, b) => a.order - b.order);
}
