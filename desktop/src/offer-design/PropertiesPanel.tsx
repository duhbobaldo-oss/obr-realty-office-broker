import { t as systemT } from '../domain/locale';
import { t } from '../domain/locale';
import React, { useState } from 'react';
import type { BindingKey, DocElement, ElementStyle } from './model';
import { BINDING_CATALOG, bindingLabel } from './model';
import { applyImageToFrame } from './imageSlots';
import { fileToPropertyImage } from '../propertyImages';

type Props = {
  element: DocElement | null;
  onChange: (patch: Partial<DocElement>) => void;
  onStyle: (patch: Partial<ElementStyle>) => void;
  onDelete: () => void;
  onLayer: (op: 'front' | 'back' | 'forward' | 'backward') => void;
};

export default function PropertiesPanel({ element, onChange, onStyle, onDelete, onLayer }: Props) {
  const [fileError, setFileError] = useState('');
  if (!element) {
    return (
      <div className="ods-props-empty">
        <p> {systemT('design.ui.65')} </p>
      </div>
    );
  }

  const isTextish =
    element.type === 'text' ||
    element.type === 'heading' ||
    element.type === 'property_field' ||
    element.type === 'office_field';
  const isBound =
    element.type === 'property_field' ||
    element.type === 'office_field' ||
    element.type === 'image' ||
    element.type === 'qr';
  const isShape = element.type === 'shape' || element.type === 'divider';
  const isImage = element.type === 'image';

  const bindingOptions = BINDING_CATALOG.filter((b) => {
    if (element.type === 'image') return b.kind === 'image';
    if (element.type === 'office_field') return b.group === 'office' && b.kind === 'text';
    if (element.type === 'property_field') return b.group === 'property' && b.kind === 'text';
    if (element.type === 'qr') return b.kind === 'text';
    return b.kind === 'text';
  });

  return (
    <div className="ods-props">
      <div className="ods-props-head">
        <strong>{typeLabel(element.type)}</strong>
        <button type="button" className="btn btn-sm btn-danger" onClick={onDelete}>
           {systemT('design.ui.42')} </button>
      </div>

      {isTextish && !element.binding && (
        <section>
          <h4> {systemT('design.ui.66')} </h4>
          <label className="ods-props-block">
             {systemT('design.ui.67')} <textarea
              className="form-input"
              rows={5}
              value={element.systemTextKey ? t(element.systemTextKey) : element.content || ''}
              onChange={(e) => onChange({ content: e.target.value, systemTextKey: undefined })}
            />
          </label>
          <p className="ods-props-hint"> {systemT('design.ui.68')} </p>
          <div className="ods-props-grid">
            <label>
               {systemT('design.ui.69')} <input
                type="number"
                className="form-input"
                value={element.style.fontSize || 12}
                onChange={(e) => onStyle({ fontSize: Number(e.target.value) || 12 })}
              />
            </label>
            <label>
               {systemT('design.ui.70')} <select
                className="form-input"
                value={String(element.style.fontWeight || 500)}
                onChange={(e) => onStyle({ fontWeight: Number(e.target.value) })}
              >
                <option value="400"> {systemT('design.ui.71')} </option>
                <option value="600"> {systemT('design.ui.72')} </option>
                <option value="700"> {systemT('design.ui.73')} </option>
              </select>
            </label>
            <label>
               {systemT('design.ui.74')} <select
                className="form-input"
                value={element.style.align || 'right'}
                onChange={(e) => onStyle({ align: e.target.value as ElementStyle['align'] })}
              >
                <option value="right"> {systemT('design.ui.75')} </option>
                <option value="center"> {systemT('design.ui.76')} </option>
                <option value="left"> {systemT('design.ui.77')} </option>
              </select>
            </label>
            <label>
               {systemT('design.ui.78')} <input
                type="color"
                className="form-input"
                value={element.style.color || '#23365f'}
                onChange={(e) => onStyle({ color: e.target.value })}
              />
            </label>
          </div>
        </section>
      )}

      {isTextish && element.binding && (
        <section>
          <h4> {systemT('design.ui.66')} </h4>
          <p className="ods-props-hint">
             {systemT('design.ui.79')} {bindingLabel(element.binding)} {systemT('design.ui.80')} </p>
          <div className="ods-props-grid">
            <label>
               {systemT('design.ui.69')} <input
                type="number"
                className="form-input"
                value={element.style.fontSize || 12}
                onChange={(e) => onStyle({ fontSize: Number(e.target.value) || 12 })}
              />
            </label>
            <label>
               {systemT('design.ui.70')} <select
                className="form-input"
                value={String(element.style.fontWeight || 500)}
                onChange={(e) => onStyle({ fontWeight: Number(e.target.value) })}
              >
                <option value="400"> {systemT('design.ui.71')} </option>
                <option value="600"> {systemT('design.ui.72')} </option>
                <option value="700"> {systemT('design.ui.73')} </option>
              </select>
            </label>
            <label>
               {systemT('design.ui.74')} <select
                className="form-input"
                value={element.style.align || 'right'}
                onChange={(e) => onStyle({ align: e.target.value as ElementStyle['align'] })}
              >
                <option value="right"> {systemT('design.ui.75')} </option>
                <option value="center"> {systemT('design.ui.76')} </option>
                <option value="left"> {systemT('design.ui.77')} </option>
              </select>
            </label>
            <label>
               {systemT('design.ui.78')} <input
                type="color"
                className="form-input"
                value={element.style.color || '#23365f'}
                onChange={(e) => onStyle({ color: e.target.value })}
              />
            </label>
          </div>
        </section>
      )}

      {(isShape || isImage || isTextish) && (
        <section>
          <h4> {systemT('design.ui.81')} </h4>
          <div className="ods-props-grid">
            {(isShape || isImage) && (
              <label>
                 {systemT('design.ui.82')} <input
                  type="color"
                  className="form-input"
                  value={element.style.background || '#e9e0d2'}
                  onChange={(e) => onStyle({ background: e.target.value })}
                />
              </label>
            )}
            <label>
               {systemT('design.ui.83')} <input
                type="number"
                className="form-input"
                value={element.style.borderWidth || 0}
                onChange={(e) => onStyle({ borderWidth: Number(e.target.value) || 0 })}
              />
            </label>
            <label>
               {systemT('design.ui.84')} <input
                type="color"
                className="form-input"
                value={element.style.borderColor || '#cccccc'}
                onChange={(e) => onStyle({ borderColor: e.target.value })}
              />
            </label>
            <label>
               {systemT('design.ui.85')} <input
                type="number"
                className="form-input"
                value={element.style.radius || 0}
                onChange={(e) => onStyle({ radius: Number(e.target.value) || 0 })}
              />
            </label>
            <label>
               {systemT('design.ui.86')} <input
                type="number"
                min={0}
                max={1}
                step={0.05}
                className="form-input"
                value={element.style.opacity ?? 1}
                onChange={(e) => onStyle({ opacity: Math.min(1, Math.max(0, Number(e.target.value))) })}
              />
            </label>
          </div>
        </section>
      )}

      {isImage && (
        <section>
          <h4> {systemT('design.ui.30')} </h4>
          <label className="ods-props-block">
             {systemT('design.ui.87')} <input
              type="file"
              accept="image/*"
              className="form-input"
              onChange={async (e) => {
                const file = e.target.files?.[0];
                e.target.value = '';
                if (!file) return;
                try {
                  const data = await fileToPropertyImage(file);
                  setFileError('');
                  onChange(applyImageToFrame(element, data));
                } catch {
                  setFileError(systemT('design.ui.88'));
                }
              }}
            />
          </label>
          <p className="ods-props-hint">
             {systemT('design.ui.89')} </p>
          {element.content ? (
            <button
              type="button"
              className="btn btn-sm btn-outline"
              onClick={() => onChange({ content: '' })}
            >
               {systemT('design.ui.90')} </button>
          ) : null}
          {fileError ? <p className="ods-props-hint" style={{ color: 'var(--danger)' }}>{fileError}</p> : null}
        </section>
      )}

      {(element.type === 'property_field' ||
        element.type === 'office_field' ||
        element.type === 'qr' ||
        ((element.type === 'text' || element.type === 'heading') && element.binding)) && (
        <section>
          <h4> {systemT('design.ui.91')} </h4>
          <label className="ods-props-block">
            {element.binding ? `${bindingLabel(element.binding)} :` : systemT('design.ui.92')}
            <input
              className="form-input"
              value={element.systemTextKey ? t(element.systemTextKey) : element.content || ''}
              onChange={(e) => onChange({ content: e.target.value, systemTextKey: undefined })}
              placeholder={systemT('design.ui.50')}
            />
          </label>
          <p className="ods-props-hint">
             {systemT('design.ui.93')} {element.binding ? bindingLabel(element.binding) : systemT('design.ui.94')}  {systemT('design.ui.95')} {element.binding ? systemT('design.ui.96') : ''}
          </p>
        </section>
      )}

      {element.type === 'specs_table' && (
        <section>
          <h4> {systemT('design.ui.97')} </h4>
          <p className="ods-props-hint">
             {systemT('design.ui.98')} </p>
        </section>
      )}

      {isBound && (
        <section>
          <h4> {systemT('design.ui.99')} </h4>
          <label className="ods-props-block">
             {systemT('design.ui.100')} <select
              className="form-input"
              value={element.binding || ''}
              onChange={(e) =>
                onChange({ binding: (e.target.value || null) as BindingKey | null })
              }
            >
              <option value=""> {systemT('design.ui.101')} </option>
              {bindingOptions.map((b) => (
                <option key={b.key} value={b.key}>
                  {b.label}
                </option>
              ))}
            </select>
          </label>
          <label className="ods-check">
            <input
              type="checkbox"
              checked={!!element.style.hideIfEmpty}
              onChange={(e) => onStyle({ hideIfEmpty: e.target.checked })}
            />
             {systemT('design.ui.102')} </label>
        </section>
      )}

      {element.type === 'qr' && (
        <section>
          <h4> {systemT('design.ui.11')} </h4>
          <p className="ods-props-hint">
             {systemT('design.ui.103')} </p>
          <label className="ods-props-block">
             {systemT('design.ui.104')} <input
              className="form-input"
              value={element.systemTextKey ? t(element.systemTextKey) : element.content || ''}
              onChange={(e) => onChange({ content: e.target.value, systemTextKey: undefined })}
              placeholder={systemT('design.ui.105')}
              dir="ltr"
            />
          </label>
        </section>
      )}

      <section>
        <h4> {systemT('design.ui.106')} </h4>
        <div className="ods-props-grid">
          <label>
             {systemT('design.ui.107')} <input
              type="number"
              className="form-input"
              value={round1(element.x)}
              onChange={(e) => onChange({ x: Number(e.target.value) || 0 })}
            />
          </label>
          <label>
             {systemT('design.ui.108')} <input
              type="number"
              className="form-input"
              value={round1(element.y)}
              onChange={(e) => onChange({ y: Number(e.target.value) || 0 })}
            />
          </label>
          <label>
             {systemT('design.ui.109')} <input
              type="number"
              className="form-input"
              value={round1(element.width)}
              onChange={(e) => onChange({ width: Math.max(4, Number(e.target.value) || 4) })}
            />
          </label>
          <label>
             {systemT('design.ui.110')} <input
              type="number"
              className="form-input"
              value={round1(element.height)}
              onChange={(e) => onChange({ height: Math.max(2, Number(e.target.value) || 2) })}
            />
          </label>
        </div>
      </section>

      <section>
        <h4> {systemT('design.ui.111')} </h4>
        <div className="ods-layer-row">
          <button type="button" className="btn btn-sm btn-outline" onClick={() => onLayer('front')}>
             {systemT('design.ui.112')} </button>
          <button type="button" className="btn btn-sm btn-outline" onClick={() => onLayer('forward')}>
             {systemT('design.ui.113')} </button>
          <button type="button" className="btn btn-sm btn-outline" onClick={() => onLayer('backward')}>
             {systemT('design.ui.114')} </button>
          <button type="button" className="btn btn-sm btn-outline" onClick={() => onLayer('back')}>
             {systemT('design.ui.115')} </button>
        </div>
      </section>
    </div>
  );
}

function round1(n: number) {
  return Math.round(n * 10) / 10;
}

function typeLabel(t: DocElement['type']): string {
  switch (t) {
    case 'heading':
      return systemT('design.ui.1');
    case 'text':
      return systemT('design.ui.17');
    case 'property_field':
      return systemT('design.ui.116');
    case 'office_field':
      return systemT('design.ui.117');
    case 'image':
      return systemT('design.ui.118');
    case 'shape':
      return systemT('design.ui.8');
    case 'divider':
      return systemT('design.ui.9');
    case 'qr':
      return systemT('design.ui.11');
    case 'specs_table':
      return systemT('design.ui.10');
    default:
      return systemT('design.ui.119');
  }
}
