// إدارة الثيمات — classic / archive / system + طبقة الواجهة الزجاجية
export type ThemeId = 'classic' | 'archive' | 'system';

export interface ThemeDef {
  id: ThemeId;
  name: string;
  desc: string;
  swatches: string[];
}

export const THEMES: ThemeDef[] = [
  {
    id: 'classic',
    name: 'كلاسيكي',
    desc: 'كحلي مع ذهبي كهرماني وعاجي دافئ',
    swatches: ['#23365f', '#d97706', '#f7f1e8']
  },
  {
    id: 'archive',
    name: 'أرشيف المكتب',
    desc: 'ورق كريمي، شريط خمري، ولمسات أخضر مائي — اختياري',
    swatches: ['#6B2D30', '#8C9A86', '#F5F2E9']
  },
  {
    id: 'system',
    name: 'نظامي',
    desc: 'رصاصي فاتح وأبيض هادئ — تباين خفيف مريح للعين',
    swatches: ['#5A5A5E', '#8A8A90', '#F1F1F2']
  }
];

const KEY = 'mawasib_theme';
const STYLE_PASS = 'mawasib_theme_style_v';

function migrateTheme(raw: string | null): ThemeId {
  if (!raw) return 'classic';
  if (THEMES.some((t) => t.id === raw)) return raw as ThemeId;
  // ثيمات محذوفة أو قديمة ترجع للكلاسيكي
  if (
    raw === 'royal' ||
    raw === 'burgundy' ||
    raw === 'basic' ||
    raw === 'corporate-blue' ||
    raw === 'neutral' ||
    raw === 'emerald' ||
    raw === 'calm' ||
    raw === 'charcoal' ||
    raw === 'pure' ||
    raw === 'dark'
  ) {
    return 'classic';
  }
  return 'classic';
}

export function currentTheme(): ThemeId {
  return migrateTheme(localStorage.getItem(KEY));
}

export function applyTheme(id: ThemeId) {
  const target = migrateTheme(id);
  localStorage.setItem(KEY, target);
  document.documentElement.dataset.theme = target;
  window.dispatchEvent(new CustomEvent('obr-theme-changed', { detail: target }));
}

export function initTheme() {
  // التراجع عن التحويل التلقائي إلى أرشيف المكتب — إعادة الكلاسيكي إن فُعِّل قسرًا.
  try {
    if (localStorage.getItem(STYLE_PASS) === 'archive-1') {
      if (localStorage.getItem(KEY) === 'archive') {
        localStorage.setItem(KEY, 'classic');
      }
      localStorage.setItem(STYLE_PASS, 'classic-restore-1');
    }
  } catch {
    /* تجاهل */
  }
  applyTheme(currentTheme());
  applySoftUi(isSoftUiEnabled());
}

/* ===== تجربة الواجهة الزجاجية — طبقة اختيارية قابلة للتراجع ===== */
const SOFT_UI_KEY = 'mawasib_soft_ui';

/** مفعّلة افتراضيًا؛ عطّلها من الإعدادات للرجوع الفوري. المفتاح القديم محفوظ للتوافق. */
export function isSoftUiEnabled(): boolean {
  const raw = localStorage.getItem(SOFT_UI_KEY);
  if (raw === null) return true;
  return raw === '1' || raw === 'true';
}

export function applySoftUi(enabled: boolean) {
  localStorage.setItem(SOFT_UI_KEY, enabled ? '1' : '0');
  if (enabled) {
    document.documentElement.dataset.softUi = 'on';
    document.documentElement.dataset.glassUi = 'on';
  } else {
    delete document.documentElement.dataset.softUi;
    delete document.documentElement.dataset.glassUi;
  }
}
