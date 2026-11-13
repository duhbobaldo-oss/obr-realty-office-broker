import { invoke } from '@tauri-apps/api/core';
import { api } from './api';

export const DEFAULT_WHATSAPP_GREETING = 'السلام عليكم.';

export function clientRecordPhone(client: {
  phone?: string | null;
  whatsapp?: string | null;
  phone_secondary?: string | null;
}): string {
  return String(client.phone || client.whatsapp || client.phone_secondary || '').trim();
}

export function buildWhatsAppGreeting(officeName?: string, custom?: string): string {
  const trimmedCustom = (custom || '').trim();
  if (trimmedCustom) return trimmedCustom;
  const name = (officeName || '').trim();
  return name ? `السلام عليكم،\nمعك ${name}.` : 'السلام عليكم.';
}

function toAsciiDigits(value: string): string {
  return value.replace(/[٠-٩۰-۹]/g, (ch) => {
    const eastern = '٠١٢٣٤٥٦٧٨٩';
    const persian = '۰۱۲۳۴۵۶۷۸۹';
    const e = eastern.indexOf(ch);
    if (e >= 0) return String(e);
    const p = persian.indexOf(ch);
    return p >= 0 ? String(p) : ch;
  });
}

export function normalizeWhatsAppPhone(raw: string): { phone: string } | { error: string } {
  const source = toAsciiDigits(String(raw || '')).trim();
  if (!source) {
    return { error: 'لا يوجد رقم جوال في سجل هذا العميل' };
  }

  let digits = source.replace(/[^\d+]/g, '');
  if (digits.startsWith('+')) digits = digits.slice(1);
  if (digits.startsWith('00')) digits = digits.slice(2);

  if (digits.startsWith('05') && digits.length === 10) {
    digits = `966${digits.slice(1)}`;
  } else if (digits.startsWith('5') && digits.length === 9) {
    digits = `966${digits}`;
  } else if (digits.startsWith('9660') && digits.length === 13) {
    digits = `966${digits.slice(4)}`;
  }

  if (!/^\d{10,15}$/.test(digits) || digits.startsWith('0') || /^0+$/.test(digits)) {
    return { error: 'رقم الجوال غير صالح. استخدم جوالًا سعوديًا صحيحًا أو رقمًا دوليًا كاملًا.' };
  }

  if (digits.startsWith('966') && !/^9665\d{8}$/.test(digits)) {
    return { error: 'رقم الجوال السعودي غير صالح' };
  }

  return { phone: digits };
}

export function buildWhatsAppUrl(phone: string, text: string): string {
  const params = new URLSearchParams();
  params.set('phone', phone);
  if (text.trim()) params.set('text', text);
  return `whatsapp://send?${params.toString()}`;
}

export function isAllowedWhatsAppUrl(url: string): boolean {
  const trimmed = url.trim();
  if (!trimmed || /[\n\r\0]/.test(trimmed)) return false;
  if (!trimmed.startsWith('whatsapp://send?')) return false;
  try {
    const parsed = new URL(trimmed);
    if (parsed.protocol !== 'whatsapp:') return false;
    const phone = parsed.searchParams.get('phone') || '';
    return /^\d{10,15}$/.test(phone);
  } catch {
    return false;
  }
}

export function communicationTypeLabel(type: string, known: { value: string; label: string }[]): string {
  if (type === 'whatsapp_opened') return 'واتساب';
  return known.find((item) => item.value === type)?.label || 'تواصل';
}

export function describeWhatsAppFailure(err: unknown): string {
  const raw = typeof err === 'string' ? err : err && typeof err === 'object' && 'message' in err
    ? String((err as { message?: string }).message || '')
    : '';
  if (/واتساب|جوال|رقم|تسجيل|حافظة|PDF|ملفات/.test(raw)) return raw;
  return 'تعذر فتح واتساب. تأكد من تثبيت تطبيق واتساب على هذا الجهاز ثم أعد المحاولة.';
}

