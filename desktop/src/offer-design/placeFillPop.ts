/** موضع مربع التعبئة تحت العنصر وداخل حدود صفحة القالب. */
export type FillPopInput = {
  elLeft: number;
  elTop: number;
  elWidth: number;
  elHeight: number;
  minLeft: number;
  maxRight: number;
  minTop: number;
  maxBottom: number;
  minWidth?: number;
  popHeight?: number;
  gap?: number;
};

export type FillPopBox = { left: number; top: number; width: number };

export function placeFillPop(p: FillPopInput): FillPopBox {
  const gap = p.gap ?? 8;
  const popH = p.popHeight ?? 48;
  const minW = p.minWidth ?? 220;
  const boundW = Math.max(0, p.maxRight - p.minLeft);
  const width = Math.min(boundW, Math.max(p.elWidth, minW));

  let left = p.elLeft;
  if (left + width > p.maxRight) left = p.maxRight - width;
  if (left < p.minLeft) left = p.minLeft;

  const below = p.elTop + p.elHeight + gap;
  const above = p.elTop - popH - gap;
  let top = below;
  if (below + popH > p.maxBottom && above >= p.minTop) top = above;
  if (top + popH > p.maxBottom) top = Math.max(p.minTop, p.maxBottom - popH);
  if (top < p.minTop) top = p.minTop;

  return { left, top, width };
}
