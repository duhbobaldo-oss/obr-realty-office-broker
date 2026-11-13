import React from 'react';
import type { OfferDesignDoc, TemplateId } from './templates';
import { getPageSize } from './sizes';

function featuresList(text: string): string[] {
  return text
    .split(/\n|،|,/)
    .map((s) => s.trim())
    .filter(Boolean)
    .slice(0, 8);
}

function SheetFrame({
  doc,
  children,
  className
}: {
  doc: OfferDesignDoc;
  children: React.ReactNode;
  className: string;
}) {
  const size = getPageSize(doc.pageSize);
  return (
    <div
      className={`ods-sheet ${className}`}
      data-template={doc.templateId}
      style={{
        width: `${size.widthMm}mm`,
        height: `${size.heightMm}mm`,
        ['--od-primary' as string]: doc.theme.primary,
        ['--od-accent' as string]: doc.theme.accent,
        ['--od-surface' as string]: doc.theme.surface,
        ['--od-ink' as string]: doc.theme.ink,
        ['--od-muted' as string]: doc.theme.muted
      }}
    >
      {children}
    </div>
  );
}

function RoyalSheet({ doc }: { doc: OfferDesignDoc }) {
  const f = doc.fields;
  const feats = featuresList(f.featuresText);
  return (
    <SheetFrame doc={doc} className="ods-royal">
      <header className="ods-royal-head">
        <div>
          <div className="ods-kicker">{f.subtitle || 'عرض عقاري'}</div>
          <h1 className="ods-title">{f.title || 'عنوان العرض'}</h1>
        </div>
        <div className="ods-office">{f.officeName}</div>
      </header>
      <div className="ods-royal-body">
        <div className="ods-media">
          {f.imageUrl ? (
            <img src={f.imageUrl} alt="" />
          ) : (
            <div className="ods-media-placeholder">مساحة الصورة</div>
          )}
        </div>
        <aside className="ods-specs">
          <div className="ods-price">{f.price || '—'}</div>
          {f.pricePerMeter && <div className="ods-ppm">{f.pricePerMeter}</div>}
          <dl>
            <div><dt>النوع</dt><dd>{f.propertyType || '—'}</dd></div>
            <div><dt>المدينة</dt><dd>{f.city || '—'}</dd></div>
            <div><dt>الحي</dt><dd>{f.district || '—'}</dd></div>
            <div><dt>المساحة</dt><dd>{f.area || '—'}</dd></div>
          </dl>
          {feats.length > 0 && (
            <ul className="ods-feats">
              {feats.map((x) => <li key={x}>{x}</li>)}
            </ul>
          )}
        </aside>
      </div>
      {f.description && <p className="ods-desc">{f.description}</p>}
      <footer className="ods-foot">
        <span>{f.phone && `هاتف: ${f.phone}`}</span>
        <span>{f.whatsapp && `واتساب: ${f.whatsapp}`}</span>
        <span>{f.falLicense && `الترخيص / السجل التجاري: ${f.falLicense}`}</span>
      </footer>
    </SheetFrame>
  );
}

function GoldLuxury({ doc }: { doc: OfferDesignDoc }) {
  const f = doc.fields;
  const feats = featuresList(f.featuresText);
  return (
    <SheetFrame doc={doc} className="ods-gold">
      <div className="ods-gold-frame">
        <div className="ods-gold-badge">{f.subtitle || 'حصري'}</div>
        <h1 className="ods-title">{f.title || 'عنوان العرض'}</h1>
        <div className="ods-gold-media">
          {f.imageUrl ? <img src={f.imageUrl} alt="" /> : <div className="ods-media-placeholder">الصورة الرئيسية</div>}
        </div>
        <div className="ods-gold-price-row">
          <div>
            <div className="ods-label">السعر</div>
            <div className="ods-price">{f.price || '—'}</div>
          </div>
          <div>
            <div className="ods-label">المساحة</div>
            <div className="ods-strong">{f.area || '—'}</div>
          </div>
          <div>
            <div className="ods-label">الموقع</div>
            <div className="ods-strong">{[f.district, f.city].filter(Boolean).join('، ') || '—'}</div>
          </div>
        </div>
        {f.description && <p className="ods-desc">{f.description}</p>}
        {feats.length > 0 && (
          <div className="ods-gold-feats">
            {feats.map((x) => <span key={x}>{x}</span>)}
          </div>
        )}
        <footer className="ods-foot ods-gold-foot">
          <strong>{f.officeName}</strong>
          <span>{f.phone}</span>
          <span>{f.whatsapp}</span>
        </footer>
      </div>
    </SheetFrame>
  );
}

function ModernSplit({ doc }: { doc: OfferDesignDoc }) {
  const f = doc.fields;
  const feats = featuresList(f.featuresText);
  return (
    <SheetFrame doc={doc} className="ods-split">
      <div className="ods-split-media">
        {f.imageUrl ? <img src={f.imageUrl} alt="" /> : <div className="ods-media-placeholder">صورة العرض</div>}
      </div>
      <div className="ods-split-panel">
        <div className="ods-kicker">{f.propertyType || 'عقار'}</div>
        <h1 className="ods-title">{f.title || 'عنوان العرض'}</h1>
        <div className="ods-price">{f.price || '—'}</div>
        <div className="ods-split-meta">
          <span>{f.area || '—'}</span>
          <span>{f.district || '—'}</span>
          <span>{f.city || '—'}</span>
        </div>
        {f.description && <p className="ods-desc">{f.description}</p>}
        {feats.length > 0 && (
          <ul className="ods-feats">
            {feats.map((x) => <li key={x}>{x}</li>)}
          </ul>
        )}
        <div className="ods-split-contact">
          <div>{f.officeName}</div>
          <div>{f.phone}</div>
          <div>{f.whatsapp}</div>
        </div>
      </div>
    </SheetFrame>
  );
}

function EmeraldCard({ doc }: { doc: OfferDesignDoc }) {
  const f = doc.fields;
  return (
    <SheetFrame doc={doc} className="ods-emerald">
      <div className="ods-emerald-top" style={{ backgroundImage: f.imageUrl ? `url(${f.imageUrl})` : undefined }}>
        {!f.imageUrl && <span>صورة</span>}
      </div>
      <div className="ods-emerald-body">
        <div className="ods-kicker">{f.subtitle || f.propertyType || 'عرض'}</div>
        <h1 className="ods-title">{f.title || 'عنوان العرض'}</h1>
        <div className="ods-price">{f.price || '—'}</div>
        <div className="ods-emerald-grid">
          <div><span>المساحة</span><b>{f.area || '—'}</b></div>
          <div><span>الحي</span><b>{f.district || '—'}</b></div>
          <div><span>المدينة</span><b>{f.city || '—'}</b></div>
          <div><span>سعر المتر</span><b>{f.pricePerMeter || '—'}</b></div>
        </div>
        {f.description && <p className="ods-desc">{f.description}</p>}
        <footer className="ods-foot">
          <strong>{f.officeName}</strong>
          <span>{f.phone}</span>
          <span>{f.whatsapp}</span>
        </footer>
      </div>
    </SheetFrame>
  );
}

const RENDERERS: Record<TemplateId, React.FC<{ doc: OfferDesignDoc }>> = {
  'royal-sheet': RoyalSheet,
  'gold-luxury': GoldLuxury,
  'modern-split': ModernSplit,
  'emerald-card': EmeraldCard
};

export default function OfferSheet({ doc }: { doc: OfferDesignDoc }) {
  const View = RENDERERS[doc.templateId] || RoyalSheet;
  return <View doc={doc} />;
}
