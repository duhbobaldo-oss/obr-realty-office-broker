import { t as systemT } from '../domain/locale';
/** مقاسات PDF القياسية لتصميم العروض */

export type PageSizeId = 'a4-portrait' | 'a4-landscape' | 'a5-portrait' | 'flyer';

export interface PageSizeDef {
  id: PageSizeId;
  label: string;
  /** العرض بالمليمتر */
  widthMm: number;
  /** الارتفاع بالمليمتر */
  heightMm: number;
  cssSize: string;
}

export const PAGE_SIZES: PageSizeDef[] = [
  { id: 'a4-portrait', get label() { return systemT('design.ui.149'); }, widthMm: 210, heightMm: 297, cssSize: 'A4 portrait' },
  { id: 'a4-landscape', get label() { return systemT('design.ui.150'); }, widthMm: 297, heightMm: 210, cssSize: 'A4 landscape' },
  { id: 'a5-portrait', get label() { return systemT('design.ui.151'); }, widthMm: 148, heightMm: 210, cssSize: 'A5 portrait' },
  { id: 'flyer', get label() { return systemT('design.ui.152'); }, widthMm: 120, heightMm: 180, cssSize: '120mm 180mm' }
];

export function getPageSize(id: PageSizeId): PageSizeDef {
  return PAGE_SIZES.find((s) => s.id === id) || PAGE_SIZES[0];
}
