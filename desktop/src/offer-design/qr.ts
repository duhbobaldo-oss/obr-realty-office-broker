import QRCode from 'qrcode';

/**
 * توليد SVG حقيقي قابل للمسح من مكتبة qrcode.
 * يتضمن هامشًا أبيض (quiet zone) ولونًا صريحًا ونسبة عرض مربّعة بدون تمديد.
 */
export function qrSvgMarkup(payload: string): string | null {
  const text = String(payload || '').trim();
  if (!text) return null;
  try {
    const qr = QRCode.create(text, { errorCorrectionLevel: 'M' });
    const n = qr.modules.size;
    const margin = 2;
    const size = n + margin * 2;
    const parts: string[] = [
      `<rect width="${size}" height="${size}" fill="#ffffff"/>`
    ];
    for (let row = 0; row < n; row++) {
      for (let col = 0; col < n; col++) {
        // BitMatrix.get(x, y) — x عمود، y صف
        if (qr.modules.get(col, row)) {
          parts.push(
            `<rect x="${col + margin}" y="${row + margin}" width="1" height="1" fill="#000000"/>`
          );
        }
      }
    }
    return [
      `<svg xmlns="http://www.w3.org/2000/svg"`,
      ` viewBox="0 0 ${size} ${size}"`,
      ` width="${size}" height="${size}"`,
      ` shape-rendering="crispEdges"`,
      ` preserveAspectRatio="xMidYMid meet">`,
      parts.join(''),
      `</svg>`
    ].join('');
  } catch {
    return null;
  }
}

export function qrDataUri(payload: string): string | null {
  const svg = qrSvgMarkup(payload);
  if (!svg) return null;
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
}