export function buildOfferShareText(input: {
  officeName?: string;
  offerName?: string;
  city?: string | null;
  district?: string | null;
  price?: unknown;
  currency?: string;
  area?: unknown;
}): string {
  const office = (input.officeName || '').trim();
  const price = input.price != null && input.price !== '' && !Number.isNaN(Number(input.price))
    ? `${Number(input.price).toLocaleString('en-US')} ${input.currency || "عملة غير محددة"}`
    : 'غير محدد';
  const area = input.area != null && input.area !== '' && !Number.isNaN(Number(input.area))
    ? `${Number(input.area)} متر مربع`
    : 'غير محددة';
  return [
    'السلام عليكم،',
    ...(office ? [`معك ${office}.`, ''] : []),
    `العرض: ${input.offerName || 'عرض عقاري'}`,
    `الحي: ${input.district || 'غير محدد'}`,
    `المدينة: ${input.city || 'غير محددة'}`,
    `السعر: ${price}`,
    `المساحة: ${area}`
  ].join('\n');
}

export async function shareOfferToWhatsApp(input: {
  clientId: string;
  phoneRaw: string;
  clientName?: string;
  propertyId: string;
  property?: {
    name?: string;
    city?: string | null;
    district?: string | null;
    price?: unknown;
  currency?: string;
    area?: unknown;
  };
}): Promise<void> {
  const parsed = normalizeWhatsAppPhone(input.phoneRaw);
  if ('error' in parsed) {
    throw new Error(parsed.error);
  }

  let officeName = '';
  let property = input.property || {};
  try {
    const [settings, full] = await Promise.all([
      api('/api/settings'),
      api(`/api/properties/${input.propertyId}`)
    ]);
    officeName = String(settings?.office_name || '');
    property = {
      name: full?.name || property.name,
      city: full?.city ?? property.city,
      district: full?.district ?? property.district,
      price: full?.price ?? property.price,
      area: full?.area ?? property.area
    };
  } catch {
    /* نكمل بالبيانات المتوفرة */
  }

  const text = buildOfferShareText({
    officeName,
    offerName: property.name,
    city: property.city,
    district: property.district,
    price: property.price,
    currency: property.currency,
    area: property.area
  });

  await invoke('open_whatsapp_chat', { phone: parsed.phone, text });

  try {
    await api(`/api/clients/${input.clientId}/communications`, {
      method: 'POST',
      body: {
        type: 'whatsapp',
        subject: 'تم فتح محادثة واتساب',
        notes: `تم فتح محادثة واتساب لعرض: ${property.name || input.propertyId}`
      }
    });
    await api(`/api/clients/${input.clientId}/link-property`, {
      method: 'POST',
      body: { property_id: input.propertyId, link_type: 'offer_share' }
    });
  } catch {
    throw new Error('تم فتح واتساب، وتعذر تسجيل الحدث في ملف العميل');
  }
}

export async function openClientWhatsAppConversation(
  clientId: string,
  phoneRaw: string,
  options?: { text?: string; subject?: string; notes?: string }
): Promise<void> {
  const parsed = normalizeWhatsAppPhone(phoneRaw);
  if ('error' in parsed) {
    throw new Error(parsed.error);
  }

  let officeName = '';
  try {
    const settings = await api('/api/settings');
    officeName = String(settings?.office_name || '');
  } catch {
    officeName = '';
  }

  const text = (options?.text || '').trim() || buildWhatsAppGreeting(officeName);
  await invoke('open_whatsapp_chat', { phone: parsed.phone, text });

  try {
    await api(`/api/clients/${clientId}/communications`, {
      method: 'POST',
      body: {
        type: 'whatsapp',
        subject: options?.subject || 'تم فتح محادثة واتساب',
        notes: options?.notes || 'تم فتح محادثة واتساب'
      }
    });
  } catch {
    throw new Error('تم فتح واتساب، وتعذر تسجيل الحدث في ملف العميل');
  }
}
