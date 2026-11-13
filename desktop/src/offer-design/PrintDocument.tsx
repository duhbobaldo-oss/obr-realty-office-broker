import React from 'react';
import type { OfferDocument } from './model';
import { sortedPages } from './model';
import ElementView from './ElementView';
import { getPageSize } from './sizes';

/** نفس مصدر العناصر للمعاينة والطباعة/PDF — بدون أدوات التحرير */
export default function PrintDocument({
  doc,
  property,
  settings
}: {
  doc: OfferDocument;
  property: any | null;
  settings: any | null;
}) {
  const size = getPageSize(doc.pageSize);
  const pages = sortedPages(doc);

  return (
    <div className="ods-print-document" id="ods-print-root">
      {pages.map((page, idx) => (
        <div
          key={page.id}
          className="ods-print-page"
          style={{
            width: `${size.widthMm}mm`,
            height: `${size.heightMm}mm`,
            background: doc.theme.surface || '#fff',
            color: doc.theme.ink,
            position: 'relative',
            overflow: 'hidden',
            pageBreakAfter: idx < pages.length - 1 ? 'always' : 'auto',
            breakAfter: idx < pages.length - 1 ? 'page' : 'auto',
            ['--od-primary' as string]: doc.theme.primary,
            ['--od-accent' as string]: doc.theme.accent
          }}
        >
          {[...page.elements]
            .sort((a, b) => a.zIndex - b.zIndex)
            .map((el) => (
              <ElementView
                key={el.id}
                element={el}
                property={property}
                settings={settings}
                editing={false}
              />
            ))}
        </div>
      ))}
    </div>
  );
}
