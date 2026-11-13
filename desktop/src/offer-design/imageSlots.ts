import type { PageSizeId } from './sizes';
import { getPageSize } from './sizes';
import type { DocElement, ElementStyle } from './model';

/** مقاسات مربعات الصور المناسبة للصفحة — الإطار ثابت والصورة تتناسب داخله. */
export type ImageSlotKind = 'hero' | 'secondary' | 'square' | 'banner';

export type ImageSlot = {
  kind: ImageSlotKind;
  width: number;
  height: number;
};

const MARGIN_MM = 12;

export function imageSlotForPage(pageSize: PageSizeId, kind: ImageSlotKind = 'hero'): ImageSlot {
  const page = getPageSize(pageSize);
  const innerW = Math.max(40, page.widthMm - MARGIN_MM * 2);
  const innerH = Math.max(30, page.heightMm - MARGIN_MM * 2);

  if (kind === 'hero') {
    const width = Math.min(110, innerW * 0.55);
    const height = Math.min(82, innerH * 0.32);
    return { kind, width: round1(width), height: round1(height) };
  }
  if (kind === 'secondary') {
    const width = Math.min(86, innerW * 0.42);
    const height = Math.min(64, innerH * 0.24);
    return { kind, width: round1(width), height: round1(height) };
  }
  if (kind === 'banner') {
    return { kind, width: round1(innerW), height: round1(Math.min(48, innerH * 0.18)) };
  }
  const side = Math.min(55, innerW * 0.28, innerH * 0.22);
  return { kind: 'square', width: round1(side), height: round1(side) };
}

export function clampImageFrame(
  width: number,
  height: number,
  pageSize: PageSizeId
): { width: number; height: number } {
  const page = getPageSize(pageSize);
  const maxW = page.widthMm - MARGIN_MM * 2;
  const maxH = page.heightMm - MARGIN_MM * 2;
  return {
    width: round1(Math.min(Math.max(16, width), maxW)),
    height: round1(Math.min(Math.max(12, height), maxH))
  };
}

/** إدراج صورة داخل إطار قائم دون تغيير مقاس المربع. */
export function applyImageToFrame(
  element: Pick<DocElement, 'width' | 'height' | 'style'>,
  dataUrl: string
): { content: string; width: number; height: number; style: ElementStyle } {
  return {
    content: dataUrl,
    width: element.width,
    height: element.height,
    style: {
      ...(element.style || {}),
      objectFit: 'contain',
      hideIfEmpty: false
    }
  };
}

function round1(n: number) {
  return Math.round(n * 10) / 10;
}
