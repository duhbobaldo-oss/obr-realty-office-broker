import { t as systemT } from '../domain/locale';
import React from 'react';
import { getUiLanguage, t } from '../domain/locale';
import type { DocElement } from './model';
import { bindingLabel } from './model';
import { displayText, displayImage, specsRows, fieldValue } from './bindings';
import { qrDataUri } from './qr';

type Props = {
  element: DocElement;
  property: any | null;
  settings: any | null;
  selected?: boolean;
  editing?: boolean;
  /** داخل مساحة الضرب: يملأ الحاوية بدل absolute بالمليمتر */
  fill?: boolean;
};

export default function ElementView({ element, property, settings, selected, editing, fill }: Props) {
  if (!element.visible) return null;

  const st = element.style || {};
  const text = element.systemTextKey ? t(element.systemTextKey) : displayText(element.binding, element.content, property, settings);
  const image = displayImage(element.binding, element.content, property, settings);
  const empty = !fieldValue(element.binding, element.content, property, settings) && !image;

  if (
    st.hideIfEmpty &&
    empty &&
    (element.type === 'property_field' ||
      element.type === 'office_field' ||
      element.type === 'image' ||
      element.type === 'text' ||
      element.type === 'heading')
  ) {
    if (!editing) return null;
  }

  const base: React.CSSProperties = fill
    ? {
        position: 'relative',
        width: '100%',
        height: '100%',
        boxSizing: 'border-box',
        overflow: 'hidden',
        opacity: st.opacity == null ? 1 : st.opacity,
        outline: selected ? '1.5px solid var(--gold, #d97706)' : undefined,
        outlineOffset: selected ? 1 : undefined,
        pointerEvents: 'none'
      }
    : {
        position: 'absolute',
        left: `${element.x}mm`,
        top: `${element.y}mm`,
        width: `${element.width}mm`,
        height: `${element.height}mm`,
        transform: element.rotation ? `rotate(${element.rotation}deg)` : undefined,
        zIndex: element.zIndex,
        opacity: st.opacity == null ? 1 : st.opacity,
        boxSizing: 'border-box',
        overflow: 'hidden',
        outline: selected ? '1.5px solid var(--gold, #d97706)' : undefined,
        outlineOffset: selected ? 1 : undefined,
        pointerEvents: 'none'
      };

  if (element.type === 'shape') {
    return (
      <div
        style={{
          ...base,
          background: st.background || '#e9e0d2',
          borderRadius: st.radius != null ? st.radius : 0,
          border: st.borderWidth ? `${st.borderWidth}px solid ${st.borderColor || '#ccc'}` : undefined
        }}
      />
    );
  }

  if (element.type === 'divider') {
    return (
      <div
        style={{
          ...base,
          height: fill ? '100%' : `${Math.max(element.height, 0.4)}mm`,
          background: st.background || st.color || '#d97706',
          borderRadius: 1
        }}
      />
    );
  }

  if (element.type === 'image') {
    return (
      <div
        style={{
          ...base,
          background: st.background || '#eee5d8',
          borderRadius: st.radius != null ? st.radius : 2,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center'
        }}
      >
        {image ? (
          <img
            src={image}
            alt=""
            style={{
              width: '100%',
              height: '100%',
              maxWidth: '100%',
              maxHeight: '100%',
              objectFit: st.objectFit || 'contain',
              objectPosition: 'center',
              display: 'block'
            }}
          />
        ) : editing ? (
          <span style={{ fontSize: 10, color: '#6f7380', fontWeight: 600, pointerEvents: 'none' }}> {systemT('design.ui.120')} </span>
        ) : null}
      </div>
    );
  }

  if (element.type === 'qr') {
    const payload = text || element.content || '';
    const uri = payload ? qrDataUri(payload) : null;
    const emptyQr = !payload || !uri;
    if (st.hideIfEmpty && emptyQr && !editing) return null;
    return (
      <div
        style={{
          ...base,
          background: '#fff',
          border: `${st.borderWidth || 1}px solid ${st.borderColor || '#23365f'}`,
          display: 'grid',
          placeItems: 'center',
          padding: 2
        }}
        title={payload || systemT('design.ui.11')}
      >
        {uri ? (
          <img
            src={uri}
            alt=""
            style={{
              width: '100%',
              height: '100%',
              display: 'block',
              objectFit: 'contain',
              objectPosition: 'center',
              imageRendering: 'pixelated',
              background: '#fff'
            }}
          />
        ) : editing ? (
          <span style={{ fontSize: 9, color: '#6f7380', fontWeight: 600 }}> {systemT('design.ui.121')} </span>
        ) : null}
      </div>
    );
  }

  if (element.type === 'specs_table') {
    const rows = specsRows(property, settings);
    return (
      <div
        style={{
          ...base,
          background: st.background || '#f9f6f1',
          border: `${st.borderWidth || 1}px solid ${st.borderColor || '#e9e0d2'}`,
          borderRadius: st.radius != null ? st.radius : 4,
          padding: '2mm',
          fontSize: st.fontSize || 10,
          color: st.color || '#23365f',
          direction: getUiLanguage() === 'ar' ? 'rtl' : 'ltr'
        }}
      >
        {rows.length === 0 ? (
          editing ? <div style={{ color: '#6f7380' }}> {systemT('design.ui.122')} </div> : null
        ) : (
          <dl style={{ margin: 0, display: 'grid', gap: 3 }}>
            {rows.slice(0, 8).map((r) => (
              <div key={r.label} style={{ display: 'flex', justifyContent: 'space-between', gap: 6 }}>
                <dt style={{ fontWeight: 700, opacity: 0.7 }}>{r.label}</dt>
                <dd style={{ margin: 0, fontWeight: 600 }}>{r.value}</dd>
              </div>
            ))}
          </dl>
        )}
      </div>
    );
  }

  const namedField = element.type === 'property_field' || element.type === 'office_field';
  const nameLabel = namedField && element.binding ? bindingLabel(element.binding) : '';
  const valueText = namedField ? fieldValue(element.binding, element.content, property, settings) : text;
  const show = namedField
    ? ''
    : text
      ? text
      : editing && element.binding
        ? `${bindingLabel(element.binding)} :`
        : editing && (element.type === 'heading' || element.type === 'text')
          ? systemT('design.ui.123')
          : '';

  return (
    <div
      style={{
        ...base,
        color: st.color || '#23365f',
        background: st.background || 'transparent',
        fontSize: st.fontSize || (element.type === 'heading' ? 16 : 11),
        fontWeight: st.fontWeight || (element.type === 'heading' ? 700 : 500),
        textAlign: st.align === 'center' ? 'center' : getUiLanguage() === 'en' ? (st.align === 'left' ? 'right' : 'left') : (st.align || 'right'),
        direction: getUiLanguage() === 'ar' ? 'rtl' : 'ltr',
        lineHeight: 1.35,
        padding: '0.5mm 1mm',
        borderRadius: st.radius != null ? st.radius : 0,
        border: st.borderWidth ? `${st.borderWidth}px solid ${st.borderColor || '#ccc'}` : undefined,
        display: 'flex',
        alignItems: 'center',
        justifyContent: st.align === 'center' ? 'center' : st.align === 'left' ? 'flex-start' : 'flex-end',
        gap: '0.8mm',
        whiteSpace: 'pre-wrap',
        wordBreak: 'break-word'
      }}
    >
      {namedField ? (
        <>
          <span style={{ fontWeight: 800, opacity: valueText ? 0.85 : 0.45 }}>{nameLabel}</span>
          <span style={{ fontWeight: 800, color: st.color || '#d97706' }}>:</span>
          {valueText ? (
            <span>{valueText}</span>
          ) : (
            <span className="ods-field-empty">{editing ? systemT('design.ui.124') : ''}</span>
          )}
        </>
      ) : text ? (
        show
      ) : (
        <span className="ods-field-empty">{show}</span>
      )}
    </div>
  );
}
