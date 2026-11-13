import { useEffect, useState } from 'react';
import { api } from './api';

/** صور العقار: حد أقصى 4 لكل عرض، مع تصغير حقيقي قبل الحفظ. */

export const MAX_PROPERTY_IMAGES = 4;

/** التخزين: الصور ملفات فعلية داخل files\properties والقاعدة تحتفظ بrelative_path فقط. */

const resolvedCache = new Map<string, string>();
const pendingFetches = new Map<string, Promise<string>>();

function isDirectSrc(url?: string | null): boolean {
  return (
    !url ||
    url.startsWith('data:') ||
    url.startsWith('http://') ||
    url.startsWith('https://') ||
    url.startsWith('blob:')
  );
}

/** يجلب المسار النسبي كـ data URL للعرض (بذاكرة مشتركة لكل التطبيق). */
function resolveRelative(url: string): Promise<string> {
  const cached = resolvedCache.get(url);
  if (cached) return Promise.resolve(cached);
  const inFlight = pendingFetches.get(url);
  if (inFlight) return inFlight;
  const task = api('/api/properties/images/read', {
    method: 'POST',
    body: { path: url }
  })
    .then((d: any) => {
      const dataUrl = typeof d === 'string' ? d : String(d?.data_url || '');
      if (dataUrl) resolvedCache.set(url, dataUrl);
      pendingFetches.delete(url);
      return dataUrl;
    })
    .catch(() => {
      pendingFetches.delete(url);
      return '';
    });
  pendingFetches.set(url, task);
  return task;
}

export function clearPropertyImageCache(): void {
  resolvedCache.clear();
  pendingFetches.clear();
}

/** hook لعرض أي مصدر صورة عقار: data URL/http يمر مباشرة، والمسار النسبي يُجلب مؤقتًا. */
export function usePropertyImageSrc(url?: string | null): string {
  const [src, setSrc] = useState<string>(() => (isDirectSrc(url) ? url || '' : resolvedCache.get(url || '') || ''));

  useEffect(() => {
    if (isDirectSrc(url)) {
      setSrc(url || '');
      return;
    }
    if (!url) {
      setSrc('');
      return;
    }
    let alive = true;
    resolveRelative(url).then((dataUrl) => {
      if (alive) setSrc(dataUrl);
    });
    return () => {
      alive = false;
    };
  }, [url]);

  return src;
}

/** إطار صورة بطاقة العرض العقاري — ثابت، والصورة تُصغَّر داخله ولا تكبّر البطاقة. */
export const LISTING_CARD_IMAGE = {
  aspectW: 4,
  aspectH: 3,
  /** المقاس الموصى به للملف قبل الرفع */
  widthPx: 1200,
  heightPx: 900
} as const;

export const LISTING_CARD_ASPECT = `${LISTING_CARD_IMAGE.aspectW} / ${LISTING_CARD_IMAGE.aspectH}`;

export function resizeImageDataUrl(dataUrl: string, maxEdge = LISTING_CARD_IMAGE.widthPx, quality = 0.82): Promise<string> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => {
      const scale = Math.min(1, maxEdge / Math.max(img.width, img.height));
      const width = Math.max(1, Math.round(img.width * scale));
      const height = Math.max(1, Math.round(img.height * scale));
      const canvas = document.createElement('canvas');
      canvas.width = width;
      canvas.height = height;
      const ctx = canvas.getContext('2d');
      if (!ctx) {
        resolve(dataUrl);
        return;
      }
      ctx.drawImage(img, 0, 0, width, height);
      try {
        resolve(canvas.toDataURL('image/jpeg', quality));
      } catch {
        resolve(dataUrl);
      }
    };
    img.onerror = () => reject(new Error('تعذر قراءة ملف الصورة'));
    img.src = dataUrl;
  });
}

export function readFileAsDataURL(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result || ''));
    reader.onerror = () => reject(new Error('تعذر قراءة الملف'));
    reader.readAsDataURL(file);
  });
}

export async function fileToPropertyImage(file: File): Promise<string> {
  if (!file.type.startsWith('image/')) {
    throw new Error('الملف المختار ليس صورة');
  }
  if (file.size > 25 * 1024 * 1024) {
    throw new Error('حجم الصورة أكبر من 25 ميجابايت');
  }
  const dataUrl = await readFileAsDataURL(file);
  return resizeImageDataUrl(dataUrl);
}

export async function uploadPropertyImage(propertyId: string, dataUrl: string, isMain: boolean) {
  return api(`/api/properties/${propertyId}/images`, {
    method: 'POST',
    body: {
      imageUrl: dataUrl,
      image_url: dataUrl,
      isMain,
      is_main: isMain
    }
  });
}
