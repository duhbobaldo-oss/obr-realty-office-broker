import { t as systemT } from '../domain/locale';
import { recordOfficeSettings } from '../domain/countryProfile';
import React, { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import {
  Save,
  Undo2,
  Redo2,
  ZoomIn,
  ZoomOut,
  Maximize2,
  Eye,
  Printer,
  FileDown,
  Plus,
  Copy,
  Trash2,
  Type,
  Heading,
  Image as ImageIcon,
  Square,
  Minus,
  QrCode,
  Table2,
  Building2,
  Briefcase,
  PenTool,
  SlidersHorizontal,
  LayoutTemplate,
  PanelRightClose,
  PanelRightOpen,
  PanelLeftClose,
  PanelLeftOpen,
  Focus,
  Minimize2,
  ChevronLeft,
  ChevronRight,
  ChevronDown,
  ChevronUp
} from 'lucide-react';
import { api } from '../api';
import DocumentCanvas from '../offer-design/DocumentCanvas';
import PrintDocument from '../offer-design/PrintDocument';
import PropertiesPanel from '../offer-design/PropertiesPanel';
import SectionGuideBanner from '../components/SectionGuide';
import { designExportFormats, exportDesignDocument } from '../offer-design/exportDesign';
import {
  BINDING_CATALOG,
  bindingLabel,
  createElement,
  emptyDocument,
  migrateToV2,
  newId,
  pageCount,
  sortedPages,
  type BindingKey,
  type DocElement,
  type DocPage,
  type ElementType,
  type OfferDocument
} from '../offer-design/model';
import { DOC_TEMPLATES, getDocTemplate, clonePage } from '../offer-design/templates-v2';
import { PAGE_SIZES, getPageSize, type PageSizeId } from '../offer-design/sizes';
import { createHistory } from '../offer-design/history';
import { placeFillPop } from '../offer-design/placeFillPop';
import { useLocale } from '../hooks/useLocale';
import { applyImageToFrame, imageSlotForPage } from '../offer-design/imageSlots';
import { fileToPropertyImage } from '../propertyImages';
import AppContextMenu, { type AppMenuItem } from '../components/AppContextMenu';

type ZoomMode = 'fit' | '100' | 'custom';
const MM_TO_PX = 96 / 25.4;
const WORKSPACE_PAD = 8;

type DesignerCollapse = { elements: boolean; props: boolean; strip: boolean };

// حالة الطي للجلسة الحالية فقط — لا localStorage، تُنسى عند إعادة تشغيل التطبيق
let sessionCollapse: DesignerCollapse = { elements: false, props: false, strip: false };

type PaletteItem = {
  group: string;
  label: string;
  type: ElementType;
  binding?: BindingKey;
};

const PALETTE: PaletteItem[] = [
  { get group() { return systemT('design.ui.0'); }, get label() { return systemT('design.ui.1'); }, type: 'heading' },
  { get group() { return systemT('design.ui.0'); }, get label() { return systemT('design.ui.2'); }, type: 'text' },
  ...BINDING_CATALOG.filter((b) => b.group === 'property' && b.kind === 'text').map((b) => ({
    get group() { return systemT('design.ui.3'); },
    get label() { return b.label; },
    type: 'property_field' as ElementType,
    binding: b.key
  })),
  ...BINDING_CATALOG.filter((b) => b.group === 'property' && b.kind === 'image').map((b) => ({
    get group() { return systemT('design.ui.4'); },
    get label() { return b.label; },
    type: 'image' as ElementType,
    binding: b.key
  })),
  ...BINDING_CATALOG.filter((b) => b.group === 'office' && b.kind === 'image').map((b) => ({
    get group() { return systemT('design.ui.4'); },
    get label() { return b.label; },
    type: 'image' as ElementType,
    binding: b.key
  })),
  { get group() { return systemT('design.ui.4'); }, get label() { return systemT('design.ui.5'); }, type: 'image' },
  ...BINDING_CATALOG.filter((b) => b.group === 'office' && b.kind === 'text').map((b) => ({
    get group() { return systemT('design.ui.6'); },
    get label() { return b.label; },
    type: 'office_field' as ElementType,
    binding: b.key
  })),
  { get group() { return systemT('design.ui.7'); }, get label() { return systemT('design.ui.8'); }, type: 'shape' },
  { get group() { return systemT('design.ui.7'); }, get label() { return systemT('design.ui.9'); }, type: 'divider' },
  { get group() { return systemT('design.ui.7'); }, get label() { return systemT('design.ui.10'); }, type: 'specs_table' },
  { group: 'QR', get label() { return systemT('design.ui.11'); }, type: 'qr', binding: 'property.adLink' as BindingKey }
];

function deepCloneDoc(doc: OfferDocument): OfferDocument {
  return JSON.parse(JSON.stringify(doc)) as OfferDocument;
}

export default function OfferDesign() {
  const { t, tf, lang } = useLocale();
  const [searchParams, setSearchParams] = useSearchParams();
  const [properties, setProperties] = useState<any[]>([]);
  const [settings, setSettings] = useState<any>(null);
  const savedCountryContext = useRef<any>(null);
  const [property, setProperty] = useState<any | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState('');
  const [error, setError] = useState('');
  const [editingId, setEditingId] = useState<string | null>(null);
  const [previewMode, setPreviewMode] = useState(false);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [activePageId, setActivePageId] = useState<string>('');
  const [zoomMode, setZoomMode] = useState<ZoomMode>('fit');
  const [customZoom, setCustomZoom] = useState(1);
  const [workspaceSize, setWorkspaceSize] = useState({ w: 720, h: 560 });
  const [canUndo, setCanUndo] = useState(false);
  const [canRedo, setCanRedo] = useState(false);
  const [dragPageId, setDragPageId] = useState<string | null>(null);
  const [textEditId, setTextEditId] = useState<string | null>(null);
  const [ctxMenu, setCtxMenu] = useState<{ x: number; y: number; element: DocElement | null } | null>(null);
  const [fillPopBox, setFillPopBox] = useState<{ left: number; top: number; width: number } | null>(null);
  const [collapsed, setCollapsed] = useState<DesignerCollapse>(sessionCollapse);

  const historyRef = useRef(createHistory(emptyDocument()));
  const [doc, setDoc] = useState<OfferDocument>(() => emptyDocument());
  const docRef = useRef(doc);
  const beforeDragRef = useRef<OfferDocument | null>(null);
  const titleSnapRef = useRef<OfferDocument | null>(null);

  const workspaceRef = useRef<HTMLDivElement>(null);
  const scalerRef = useRef<HTMLDivElement>(null);
  const imageFileRef = useRef<HTMLInputElement>(null);
  const insertImageIdRef = useRef<string | null>(null);
  const openedIdRef = useRef<string | null>(null);
  const bootstrappedRef = useRef(false);
  const [exportOpen, setExportOpen] = useState(false);
  const exportMenuRef = useRef<HTMLDivElement>(null);
  const printOnceRef = useRef(false);
  const handleUndoRef = useRef<() => void>(() => {});
  const handleRedoRef = useRef<() => void>(() => {});
  const deleteSelectedRef = useRef<() => void>(() => {});
  const patchElementRef = useRef<(id: string, patch: Partial<DocElement>, record?: boolean) => void>(() => {});
  const handlePrintRef = useRef<() => void>(() => {});

  useEffect(() => {
    docRef.current = doc;
  }, [doc]);

  useEffect(() => {
    if (!exportOpen) return;
    const onDoc = (e: MouseEvent) => {
      if (!exportMenuRef.current?.contains(e.target as Node)) setExportOpen(false);
    };
    document.addEventListener('mousedown', onDoc);
    return () => document.removeEventListener('mousedown', onDoc);
  }, [exportOpen]);

  const syncHistoryFlags = useCallback(() => {
    setCanUndo(historyRef.current.canUndo());
    setCanRedo(historyRef.current.canRedo());
  }, []);

  const commitDoc = useCallback((next: OfferDocument, record = true) => {
    const stamped = { ...next, updatedAt: new Date().toISOString() };
    if (record) historyRef.current.set(deepCloneDoc(stamped), true);
    else historyRef.current.replace(deepCloneDoc(stamped));
    setDoc(stamped);
    docRef.current = stamped;
    syncHistoryFlags();
  }, [syncHistoryFlags]);

  const patchDocLive = useCallback((mutator: (d: OfferDocument) => OfferDocument) => {
    setDoc((prev) => {
      const next = mutator(deepCloneDoc(prev));
      historyRef.current.replace(deepCloneDoc(next));
      docRef.current = next;
      return next;
    });
  }, []);

  const flash = useCallback((msg: string, isError = false) => {
    if (isError) {
      setError(msg);
      setMessage('');
    } else {
      setMessage(msg);
      setError('');
    }
    setTimeout(() => {
      setMessage('');
      setError('');
    }, 3500);
  }, []);

  const pages = useMemo(() => sortedPages(doc), [doc]);
  const activePage = useMemo(
    () => pages.find((p) => p.id === activePageId) || pages[0],
    [pages, activePageId]
  );

  const loadProperty = useCallback(async (id: string, list: any[]) => {
    if (!id) {
      setProperty(null);
      return;
    }
    const fromList = list.find((p) => p.id === id);
    try {
      const full = await api(`/api/properties/${id}`);
      setProperty(full);
    } catch {
      setProperty(fromList || null);
    }
  }, []);

  const openDocument = useCallback(async (item: any, settingsData: any, list: any[], quiet = false) => {
    try {
      savedCountryContext.current = item;
      setSettings(recordOfficeSettings(settingsData, item));
      const content = typeof item.content === 'string' ? JSON.parse(item.content) : item.content;
      const migrated = migrateToV2(content);
      migrated.propertyId = migrated.propertyId || item.property_id || '';
      migrated.title = item.title || migrated.title;
      historyRef.current.reset(deepCloneDoc(migrated));
      setDoc(migrated);
      setEditingId(item.id);
      setActivePageId(sortedPages(migrated)[0]?.id || '');
      setSelectedId(null);
      setZoomMode('fit');
      await loadProperty(migrated.propertyId, list);
      syncHistoryFlags();
      if (!quiet) flash(systemT('design.ui.12'));
    } catch {
      flash(systemT('design.ui.13'), true);
    }
  }, [flash, loadProperty, syncHistoryFlags]);

  const load = useCallback(async () => {
    try {
      const [propsData, settingsData, brochures] = await Promise.all([
        api('/api/properties?limit=500'),
        api('/api/settings').catch(() => null),
        api('/api/brochures').catch(() => [])
      ]);
      const list = propsData.items || [];
      setProperties(list);
      setSettings(settingsData);

      const openId = searchParams.get('id');
      if (openId && openedIdRef.current !== openId) {
        openedIdRef.current = openId;
        const fromList = (brochures || []).find((b: any) => b.id === openId);
        if (fromList) {
          await openDocument(fromList, settingsData, list, true);
        } else {
          try {
            const one = await api(`/api/brochures/${openId}`);
            await openDocument(one, settingsData, list, true);
          } catch {
            setError(systemT('design.ui.14'));
            if (!bootstrappedRef.current) {
              const blank = emptyDocument();
              historyRef.current.reset(blank);
              setDoc(blank);
              setActivePageId(blank.pages[0].id);
              bootstrappedRef.current = true;
            }
          }
        }
      } else if (!openId && !bootstrappedRef.current) {
        const blank = getDocTemplate('royal-sheet').build();
        historyRef.current.reset(blank);
        setDoc(blank);
        setActivePageId(blank.pages[0].id);
        bootstrappedRef.current = true;
        syncHistoryFlags();
      }
    } catch (err: any) {
      setError(err?.message || systemT('design.ui.15'));
    } finally {
      setLoading(false);
    }
  }, [searchParams, openDocument, syncHistoryFlags]);

  useEffect(() => {
    load();
  }, [load]);

  useEffect(() => {
    const el = workspaceRef.current;
    if (!el) return;
    const measure = () => {
      const w = Math.max(240, el.clientWidth - WORKSPACE_PAD * 2);
      const h = Math.max(280, el.clientHeight - WORKSPACE_PAD * 2);
      setWorkspaceSize({ w, h });
    };
    measure();
    const ro = typeof ResizeObserver !== 'undefined' ? new ResizeObserver(measure) : null;
    ro?.observe(el);
    window.addEventListener('resize', measure);
    return () => {
      ro?.disconnect();
      window.removeEventListener('resize', measure);
    };
  }, [loading, previewMode]);

  const pageDef = getPageSize(doc.pageSize);
  const pagePx = { w: pageDef.widthMm * MM_TO_PX, h: pageDef.heightMm * MM_TO_PX };
  const fitWidthScale = pagePx.w > 0 ? workspaceSize.w / pagePx.w : 1;
  const previewScale =
    zoomMode === 'fit'
      ? Math.min(Math.max(fitWidthScale, 0.5), 1.35)
      : zoomMode === '100'
        ? 1
        : Math.min(2, Math.max(0.25, customZoom));

  const selected = useMemo(() => {
    if (!activePage || !selectedId) return null;
    return activePage.elements.find((e) => e.id === selectedId) || null;
  }, [activePage, selectedId]);

  // ===== مساحة العمل الأولوية: طي اللوحتين + شريط الصفحات (جلسة فقط) =====
  useEffect(() => {
    sessionCollapse = collapsed;
  }, [collapsed]);
  const hideElements = !previewMode && collapsed.elements;
  const hideProps = !previewMode && collapsed.props;
  const focusMode = hideElements && hideProps;
  const stripFolded = !previewMode && collapsed.strip;
  const activePageNo = Math.max(0, pages.findIndex((p) => p.id === activePage?.id)) + (activePage ? 1 : 0);
  const activeTabRef = useRef<HTMLButtonElement>(null);

  const setCollapse = useCallback((patch: Partial<DesignerCollapse>) => {
    setCollapsed((c) => ({ ...c, ...patch }));
  }, []);

  const goToPage = useCallback(
    (delta: number) => {
      const idx = pages.findIndex((p) => p.id === activePage?.id);
      if (idx < 0) return;
      const next = pages[Math.min(pages.length - 1, Math.max(0, idx + delta))];
      if (next && next.id !== activePage?.id) {
        setActivePageId(next.id);
        setSelectedId(null);
      }
    },
    [pages, activePage]
  );

  // تمركز الصفحة داخل الـCanvas تلقائيًا عند تغيير حالة اللوحات/الشريط/التقريب
  useLayoutEffect(() => {
    const ws = workspaceRef.current;
    if (!ws || previewMode) return;
    const fitsH = pagePx.w * previewScale <= ws.clientWidth;
    const fitsV = pagePx.h * previewScale <= ws.clientHeight;
    ws.classList.toggle('is-fit-centered', fitsH && fitsV);
    // إشارة التمرير الأفقي تعتمد على اتجاه النص (RTL: سالبة في Chromium)
    const rtl = getComputedStyle(ws).direction === 'rtl';
    const overflowX = ws.scrollWidth - ws.clientWidth;
    const overflowY = ws.scrollHeight - ws.clientHeight;
    if (overflowX > 0) ws.scrollLeft = (rtl ? -1 : 1) * (overflowX / 2);
    if (overflowY > 0) ws.scrollTop = overflowY / 2;
  }, [hideElements, hideProps, stripFolded, previewScale, doc.pageSize, previewMode]);

  // إبقاء تبويب الصفحة الفعالة مرئيًا داخل التمرير الأفقي
  useEffect(() => {
    activeTabRef.current?.scrollIntoView({ inline: 'nearest', block: 'nearest' });
  }, [activePageId, stripFolded]);

  const fillableSelected =
    !!selected &&
    (selected.type === 'property_field' ||
      selected.type === 'office_field' ||
      selected.type === 'text' ||
      selected.type === 'heading' ||
      selected.type === 'qr');
  const fillPopLabel =
    selected && fillableSelected
      ? selected.binding
        ? bindingLabel(selected.binding)
        : selected.type === 'heading'
          ? systemT('design.ui.1')
          : selected.type === 'qr'
            ? systemT('design.ui.16')
            : systemT('design.ui.17')
      : '';

  useLayoutEffect(() => {
    const scaler = scalerRef.current;
    const show = !previewMode && selected && (fillableSelected || selected.type === 'specs_table');
    if (!scaler || !show) {
      setFillPopBox(null);
      return;
    }
    const pageEl = scaler.querySelector('.ods-doc-page') as HTMLElement | null;
    const hit = scaler.querySelector('.ods-el-hit.is-selected') as HTMLElement | null;
    if (!pageEl || !hit) {
      setFillPopBox(null);
      return;
    }
    const sr = scaler.getBoundingClientRect();
    const pr = pageEl.getBoundingClientRect();
    const er = hit.getBoundingClientRect();
    setFillPopBox(
      placeFillPop({
        elLeft: er.left - sr.left,
        elTop: er.top - sr.top,
        elWidth: er.width,
        elHeight: er.height,
        minLeft: pr.left - sr.left,
        maxRight: pr.right - sr.left,
        minTop: pr.top - sr.top,
        maxBottom: pr.bottom - sr.top,
        minWidth: Math.min(220, pr.width),
        popHeight: 48,
        gap: 6
      })
    );
  }, [
    selected,
    selectedId,
    fillableSelected,
    previewMode,
    previewScale,
    doc,
    selected?.x,
    selected?.y,
    selected?.width,
    selected?.height,
    selected?.content
  ]);

  const updatePages = (nextPages: DocPage[], record = true) => {
    commitDoc({ ...doc, pages: nextPages }, record);
  };

  const mapActiveElements = (fn: (els: DocElement[]) => DocElement[], record = true) => {
    if (!activePage) return;
    const nextPages = doc.pages.map((p) =>
      p.id === activePage.id ? { ...p, elements: fn(p.elements.map((e) => ({ ...e, style: { ...e.style } }))) } : p
    );
    updatePages(nextPages, record);
  };

  const addFromPalette = (item: PaletteItem) => {
    if (!activePage) return;
    const maxZ = activePage.elements.reduce((m, e) => Math.max(m, e.zIndex), 0);
    const slot = item.type === 'image' ? imageSlotForPage(doc.pageSize, 'hero') : null;
    const el = createElement(item.type, {
      binding: item.binding || null,
      zIndex: maxZ + 1,
      x: 18 + (activePage.elements.length % 5) * 4,
      y: 24 + (activePage.elements.length % 5) * 4,
      ...(slot ? { width: slot.width, height: slot.height, style: { objectFit: 'contain' as const } } : {})
    });
    mapActiveElements((els) => [...els, el], true);
    setSelectedId(el.id);
  };

  const pickImageFor = (id: string) => {
    insertImageIdRef.current = id;
    imageFileRef.current?.click();
  };

  const onImageFileChosen = async (file: File | undefined) => {
    const id = insertImageIdRef.current;
    insertImageIdRef.current = null;
    if (!file || !id || !activePage) return;
    const target = activePage.elements.find((e) => e.id === id);
    if (!target || target.type !== 'image') return;
    try {
      const dataUrl = await fileToPropertyImage(file);
      const next = applyImageToFrame(target, dataUrl);
      patchElement(id, next, true);
    } catch (err: any) {
      flash(err?.message || systemT('design.ui.18'), true);
    }
  };

  const patchElement = (id: string, patch: Partial<DocElement>, record = true) => {
    if ('content' in patch) patch = { ...patch, systemTextKey: undefined };
    mapActiveElements(
      (els) =>
        els.map((e) =>
          e.id === id
            ? {
                ...e,
                ...patch,
                style: patch.style ? { ...e.style, ...patch.style } : e.style
              }
            : e
        ),
      record
    );
  };

  const onPatchElementLive = (id: string, patch: Partial<DocElement>) => {
    if ('content' in patch) patch = { ...patch, systemTextKey: undefined };
    if (!beforeDragRef.current) beforeDragRef.current = deepCloneDoc(docRef.current);
    patchDocLive((d) => {
      const page = sortedPages(d).find((p) => p.id === (activePage?.id || ''));
      if (!page) return d;
      return {
        ...d,
        pages: d.pages.map((p) =>
          p.id === page.id
            ? {
                ...p,
                elements: p.elements.map((e) => (e.id === id ? { ...e, ...patch } : e))
              }
            : p
        )
      };
    });
  };

  const onCommitDrag = () => {
    const before = beforeDragRef.current;
    beforeDragRef.current = null;
    const after = deepCloneDoc(docRef.current);
    after.updatedAt = new Date().toISOString();
    if (before) {
      historyRef.current.replace(deepCloneDoc(before));
      historyRef.current.set(deepCloneDoc(after), true);
    } else {
      historyRef.current.replace(deepCloneDoc(after));
    }
    setDoc(after);
    docRef.current = after;
    syncHistoryFlags();
  };

  const handleUndo = () => {
    const prev = historyRef.current.undo();
    if (!prev) return;
    const next = deepCloneDoc(prev);
    setDoc(next);
    docRef.current = next;
    setSelectedId(null);
    syncHistoryFlags();
  };

  const handleRedo = () => {
    const nextHist = historyRef.current.redo();
    if (!nextHist) return;
    const next = deepCloneDoc(nextHist);
    setDoc(next);
    docRef.current = next;
    setSelectedId(null);
    syncHistoryFlags();
  };

  const deleteSelected = () => {
    if (!selectedId) return;
    mapActiveElements((els) => els.filter((e) => e.id !== selectedId), true);
    setSelectedId(null);
    setTextEditId(null);
  };

  const duplicateSelected = () => {
    if (!selected || !activePage) return;
    const maxZ = activePage.elements.reduce((m, e) => Math.max(m, e.zIndex), 0);
    const copy: DocElement = {
      ...selected,
      id: newId('el'),
      x: selected.x + 4,
      y: selected.y + 4,
      zIndex: maxZ + 1,
      style: { ...selected.style }
    };
    mapActiveElements((els) => [...els, copy], true);
    setSelectedId(copy.id);
  };

  const copySelectedElement = async () => {
    if (!selected) return;
    const payload = JSON.stringify({ obrOds: 1, element: selected });
    try {
      await navigator.clipboard.writeText(payload);
    } catch {
      flash(systemT('design.ui.19'), true);
    }
  };

  const pasteElement = async () => {
    if (!activePage) return;
    try {
      const raw = await navigator.clipboard.readText();
      const data = JSON.parse(raw);
      if (!data || data.obrOds !== 1 || !data.element) return;
      const src = data.element as DocElement;
      const maxZ = activePage.elements.reduce((m, e) => Math.max(m, e.zIndex), 0);
      const copy: DocElement = {
        ...src,
        id: newId('el'),
        x: (src.x || 0) + 4,
        y: (src.y || 0) + 4,
        zIndex: maxZ + 1,
        style: { ...(src.style || {}) }
      };
      mapActiveElements((els) => [...els, copy], true);
      setSelectedId(copy.id);
    } catch {
      flash(systemT('design.ui.20'), true);
    }
  };

  const beginTextEdit = (el: DocElement | null) => {
    if (!el) return;
    setSelectedId(el.id);
    if ((el.type === 'text' || el.type === 'heading') && !el.binding) {
      setTextEditId(el.id);
    } else {
      setTextEditId(null);
    }
  };

  const layerSelected = (op: 'front' | 'back' | 'forward' | 'backward') => {
    if (!selectedId || !activePage) return;
    mapActiveElements((els) => {
      const sorted = [...els].sort((a, b) => a.zIndex - b.zIndex);
      const idx = sorted.findIndex((e) => e.id === selectedId);
      if (idx < 0) return els;
      const item = sorted[idx];
      let next = [...sorted];
      if (op === 'front') {
        next = [...sorted.filter((e) => e.id !== selectedId), item];
      } else if (op === 'back') {
        next = [item, ...sorted.filter((e) => e.id !== selectedId)];
      } else if (op === 'forward' && idx < sorted.length - 1) {
        next[idx] = sorted[idx + 1];
        next[idx + 1] = item;
      } else if (op === 'backward' && idx > 0) {
        next[idx] = sorted[idx - 1];
        next[idx - 1] = item;
      }
      return next.map((e, i) => ({ ...e, zIndex: i + 1 }));
    }, true);
  };

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return;
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'z') {
        e.preventDefault();
        if (e.shiftKey) handleRedoRef.current();
        else handleUndoRef.current();
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'y') {
        e.preventDefault();
        handleRedoRef.current();
        return;
      }
      if ((e.key === 'Delete' || e.key === 'Backspace') && selectedId && !previewMode) {
        e.preventDefault();
        deleteSelectedRef.current();
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'c' && selectedId && !previewMode) {
        e.preventDefault();
        void copySelectedElement();
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'v' && !previewMode) {
        e.preventDefault();
        void pasteElement();
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'd' && selectedId && !previewMode) {
        e.preventDefault();
        duplicateSelected();
        return;
      }
      if (!selectedId || !selected || previewMode) return;
      const step = e.shiftKey ? 2 : 0.5;
      let dx = 0;
      let dy = 0;
      if (e.key === 'ArrowLeft') dx = -step;
      if (e.key === 'ArrowRight') dx = step;
      if (e.key === 'ArrowUp') dy = -step;
      if (e.key === 'ArrowDown') dy = step;
      if (dx || dy) {
        e.preventDefault();
        const size = getPageSize(doc.pageSize);
        patchElementRef.current(
          selectedId,
          {
            x: Math.max(0, Math.min(size.widthMm - selected.width, selected.x + dx)),
            y: Math.max(0, Math.min(size.heightMm - selected.height, selected.y + dy))
          },
          true
        );
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [selectedId, selected, previewMode, doc.pageSize]);

  const addPage = () => {
    const order = pages.length;
    const page: DocPage = {
      id: newId('page'),
      name: tf('design.pageNumber', {number: order + 1}),
      systemName: true,
      order,
      elements: []
    };
    commitDoc({ ...doc, pages: [...doc.pages, page] }, true);
    setActivePageId(page.id);
    setSelectedId(null);
  };

  const duplicatePage = () => {
    if (!activePage) return;
    const copy = clonePage(activePage);
    copy.order = pages.length;
    commitDoc({ ...doc, pages: [...doc.pages, copy] }, true);
    setActivePageId(copy.id);
  };

  const deletePage = () => {
    if (pages.length <= 1) {
      flash(systemT('design.ui.21'), true);
      return;
    }
    if (!activePage) return;
    if (!confirm(tf('design.deletePage', {name: activePage.name}))) return;
    const next = doc.pages
      .filter((p) => p.id !== activePage.id)
      .map((p, i) => ({ ...p, order: i }));
    commitDoc({ ...doc, pages: next }, true);
    setActivePageId(next[0]?.id || '');
    setSelectedId(null);
  };

  const renamePage = () => {
    if (!activePage) return;
    const name = prompt(systemT('design.ui.22'), activePage.systemName ? tf('design.pageNumber', {number: activePage.order + 1}) : activePage.name);
    if (!name?.trim()) return;
    commitDoc(
      {
        ...doc,
        pages: doc.pages.map((p) => (p.id === activePage.id ? { ...p, name: name.trim(), systemName: false } : p))
      },
      true
    );
  };

  const reorderPages = (fromId: string, toId: string) => {
    if (fromId === toId) return;
    const ordered = sortedPages(doc);
    const from = ordered.findIndex((p) => p.id === fromId);
    const to = ordered.findIndex((p) => p.id === toId);
    if (from < 0 || to < 0) return;
    const next = [...ordered];
    const [item] = next.splice(from, 1);
    next.splice(to, 0, item);
    commitDoc(
      { ...doc, pages: next.map((p, i) => ({ ...p, order: i })) },
      true
    );
  };

  const applyTemplate = (id: string) => {
    if (!confirm(systemT('design.ui.23'))) return;
    const built = getDocTemplate(id).build();
    built.propertyId = doc.propertyId;
    historyRef.current.reset(deepCloneDoc(built));
    setDoc(built);
    setActivePageId(built.pages[0]?.id || '');
    setSelectedId(null);
    setZoomMode('fit');
    syncHistoryFlags();
  };

  const applyProperty = async (id: string) => {
    commitDoc({ ...doc, propertyId: id }, true);
    await loadProperty(id, properties);
  };

  const handleSave = async () => {
    const title = (doc.titleKey ? t(doc.titleKey) : doc.title)?.trim() || property?.name || systemT('design.ui.24');
    setBusy(true);
    try {
      const payload = {
        property_id: doc.propertyId || null,
        template_id: doc.templateOrigin || 'custom',
        title,
        content: { ...doc, title, version: 2 }
      };
      if (editingId) {
        await api(`/api/brochures/${editingId}`, { method: 'PUT', body: payload });
        flash(systemT('design.ui.25'));
      } else {
        const created = await api('/api/brochures', { method: 'POST', body: payload });
        const saved = await api(`/api/brochures/${created.id}`);
        savedCountryContext.current = saved;
        setSettings(recordOfficeSettings(settings, saved));
        setEditingId(created.id);
        setSearchParams({ id: created.id }, { replace: true });
        openedIdRef.current = created.id;
        flash(systemT('design.ui.25'));
      }
    } catch (err: any) {
      flash(err?.message || systemT('design.ui.26'), true);
    } finally {
      setBusy(false);
    }
  };

  const handlePrint = () => {
    let styleEl = document.getElementById('ods-print-page-style') as HTMLStyleElement | null;
    if (!styleEl) {
      styleEl = document.createElement('style');
      styleEl.id = 'ods-print-page-style';
      document.head.appendChild(styleEl);
    }
    styleEl.textContent = `@media print {
      @page { size: ${pageDef.cssSize}; margin: 0; }
      .ods-print-page { page-break-after: always; break-after: page; }
      .ods-print-page:last-child { page-break-after: auto; break-after: auto; }
    }`;
    document.body.classList.add('ods-printing');
    window.print();
    setTimeout(() => document.body.classList.remove('ods-printing'), 800);
  };

  handleUndoRef.current = handleUndo;
  handleRedoRef.current = handleRedo;
  deleteSelectedRef.current = deleteSelected;
  patchElementRef.current = patchElement;
  handlePrintRef.current = handlePrint;

  useEffect(() => {
    const refreshBound = () => {
      const pid = docRef.current.propertyId;
      if (pid) loadProperty(pid, properties);
      api('/api/settings').then(s => setSettings(savedCountryContext.current ? recordOfficeSettings(s, savedCountryContext.current) : s)).catch(() => {});
    };
    const onVis = () => {
      if (document.visibilityState === 'visible') refreshBound();
    };
    window.addEventListener('focus', refreshBound);
    document.addEventListener('visibilitychange', onVis);
    return () => {
      window.removeEventListener('focus', refreshBound);
      document.removeEventListener('visibilitychange', onVis);
    };
  }, [loadProperty, properties]);

  useEffect(() => {
    if (loading || printOnceRef.current) return;
    if (searchParams.get('print') !== '1') return;
    printOnceRef.current = true;
    setPreviewMode(true);
    const t = window.setTimeout(() => handlePrintRef.current(), 600);
    return () => window.clearTimeout(t);
  }, [loading, searchParams]);

  const buildExportBody = () => {
    const root = document.getElementById('ods-print-root');
    if (!root) return null;
    const styleEls = Array.from(document.querySelectorAll('style'))
      .map((el) => el.innerHTML)
      .join('\n');
    return { bodyHtml: root.outerHTML, extraCss: styleEls };
  };

  const handleExportFormat = async (format: 'pdf' | 'doc' | 'png') => {
    setExportOpen(false);
    setBusy(true);
    try {
      const pack = buildExportBody();
      if (!pack) {
        flash(systemT('design.ui.27'), true);
        return;
      }
      const result = await exportDesignDocument({
        format,
        title: (doc.titleKey ? t(doc.titleKey) : doc.title) || systemT('design.ui.28'),
        bodyHtml: pack.bodyHtml,
        pageCssSize: pageDef.cssSize,
        extraCss: pack.extraCss,
        widthPx: Math.ceil(pagePx.w) + 24,
        heightPx: Math.ceil(pagePx.h) * Math.max(1, pageCount(doc)) + 24
      });
      if ('cancelled' in result) {
        flash(systemT('design.ui.29'));
        return;
      }
      const labels = { pdf: 'PDF', doc: 'Word', png: systemT('design.ui.30') } as const;
      flash(tf('design.fileSaved', {format: labels[format], path: result.path}));
    } catch (err: any) {
      flash(err?.message || systemT('design.ui.31'), true);
    } finally {
      setBusy(false);
    }
  };

  const handleExportFile = async () => {
    setBusy(true);
    try {
      const pack = buildExportBody();
      if (!pack) {
        flash(systemT('design.ui.27'), true);
        return;
      }
      const result = await exportDesignDocument({
        format: 'html',
        title: (doc.titleKey ? t(doc.titleKey) : doc.title) || systemT('design.ui.28'),
        bodyHtml: pack.bodyHtml,
        pageCssSize: pageDef.cssSize,
        extraCss: pack.extraCss,
        widthPx: Math.ceil(pagePx.w),
        heightPx: Math.ceil(pagePx.h)
      });
      if ('cancelled' in result) {
        flash(systemT('design.ui.29'));
        return;
      }
      flash(tf('design.fileSaved', {format: 'HTML', path: result.path}));
    } catch (err: any) {
      flash(err?.message || systemT('design.ui.32'), true);
    } finally {
      setBusy(false);
    }
  };

  const bumpZoom = (delta: number) => {
    const next = Math.min(2, Math.max(0.25, previewScale + delta));
    setCustomZoom(next);
    setZoomMode('custom');
  };

  const paletteGroups = useMemo(() => {
    const map = new Map<string, PaletteItem[]>();
    for (const item of PALETTE) {
      const list = map.get(item.group) || [];
      list.push(item);
      map.set(item.group, list);
    }
    return [...map.entries()];
  }, [lang]);

  const designerMenuItems = useMemo((): AppMenuItem[] => {
    const el = ctxMenu?.element || selected;
    const canEditText = !!el && (el.type === 'text' || el.type === 'heading' || el.type === 'property_field' || el.type === 'office_field' || el.type === 'qr');
    const items: AppMenuItem[] = [
      {
        id: 'edit',
        get label() { return systemT('design.ui.33'); },
        disabled: !canEditText,
        onClick: () => beginTextEdit(el || null)
      },
      {
        id: 'insert-image',
        get label() { return systemT('design.ui.34'); },
        disabled: el?.type !== 'image',
        onClick: () => el?.type === 'image' && pickImageFor(el.id)
      },
      {
        id: 'dup',
        get label() { return systemT('design.ui.35'); },
        disabled: !el,
        onClick: duplicateSelected
      },
      {
        id: 'copy',
        get label() { return systemT('design.ui.36'); },
        shortcut: 'Ctrl+C',
        disabled: !el,
        onClick: () => { void copySelectedElement(); }
      },
      {
        id: 'paste',
        get label() { return systemT('design.ui.37'); },
        shortcut: 'Ctrl+V',
        onClick: () => { void pasteElement(); }
      },
      { id: 'sep1', separator: true },
      {
        id: 'front',
        get label() { return systemT('design.ui.38'); },
        disabled: !el,
        onClick: () => layerSelected('front')
      },
      {
        id: 'back',
        get label() { return systemT('design.ui.39'); },
        disabled: !el,
        onClick: () => layerSelected('back')
      },
      { id: 'sep2', separator: true },
      {
        id: 'undo',
        get label() { return systemT('design.ui.40'); },
        shortcut: 'Ctrl+Z',
        disabled: !canUndo,
        onClick: handleUndo
      },
      {
        id: 'redo',
        get label() { return systemT('design.ui.41'); },
        shortcut: 'Ctrl+Y',
        disabled: !canRedo,
        onClick: handleRedo
      },
      { id: 'sep3', separator: true },
      {
        id: 'del',
        get label() { return systemT('design.ui.42'); },
        shortcut: 'Del',
        danger: true,
        disabled: !el,
        onClick: deleteSelected
      }
    ];
    return items;
  }, [ctxMenu, selected, canUndo, canRedo]);

  if (loading) {
    return <div className="content-card">{t('ods.loading')}</div>;
  }

  return (
    <div className="ods-studio ods-editor">
      <input
        ref={imageFileRef}
        type="file"
        accept="image/*"
        hidden
        onChange={(e) => {
          const file = e.target.files?.[0];
          e.target.value = '';
          void onImageFileChosen(file);
        }}
      />
      <div className="ods-toolbar">
        <div className="ods-toolbar-title">
          <PenTool size={16} strokeWidth={1.8} aria-hidden />
          {t('ods.title')} <SectionGuideBanner id="offer-design" />
        </div>
        <div className="ods-toolbar-actions">
          <button type="button" className="btn btn-primary btn-sm" disabled={busy} onClick={handleSave}>
            <Save size={14} /> {t('ods.save-library')}
          </button>
          <button type="button" className="btn btn-outline btn-sm" disabled={!canUndo} onClick={handleUndo} title={t('ods.undo')}>
            <Undo2 size={14} /> {t('ods.undo')}
          </button>
          <button type="button" className="btn btn-outline btn-sm" disabled={!canRedo} onClick={handleRedo} title={t('ods.redo')}>
            <Redo2 size={14} /> {t('ods.redo')}
          </button>
          <span className="ods-toolbar-sep" />
          <button type="button" className={`btn btn-sm ${zoomMode === 'fit' ? 'btn-primary' : 'btn-outline'}`} onClick={() => setZoomMode('fit')}>
            <Maximize2 size={13} /> {t('ods.fit')}
          </button>
          <button type="button" className="btn btn-sm btn-outline" onClick={() => bumpZoom(-0.1)}>
            <ZoomOut size={13} />
          </button>
          <span className="ods-zoom-label">{Math.round(previewScale * 100)}%</span>
          <button type="button" className="btn btn-sm btn-outline" onClick={() => bumpZoom(0.1)}>
            <ZoomIn size={13} />
          </button>
          <button
            type="button"
            className={`btn btn-sm ${previewMode ? 'btn-primary' : 'btn-outline'}`}
            onClick={() => {
              setPreviewMode((v) => {
                const next = !v;
                if (next) {
                  setZoomMode('fit');
                  setSelectedId(null);
                }
                return next;
              });
            }}
          >
            <Eye size={13} /> {previewMode ? t('ods.preview-close') : t('ods.preview')}
          </button>
          <button
            type="button"
            className={`btn btn-sm ${focusMode ? 'btn-primary' : 'btn-outline'}`}
            disabled={previewMode}
            onClick={() => setCollapse({ elements: true, props: true })}
            title={systemT('design.ui.43')}
          >
            <Focus size={13} /> {t('ods.focus')}
          </button>
          <button type="button" className="btn btn-sm btn-outline" onClick={handlePrint}>
            <Printer size={13} /> {t('ods.print-save')}
          </button>
          <div className="ods-export-wrap" ref={exportMenuRef}>
            <button
              type="button"
              className="btn btn-sm btn-outline"
              disabled={busy}
              aria-expanded={exportOpen}
              onClick={() => setExportOpen((v) => !v)}
            >
              <FileDown size={13} /> {t('ods.export')}
            </button>
            {exportOpen && (
              <div className="ods-export-menu" role="menu">
                {designExportFormats().map((f) => (
                  <button
                    key={f.id}
                    type="button"
                    role="menuitem"
                    disabled={busy}
                    onClick={() => void handleExportFormat(f.id as 'pdf' | 'doc' | 'png')}
                  >
                    {f.id === 'pdf' ? t('ods.export-pdf') : f.id === 'doc' ? t('ods.export-word') : t('ods.export-image')}
                  </button>
                ))}
                <button type="button" role="menuitem" disabled={busy} onClick={() => { setExportOpen(false); void handleExportFile(); }}>
                  {t('ods.export-html')}
                </button>
              </div>
            )}
          </div>
        </div>
      </div>

      {(message || error) && (
        <div className={error ? 'error-alert' : 'success-alert'} style={{ marginBottom: 8 }}>
          {error || message}
        </div>
      )}

      <div
        className={`ods-editor-body${previewMode ? ' is-preview' : ''}${hideElements ? ' hide-elements' : ''}${
          hideProps ? ' hide-props' : ''
        }`}
      >
        {!previewMode && !hideElements && (
          <aside className="ods-palette content-card">
            <div className="form-group">
              <label>{t('ods.doc-name')}</label>
              <input
                className="form-input"
                value={doc.titleKey ? t(doc.titleKey) : doc.title}
                onFocus={() => {
                  titleSnapRef.current = deepCloneDoc(docRef.current);
                }}
                onChange={(e) => patchDocLive((d) => ({ ...d, title: e.target.value, titleKey: undefined }))}
                onBlur={() => {
                  const before = titleSnapRef.current;
                  titleSnapRef.current = null;
                  const after = deepCloneDoc(docRef.current);
                  if (before && before.title !== after.title) {
                    historyRef.current.replace(deepCloneDoc(before));
                    historyRef.current.set(after, true);
                    setDoc(after);
                    docRef.current = after;
                    syncHistoryFlags();
                  }
                }}
              />
            </div>
            <div className="form-group">
              <label>{t('ods.linked-property')}</label>
              <select
                className="form-input"
                value={doc.propertyId || ''}
                onChange={(e) => applyProperty(e.target.value)}
              >
                <option value="">{t('ods.no-property')}</option>
                {properties.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.name}
                    {p.city ? ` · ${p.city}` : ''}
                  </option>
                ))}
              </select>
            </div>
            <div className="form-group">
              <label>{t('ods.page-size')}</label>
              <select
                className="form-input"
                value={doc.pageSize}
                onChange={(e) => {
                  commitDoc({ ...doc, pageSize: e.target.value as PageSizeId }, true);
                  setZoomMode('fit');
                }}
              >
                {PAGE_SIZES.map((s) => (
                  <option key={s.id} value={s.id}>
                    {s.label}
                  </option>
                ))}
              </select>
            </div>
            <div className="form-group">
              <label>{t('ods.templates')}</label>
              <select
                className="form-input"
                defaultValue=""
                onChange={(e) => {
                  if (e.target.value) applyTemplate(e.target.value);
                  e.target.value = '';
                }}
              >
                <option value="">{t('ods.choose-template')}</option>
                {DOC_TEMPLATES.map((tpl) => (
                  <option key={tpl.id} value={tpl.id} title={tpl.description}>
                    {tpl.name}
                  </option>
                ))}
              </select>
            </div>

            <div className="ods-pane-head">
              <h3><LayoutTemplate size={15} strokeWidth={1.8} aria-hidden /> {t('ods.elements')}</h3>
              <button
                type="button"
                className="ods-pane-collapse"
                onClick={() => setCollapse({ elements: true })}
                title={systemT('design.ui.44')}
                aria-label={systemT('design.ui.44')}
              >
                <PanelRightClose size={14} />
              </button>
            </div>
            {paletteGroups.map(([group, items]) => (
              <div key={group} className="ods-palette-group">
                <div className="ods-palette-group-title">{group}</div>
                <div className="ods-palette-items">
                  {items.map((item) => (
                    <button
                      key={`${item.group}-${item.label}`}
                      type="button"
                      className="ods-palette-btn"
                      onClick={() => addFromPalette(item)}
                      title={item.label}
                    >
                      <PaletteIcon type={item.type} />
                      <span>{item.label}</span>
                    </button>
                  ))}
                </div>
              </div>
            ))}
          </aside>
        )}

        <section className="ods-canvas-pane">
          {!previewMode && hideElements && (
            <button
              type="button"
              className="ods-panel-restore is-start"
              onClick={() => setCollapse({ elements: false })}
              title={systemT('design.ui.45')}
              aria-label={systemT('design.ui.45')}
            >
              <span className="ods-restore-ico"><PanelRightOpen size={17} aria-hidden /></span>
              <span className="ods-restore-txt">
                <span className="ods-restore-title">{t('ods.panel-elements')}</span>
                <span className="ods-restore-sub"> {systemT('design.ui.46')} </span>
              </span>
            </button>
          )}
          {!previewMode && hideProps && (
            <button
              type="button"
              className="ods-panel-restore is-end"
              onClick={() => setCollapse({ props: false })}
              title={systemT('design.ui.47')}
              aria-label={systemT('design.ui.47')}
            >
              <span className="ods-restore-ico"><PanelLeftOpen size={17} aria-hidden /></span>
              <span className="ods-restore-txt">
                <span className="ods-restore-title">{t('ods.panel-props')}</span>
                <span className="ods-restore-sub"> {systemT('design.ui.48')} </span>
              </span>
            </button>
          )}
          {!previewMode && focusMode && (
            <button
              type="button"
              className="ods-focus-exit"
              onClick={() => setCollapse({ elements: false, props: false })}
              title={t('ods.focus-exit')}
              aria-label={t('ods.focus-exit')}
            >
              <span className="ods-restore-ico"><Minimize2 size={17} aria-hidden /></span>
              <span className="ods-restore-txt">
                <span className="ods-restore-title">{t('ods.focus-exit')}</span>
                <span className="ods-restore-sub"> {systemT('design.ui.49')} </span>
              </span>
            </button>
          )}
          <div
            className="ods-preview-workspace"
            ref={workspaceRef}
            onContextMenu={(e) => {
              if (previewMode) return;
              if ((e.target as HTMLElement).closest('.ods-doc-page')) return;
              e.preventDefault();
              e.stopPropagation();
              setCtxMenu({ x: e.clientX, y: e.clientY, element: selected });
            }}
          >
            <div
              className="ods-preview-scaler"
              ref={scalerRef}
              style={{
                width: pagePx.w * previewScale,
                height: pagePx.h * previewScale
              }}
            >
              <div
                className="ods-preview-scale-inner"
                style={{
                  width: pagePx.w,
                  height: pagePx.h,
                  transform: `scale(${previewScale})`,
                  transformOrigin: 'top left'
                }}
              >
                {activePage && (
                  <DocumentCanvas
                    doc={doc}
                    page={activePage}
                    property={property}
                    settings={settings}
                    selectedId={previewMode ? null : selectedId}
                    scale={previewScale}
                    editing={!previewMode}
                    textEditId={textEditId}
                    onSelect={(id) => {
                      setSelectedId(id);
                      if (id !== textEditId) setTextEditId(null);
                    }}
                    onPatchElement={onPatchElementLive}
                    onCommitDrag={onCommitDrag}
                    onEditContent={(id, content) => patchElement(id, { content }, true)}
                    onTextEditId={setTextEditId}
                    onElementContextMenu={(e, el) => {
                      setCtxMenu({ x: e.clientX, y: e.clientY, element: el });
                    }}
                    onInsertImage={pickImageFor}
                  />
                )}
              </div>
              {!previewMode && fillableSelected && selected && fillPopBox && (
                <div
                  className="ods-fill-pop"
                  style={{
                    left: fillPopBox.left,
                    top: fillPopBox.top,
                    width: fillPopBox.width
                  }}
                  onPointerDown={(e) => e.stopPropagation()}
                >
                  <span className="ods-fill-pop-label">{fillPopLabel}</span>
                  <span className="ods-fill-pop-colon">:</span>
                  <input
                    key={selected.id}
                    className="ods-fill-pop-input"
                    value={selected.content || ''}
                    placeholder={systemT('design.ui.50')}
                    aria-label={tf('design.fill', {label: fillPopLabel})}
                    autoFocus
                    onPointerDown={(e) => e.stopPropagation()}
                    onChange={(e) => onPatchElementLive(selected.id, { content: e.target.value })}
                    onBlur={onCommitDrag}
                  />
                </div>
              )}
              {!previewMode && selected?.type === 'specs_table' && fillPopBox && (
                <div
                  className="ods-fill-pop ods-fill-pop-hint"
                  style={{
                    left: fillPopBox.left,
                    top: fillPopBox.top,
                    width: fillPopBox.width
                  }}
                >
                   {systemT('design.ui.51')} </div>
              )}
            </div>
          </div>

          <div className="ods-page-tabs-wrap" role="group" aria-label={systemT('design.ui.52')}>
            <div className="ods-page-tabs-main">
              {!previewMode && (
                <>
                  <button
                    type="button"
                    className="ods-page-nav"
                    onClick={() => goToPage(-1)}
                    disabled={activePageNo <= 1}
                    title={systemT('design.ui.53')}
                    aria-label={systemT('design.ui.53')}
                  >
                    <ChevronRight size={14} />
                  </button>
                  <button
                    type="button"
                    className="ods-page-nav"
                    onClick={() => goToPage(1)}
                    disabled={activePageNo >= pages.length}
                    title={systemT('design.ui.54')}
                    aria-label={systemT('design.ui.54')}
                  >
                    <ChevronLeft size={14} />
                  </button>
                </>
              )}
              <div className="ods-page-tabs" role="tablist">
                {pages.map((p) => (
                  <button
                    key={p.id}
                    ref={p.id === activePage?.id ? activeTabRef : undefined}
                    type="button"
                    role="tab"
                    aria-selected={p.id === activePage?.id}
                    draggable={!previewMode}
                    className={`ods-page-tab${p.id === activePage?.id ? ' active' : ''}`}
                    onClick={() => {
                      setActivePageId(p.id);
                      setSelectedId(null);
                    }}
                    onDoubleClick={renamePage}
                    onDragStart={() => setDragPageId(p.id)}
                    onDragOver={(e) => e.preventDefault()}
                    onDrop={() => {
                      if (dragPageId) reorderPages(dragPageId, p.id);
                      setDragPageId(null);
                    }}
                  >
                    {p.systemName ? tf('design.pageNumber', { number: p.order + 1 }) : p.name}
                  </button>
                ))}
                {!previewMode && (
                  <button type="button" className="ods-page-tab ods-page-add" onClick={addPage} title={systemT('design.ui.55')}>
                    <Plus size={14} />
                  </button>
                )}
              </div>
            </div>
            <div className="ods-page-tabs-side">
              {!previewMode && (
                <>
                  <button type="button" className="btn btn-sm btn-outline" onClick={duplicatePage} title={systemT('design.ui.56')}>
                    <Copy size={12} />
                  </button>
                  <button type="button" className="btn btn-sm btn-outline" onClick={deletePage} title={systemT('design.ui.57')}>
                    <Trash2 size={12} />
                  </button>
                </>
              )}
              {stripFolded ? (
                <span className="ods-page-meta">
                   {systemT('design.ui.58')} {activePageNo}  {systemT('design.ui.59')} {pages.length}
                </span>
              ) : (
                <span className="ods-page-meta">
                  {pageCount(doc)}  {systemT('design.ui.60')} {pageDef.label}
                </span>
              )}
              <button
                type="button"
                className="ods-strip-fold"
                disabled={previewMode}
                onClick={() => setCollapse({ strip: !collapsed.strip })}
                title={stripFolded ? systemT('design.ui.61') : systemT('design.ui.62')}
                aria-label={stripFolded ? systemT('design.ui.61') : systemT('design.ui.62')}
              >
                {stripFolded ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
              </button>
            </div>
          </div>
        </section>

        {!previewMode && !hideProps && (
          <aside className="ods-props-pane content-card">
            <div className="ods-pane-head">
              <h3><SlidersHorizontal size={15} strokeWidth={1.8} aria-hidden />  {systemT('design.ui.63')} </h3>
              <button
                type="button"
                className="ods-pane-collapse"
                onClick={() => setCollapse({ props: true })}
                title={systemT('design.ui.64')}
                aria-label={systemT('design.ui.64')}
              >
                <PanelLeftClose size={14} />
              </button>
            </div>
            <PropertiesPanel
              element={selected}
              onChange={(patch) => selected && patchElement(selected.id, patch, true)}
              onStyle={(stylePatch) =>
                selected && patchElement(selected.id, { style: { ...selected.style, ...stylePatch } }, true)
              }
              onDelete={deleteSelected}
              onLayer={layerSelected}
            />
          </aside>
        )}
      </div>

      {/* مصدر الطباعة/PDF — نفس العناصر، خارج مساحة التحرير */}
      <div className="ods-print-only" aria-hidden>
        <PrintDocument doc={doc} property={property} settings={settings} />
      </div>
      {ctxMenu && !previewMode && (
        <AppContextMenu
          x={ctxMenu.x}
          y={ctxMenu.y}
          items={designerMenuItems}
          onClose={() => setCtxMenu(null)}
        />
      )}
    </div>
  );
}

function PaletteIcon({ type }: { type: ElementType }) {
  const s = 14;
  switch (type) {
    case 'heading':
      return <Heading size={s} />;
    case 'text':
      return <Type size={s} />;
    case 'property_field':
      return <Building2 size={s} />;
    case 'office_field':
      return <Briefcase size={s} />;
    case 'image':
      return <ImageIcon size={s} />;
    case 'shape':
      return <Square size={s} />;
    case 'divider':
      return <Minus size={s} />;
    case 'qr':
      return <QrCode size={s} />;
    case 'specs_table':
      return <Table2 size={s} />;
    default:
      return <Type size={s} />;
  }
}
