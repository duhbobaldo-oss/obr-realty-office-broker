import { getCurrentWindow } from '@tauri-apps/api/window';

const DEFAULT_TITLE = 'مكتب الوسيط العقاري - OBR';
const STORAGE_KEY = 'mawasib_office';
const LEGACY_DEFAULT_TITLE = 'مكتب الوسيط العقاري';
const LEGACY_TITLES = ['Realty Office', LEGACY_DEFAULT_TITLE];

export function getAppTitle(): string {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (!stored || LEGACY_TITLES.includes(stored)) return DEFAULT_TITLE;
    return stored;
  } catch {
    return DEFAULT_TITLE;
  }
}

/** يحدّث عنوان النافذة + التخزين المحلي + حدث للواجهة (Sidebar/Header). */
export function setAppTitle(name?: string | null) {
  const raw = (name && name.trim()) || DEFAULT_TITLE;
  const title = LEGACY_TITLES.includes(raw) ? DEFAULT_TITLE : raw;
  try {
    localStorage.setItem(STORAGE_KEY, title);
  } catch {
    /* تجاهل */
  }
  try {
    window.dispatchEvent(new CustomEvent('mawasib:title', { detail: title }));
  } catch {
    /* تجاهل */
  }
  try {
    void getCurrentWindow().setTitle(title);
  } catch {
    /* خارج Tauri أو نافذة غير جاهزة */
  }
}

/** يبث تحديث إعدادات المكتب للواجهات المشتركة (اسم، فال، هاتف...). */
export function broadcastOfficeSettings(settings: {
  country?: string;
  broker_license_name?: string | null;
  office_name?: string | null;
  fal_license_number?: string | null;
  fal_license_type?: string | null;
  office_phone?: string | null;
  office_whatsapp?: string | null;
}) {
  setAppTitle(settings.office_name);
  try {
    window.dispatchEvent(new CustomEvent('mawasib:office-settings', { detail: settings }));
  } catch {
    /* تجاهل */
  }
}
