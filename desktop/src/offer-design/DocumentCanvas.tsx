import { t as systemT } from '../domain/locale';
import React, { useEffect, useMemo, useRef, useState } from 'react';
import type { DocElement, DocPage, OfferDocument } from './model';
import ElementView from './ElementView';
import { getPageSize } from './sizes';

const MM_TO_PX = 96 / 25.4;

type Props = {
  doc: OfferDocument;
  page: DocPage;
  property: any | null;
  settings: any | null;
  selectedId: string | null;
  scale: number;
  editing: boolean;
  textEditId?: string | null;
  onSelect: (id: string | null) => void;
  onPatchElement: (id: string, patch: Partial<DocElement>) => void;
  onCommitDrag: () => void;
  onEditContent?: (id: string, content: string) => void;
  onTextEditId?: (id: string | null) => void;
  onElementContextMenu?: (e: React.MouseEvent, el: DocElement | null) => void;
  onInsertImage?: (id: string) => void;
};

type DragMode = 'move' | 'resize' | null;

export default function DocumentCanvas({
  doc,
  page,
  property,
  settings,
  selectedId,
  scale,
  editing,
  textEditId: textEditIdProp,
  onSelect,
  onPatchElement,
  onCommitDrag,
  onEditContent,
  onTextEditId,
  onElementContextMenu,
  onInsertImage
}: Props) {
  const size = getPageSize(doc.pageSize);
  const pageRef = useRef<HTMLDivElement>(null);
  const [textEditIdLocal, setTextEditIdLocal] = useState<string | null>(null);
  const textEditId = textEditIdProp !== undefined ? textEditIdProp : textEditIdLocal;
  const setTextEditId = (id: string | null) => {
    setTextEditIdLocal(id);
    onTextEditId?.(id);
  };
  const drag = useRef<{
    mode: DragMode;
    id: string;
    startX: number;
    startY: number;
    orig: DocElement;
    moved: boolean;
  } | null>(null);

  const elements = useMemo(
    () => [...page.elements].sort((a, b) => a.zIndex - b.zIndex),
    [page.elements]
  );

  useEffect(() => {
    const onMove = (e: PointerEvent) => {
      const d = drag.current;
      if (!d || textEditId) return;
      const dxMm = ((e.clientX - d.startX) / scale) / MM_TO_PX;
      const dyMm = ((e.clientY - d.startY) / scale) / MM_TO_PX;
      if (Math.abs(dxMm) + Math.abs(dyMm) > 0.05) d.moved = true;
      if (d.mode === 'move') {
        const x = Math.max(0, Math.min(size.widthMm - d.orig.width, d.orig.x + dxMm));
        const y = Math.max(0, Math.min(size.heightMm - d.orig.height, d.orig.y + dyMm));
        onPatchElement(d.id, { x, y });
      } else if (d.mode === 'resize') {
        const width = Math.max(8, Math.min(size.widthMm - d.orig.x, d.orig.width + dxMm));
        const height = Math.max(4, Math.min(size.heightMm - d.orig.y, d.orig.height + dyMm));
        onPatchElement(d.id, { width, height });
      }
    };
    const onUp = () => {
      const d = drag.current;
      drag.current = null;
      if (d?.moved) onCommitDrag();
    };
    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', onUp);
    return () => {
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('pointerup', onUp);
    };
  }, [onPatchElement, onCommitDrag, scale, size.widthMm, size.heightMm, textEditId]);

  const startDrag = (e: React.PointerEvent, el: DocElement, mode: DragMode) => {
    if (!editing || !mode || textEditId) return;
    if (e.button !== 0) return;
    e.stopPropagation();
    e.preventDefault();
    onSelect(el.id);
    drag.current = {
      mode,
      id: el.id,
      startX: e.clientX,
      startY: e.clientY,
      orig: { ...el },
      moved: false
    };
  };

  const canType = (el: DocElement) =>
    (el.type === 'text' || el.type === 'heading') && !el.binding;

  return (
    <div
      className="ods-doc-page"
      ref={pageRef}
      onPointerDown={() => {
        if (!textEditId) onSelect(null);
      }}
      onContextMenu={(e) => {
        if (!editing) return;
        e.preventDefault();
        if (!(e.target as HTMLElement).closest('.ods-el-hit')) {
          onSelect(null);
          onElementContextMenu?.(e, null);
        }
        e.stopPropagation();
      }}
      style={{
        width: `${size.widthMm}mm`,
        height: `${size.heightMm}mm`,
        background: doc.theme.surface || '#fff',
        color: doc.theme.ink,
        position: 'relative',
        boxShadow: editing ? '0 8px 28px rgba(20,30,50,0.18)' : 'none',
        overflow: 'hidden',
        direction: 'ltr',
        ['--od-primary' as string]: doc.theme.primary,
        ['--od-accent' as string]: doc.theme.accent
      }}
    >
      {elements.map((el) => (
        <div
          key={el.id}
          className={`ods-el-hit${selectedId === el.id ? ' is-selected' : ''}`}
          style={{
            position: 'absolute',
            left: `${el.x}mm`,
            top: `${el.y}mm`,
            width: `${el.width}mm`,
            height: `${el.height}mm`,
            zIndex: el.zIndex + 1000,
            cursor: editing ? 'move' : 'default',
            pointerEvents: editing ? 'auto' : 'none'
          }}
          onPointerDown={(e) => startDrag(e, el, 'move')}
          onContextMenu={(e) => {
            if (!editing) return;
            e.preventDefault();
            e.stopPropagation();
            drag.current = null;
            onSelect(el.id);
            onElementContextMenu?.(e, el);
          }}
          onDoubleClick={(e) => {
            if (!editing) return;
            if (el.type === 'image' && onInsertImage) {
              e.stopPropagation();
              e.preventDefault();
              drag.current = null;
              onSelect(el.id);
              onInsertImage(el.id);
              return;
            }
            if (!canType(el) || !onEditContent) return;
            e.stopPropagation();
            e.preventDefault();
            drag.current = null;
            onSelect(el.id);
            setTextEditId(el.id);
          }}
        >
          {textEditId === el.id ? (
            <textarea
              className="ods-inline-edit"
              autoFocus
              defaultValue={el.systemTextKey ? systemT(el.systemTextKey) : el.content || ''}
              aria-label={systemT('design.ui.33')}
              onPointerDown={(e) => e.stopPropagation()}
              onBlur={(e) => {
                onEditContent?.(el.id, e.currentTarget.value);
                setTextEditId(null);
              }}
              onKeyDown={(e) => {
                if (e.key === 'Escape') {
                  e.preventDefault();
                  setTextEditId(null);
                }
              }}
            />
          ) : (
            <ElementView
              element={el}
              property={property}
              settings={settings}
              selected={selectedId === el.id}
              editing={editing}
              fill
            />
          )}
          {editing && el.type === 'image' && onInsertImage && (
            <button
              type="button"
              className="ods-insert-image-btn"
              onPointerDown={(e) => e.stopPropagation()}
              onClick={(e) => {
                e.stopPropagation();
                onSelect(el.id);
                onInsertImage(el.id);
              }}
            >
               {systemT('design.ui.167')} </button>
          )}
          {editing && selectedId === el.id && textEditId !== el.id && (
            <button
              type="button"
              className="ods-resize-handle"
              aria-label={systemT('design.ui.168')}
              onPointerDown={(e) => startDrag(e, el, 'resize')}
            />
          )}
        </div>
      ))}
    </div>
  );
}
