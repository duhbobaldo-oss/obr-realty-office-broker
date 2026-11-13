import { t as systemT } from '../domain/locale';
import { t } from '../domain/locale';
import type { OfferDocument, DocElement, DocPage } from './model';
import { emptyDocument, createElement, newId, defaultTheme } from './model';
import type { PageSizeId } from './sizes';
import { getPageSize } from './sizes';

export interface DocTemplateDef {
  id: string;
  name: string;
  description: string;
  pageSize: PageSizeId;
  build: () => OfferDocument;
}

type Theme = ReturnType<typeof defaultTheme>;

// لوحة ألوان محايدة محسّنة — صور العقار هي البطل
const NAVY = '#1c2e52';
const NAVY_DARK = '#13203e';
const GOLD = '#b98a2f';
const GOLD_DEEP = '#9a7125';
const GREEN = '#0e5e4a';
const GREEN_DARK = '#094536';
const CHARCOAL = '#22283a';
const WHITE = '#ffffff';
const OFFWHITE = '#f8f9fb';
const INK = '#232936';
const MUTED = '#6b7280';
const BORDER = '#e5e7eb';
const BORDER_GOLD = '#d9c08a';

function assemble(
  id: string,
  title: string,
  pageSize: PageSizeId,
  theme: Theme,
  elements: DocElement[]
): OfferDocument {
  return emptyDocument({
    title,
    pageSize,
    templateOrigin: id,
    titleKey: `template.${id}`,
    theme,
    pages: [{ id: newId('page'), name: systemT('design.ui.147'), systemName: true, order: 0, elements }]
  });
}

/** شريط التواصل والرخصة — سطر خفيف في الأسفل (التزام فال) */
function footer(widthMm: number, heightMm: number, theme: Theme, dark = false, inset = 10): DocElement[] {
  const y = heightMm - 16;
  const inkCol = dark ? 'rgba(255,255,255,0.92)' : theme.ink;
  const mutCol = dark ? 'rgba(255,255,255,0.72)' : theme.muted;
  const third = (widthMm - inset * 2) / 3;
  return [
    createElement('divider', {
      x: inset, y: y - 4, width: widthMm - inset * 2, height: 0.4, zIndex: 6,
      style: { background: dark ? 'rgba(255,255,255,0.35)' : (theme.accent === GOLD ? BORDER : BORDER) }
    }),
    createElement('office_field', {
      x: inset, y, width: third, height: 8, binding: 'office.name', zIndex: 6,
      style: { fontSize: 7, color: inkCol, align: 'right', fontWeight: 700 }
    }),
    createElement('office_field', {
      x: inset + third, y, width: third, height: 8, binding: 'office.phone', zIndex: 6,
      style: { fontSize: 7, color: mutCol, align: 'center', hideIfEmpty: true }
    }),
    createElement('office_field', {
      x: inset + third * 2, y, width: third, height: 8, binding: 'office.falLicense', zIndex: 6,
      style: { fontSize: 7, color: mutCol, align: 'left', hideIfEmpty: true }
    })
  ];
}

/** شريط حقول افتراضي — صف واحد من الحقول المهمة */
function statRow(
  y: number, inset: number, widthMm: number, zIndex: number,
  bindings: (BindingSpec | null)[],
  opts?: { boxBg?: string; boxBorder?: string; ink?: string; radius?: number }
): DocElement[] {
  const boxW = widthMm / 3;
  const els: DocElement[] = [];
  bindings.slice(0, 3).forEach((spec, i) => {
    const x = inset + i * (boxW - 2);
    if (opts?.boxBg) {
      els.push(createElement('shape', {
        x, y, width: boxW - 2, height: 18, zIndex,
        style: { background: opts.boxBg, borderColor: opts.boxBorder, borderWidth: 0.6, radius: opts.radius ?? 8 }
      }));
    }
    if (spec) {
      els.push(createElement('property_field', {
        x: x + 3, y: y + 3, width: boxW - 8, height: 12, binding: spec.binding, zIndex: zIndex + 1,
        style: { fontSize: 9, fontWeight: 700, color: spec.color ?? opts?.ink ?? INK, align: 'center', hideIfEmpty: true }
      }));
    }
  });
  return els;
}

type BindingSpec = {
  binding: 'property.price' | 'property.area' | 'property.city' | 'property.district' | 'property.propertyType' | 'property.purpose' | 'property.frontage' | 'property.streetWidth' | 'property.pricePerMeter';
  color?: string;
};

/** ═══ 1) بروشور الهيرو — صورة تملأ الأعلى مع العنوان فوقها ═══ */
function heroBrochure(): OfferDocument {
  const pageSize: PageSizeId = 'a4-portrait';
  const { widthMm, heightMm } = getPageSize(pageSize);
  const theme = { ...defaultTheme(), primary: NAVY, accent: GOLD, surface: OFFWHITE };
  const heroH = 168;
  const els: DocElement[] = [
    // البطل: صورة العقار تملأ العرض
    createElement('image', {
      x: 0, y: 0, width: widthMm, height: heroH, binding: 'property.image', zIndex: 0,
      style: { objectFit: 'cover', background: '#e8eaf0', hideIfEmpty: false }
    }),
    // تعتيم سفلي تدريجي (بلاطة واحدة) لقراءة العنوان
    createElement('shape', {
      x: 0, y: heroH - 64, width: widthMm, height: 64, zIndex: 1,
      style: { background: 'rgba(19,32,62,0.55)', radius: 0 }
    }),
    // شعار المكتب صغير فوق الصورة
    createElement('office_field', {
      x: 8, y: 6, width: 70, height: 8, binding: 'office.name', zIndex: 2,
      style: { fontSize: 7, color: 'rgba(255,255,255,0.95)', align: 'right', fontWeight: 700, hideIfEmpty: true }
    }),
    // العنوان فوق الصورة
    createElement('heading', {
      x: 12, y: heroH - 56, width: widthMm - 24, height: 22, binding: 'property.title', zIndex: 2,
      style: { fontSize: 19, fontWeight: 700, color: WHITE, align: 'right', hideIfEmpty: true }
    }),
    createElement('property_field', {
      x: 12, y: heroH - 32, width: widthMm - 24, height: 12, binding: 'property.purpose', zIndex: 2,
      style: { fontSize: 9, fontWeight: 700, color: '#f0d9a8', align: 'right', hideIfEmpty: true }
    }),
    // شريط السعر تحت الصورة مباشرة
    createElement('shape', {
      x: 10, y: 176, width: 96, height: 22, zIndex: 2,
      style: { background: NAVY, radius: 10 }
    }),
    createElement('property_field', {
      x: 14, y: 180, width: 88, height: 14, binding: 'property.price', zIndex: 3,
      style: { fontSize: 12, fontWeight: 700, color: GOLD, align: 'center', hideIfEmpty: true }
    }),
    // الموقع بجوار السعر
    createElement('property_field', {
      x: 112, y: 181, width: 52, height: 12, binding: 'property.city', zIndex: 3,
      style: { fontSize: 9, fontWeight: 700, color: NAVY, align: 'right', hideIfEmpty: true }
    }),
    createElement('property_field', {
      x: 166, y: 181, width: 34, height: 12, binding: 'property.propertyType', zIndex: 3,
      style: { fontSize: 9, color: MUTED, align: 'right', hideIfEmpty: true }
    }),
    // خط ذهبي فاصل
    createElement('divider', {
      x: 12, y: 206, width: 40, height: 1, zIndex: 3,
      style: { background: GOLD }
    }),
    // الوصف — 60 كلمة
    createElement('text', {
      x: 12, y: 214, width: widthMm - 24, height: 40, binding: 'property.description', zIndex: 3,
      style: { fontSize: 9.5, color: INK, align: 'right', hideIfEmpty: true }
    }),
    // صف الميزات المهمة فقط: المساحة والحي والواجهة
    ...statRow(262, 12, widthMm - 24, 3, [
      { binding: 'property.area', color: NAVY },
      { binding: 'property.district', color: NAVY },
      { binding: 'property.frontage', color: NAVY }
    ], { boxBg: NAVY, boxBorder: 'transparent', ink: WHITE, radius: 8 }),
    ...footer(widthMm, heightMm, theme)
  ];
  return assemble('royal-sheet', t('template.royal-sheet'), pageSize, theme, els);
}

/** ═══ 2) الملف الاستثماري — تنفيذي أبيض بمؤشرات حديثة ═══ */
function investmentBrief(): OfferDocument {
  const pageSize: PageSizeId = 'a4-portrait';
  const { widthMm, heightMm } = getPageSize(pageSize);
  const theme = { ...defaultTheme(), primary: NAVY, accent: GOLD, surface: WHITE };
  const els: DocElement[] = [
    createElement('image', {
      x: 10, y: 10, width: widthMm - 20, height: 122, binding: 'property.image', zIndex: 0,
      style: { objectFit: 'cover', background: '#e8eaf0', radius: 8, hideIfEmpty: false }
    }),
    createElement('office_field', {
      x: 12, y: 138, width: widthMm - 24, height: 8, binding: 'office.name', zIndex: 2,
      style: { fontSize: 7, color: MUTED, align: 'right', fontWeight: 700, hideIfEmpty: true }
    }),
    createElement('heading', {
      x: 12, y: 150, width: widthMm - 24, height: 20, binding: 'property.title', zIndex: 2,
      style: { fontSize: 20, fontWeight: 700, color: NAVY, align: 'right', hideIfEmpty: true }
    }),
    // السعر وسعر المتر — بند بارز
    createElement('shape', {
      x: 12, y: 176, width: 104, height: 24, zIndex: 2,
      style: { background: '#fdf6e7', borderColor: BORDER_GOLD, borderWidth: 0.8, radius: 10 }
    }),
    createElement('property_field', {
      x: 16, y: 181, width: 96, height: 14, binding: 'property.price', zIndex: 3,
      style: { fontSize: 13, fontWeight: 700, color: GOLD_DEEP, align: 'center', hideIfEmpty: true }
    }),
    createElement('shape', {
      x: 120, y: 176, width: 80, height: 24, zIndex: 2,
      style: { background: '#eef2f9', borderColor: BORDER, borderWidth: 0.8, radius: 10 }
    }),
    createElement('property_field', {
      x: 124, y: 181, width: 72, height: 14, binding: 'property.pricePerMeter', zIndex: 3,
      style: { fontSize: 11, fontWeight: 700, color: NAVY, align: 'center', hideIfEmpty: true }
    }),
    // مؤشرات استثمارية: أربعة tiles
    ...[
      { binding: 'property.area' as const },
      { binding: 'property.propertyType' as const },
      { binding: 'property.purpose' as const },
      { binding: 'property.frontage' as const }
    ].flatMap((spec, i): DocElement[] => {
      const tileW = (widthMm - 24 - 3 * 4) / 4;
      const x = 12 + i * (tileW + 4);
      return [
        createElement('shape', {
          x, y: 210, width: tileW, height: 26, zIndex: 2,
          style: { background: OFFWHITE, borderColor: BORDER, borderWidth: 0.6, radius: 8 }
        }),
        createElement('property_field', {
          x: x + 2, y: 218, width: tileW - 4, height: 12, binding: spec.binding, zIndex: 3,
          style: { fontSize: 9, fontWeight: 700, color: NAVY, align: 'center', hideIfEmpty: true }
        })
      ];
    }),
    // الوصف
    createElement('text', {
      x: 12, y: 248, width: widthMm - 24, height: 34, binding: 'property.description', zIndex: 3,
      style: { fontSize: 9.5, color: MUTED, align: 'right', hideIfEmpty: true }
    }),
    ...footer(widthMm, heightMm, theme)
  ];
  return assemble('investment-brief', t('template.investment-brief'), pageSize, theme, els);
}

/** ═══ 3) الفاخر الذهبي — full-bleed مع إطار ذهبي ═══ */
function goldBrochure(): OfferDocument {
  const pageSize: PageSizeId = 'a4-portrait';
  const { widthMm, heightMm } = getPageSize(pageSize);
  const theme = { ...defaultTheme(), primary: NAVY_DARK, accent: GOLD, surface: '#fffdf8' };
  const els: DocElement[] = [
    // إطار ذهبي خارجي
    createElement('shape', {
      x: 5, y: 5, width: widthMm - 10, height: heightMm - 10, zIndex: 0,
      style: { background: WHITE, borderColor: BORDER_GOLD, borderWidth: 1, radius: 2 }
    }),
    // البطل: صورة full-bleed داخل الإطار
    createElement('image', {
      x: 12, y: 12, width: widthMm - 24, height: 148, binding: 'property.image', zIndex: 1,
      style: { objectFit: 'cover', background: '#e9e5da', hideIfEmpty: false }
    }),
    createElement('office_field', {
      x: 16, y: 16, width: 76, height: 8, binding: 'office.name', zIndex: 3,
      style: { fontSize: 7, color: WHITE, align: 'right', fontWeight: 700, hideIfEmpty: true }
    }),
    // لوسم اللوحة السفلية
    createElement('heading', {
      x: 16, y: 170, width: widthMm - 32, height: 18, binding: 'property.title', zIndex: 2,
      style: { fontSize: 18, fontWeight: 700, color: NAVY_DARK, align: 'center', hideIfEmpty: true }
    }),
    // السعر — كبير بالوسط
    createElement('property_field', {
      x: 16, y: 192, width: widthMm - 32, height: 14, binding: 'property.price', zIndex: 2,
      style: { fontSize: 17, fontWeight: 700, color: GOLD_DEEP, align: 'center', hideIfEmpty: true }
    }),
    // الموقع سطر واحد
    createElement('property_field', {
      x: 16, y: 210, width: widthMm - 32, height: 10, binding: 'property.city', zIndex: 2,
      style: { fontSize: 9, fontWeight: 700, color: NAVY_DARK, align: 'center', hideIfEmpty: true }
    }),
    createElement('divider', {
      x: widthMm / 2 - 24, y: 224, width: 48, height: 0.8, zIndex: 2,
      style: { background: BORDER_GOLD }
    }),
    // الوصف قصير
    createElement('text', {
      x: 24, y: 234, width: widthMm - 48, height: 28, binding: 'property.description', zIndex: 2,
      style: { fontSize: 9, color: MUTED, align: 'center', hideIfEmpty: true }
    }),
    // الميزات: مساحة + نوع بالوسط
    ...statRow(268, 16, widthMm - 32, 2, [
      { binding: 'property.area', color: NAVY_DARK },
      { binding: 'property.district', color: NAVY_DARK },
      { binding: 'property.propertyType', color: NAVY_DARK }
    ], { boxBg: '#f7f2e6', boxBorder: BORDER_GOLD, radius: 8 }),
    ...footer(widthMm, heightMm, theme, false, 16)
  ];
  return assemble('gold-luxury', t('template.gold-luxury'), pageSize, theme, els);
}

/** ═══ 4) بطاقة الفرصة — A5 هوية خضراء سريعة ═══ */
function opportunityCard(): OfferDocument {
  const pageSize: PageSizeId = 'a5-portrait';
  const { widthMm, heightMm } = getPageSize(pageSize);
  const theme = { ...defaultTheme(), primary: GREEN, accent: GOLD, surface: WHITE };
  const heroH = 118;
  const els: DocElement[] = [
    createElement('shape', {
      x: 4, y: 4, width: widthMm - 8, height: heightMm - 8, zIndex: 0,
      style: { background: WHITE, borderColor: BORDER, borderWidth: 0.8, radius: 4 }
    }),
    createElement('image', {
      x: 10, y: 10, width: widthMm - 20, height: heroH, binding: 'property.image', zIndex: 1,
      style: { objectFit: 'cover', background: '#e7eceb', radius: 3, hideIfEmpty: false }
    }),
    // شعار صغير فوق الصورة
    createElement('office_field', {
      x: 14, y: 14, width: 60, height: 8, binding: 'office.name', zIndex: 3,
      style: { fontSize: 6.5, color: WHITE, align: 'right', fontWeight: 700, hideIfEmpty: true }
    }),
    // شريحة السعر تتداخل مع أسفل الصورة
    createElement('shape', {
      x: 14, y: 118, width: 74, height: 18, zIndex: 3,
      style: { background: GREEN, radius: 9 }
    }),
    createElement('property_field', {
      x: 17, y: 121, width: 68, height: 12, binding: 'property.price', zIndex: 4,
      style: { fontSize: 10, fontWeight: 700, color: '#f0d9a8', align: 'center', hideIfEmpty: true }
    }),
    createElement('heading', {
      x: 14, y: 142, width: widthMm - 28, height: 14, binding: 'property.title', zIndex: 2,
      style: { fontSize: 12, fontWeight: 700, color: GREEN_DARK, align: 'right', hideIfEmpty: true }
    }),
    createElement('property_field', {
      x: 14, y: 158, width: widthMm - 28, height: 10, binding: 'property.city', zIndex: 2,
      style: { fontSize: 8, fontWeight: 700, color: GREEN, align: 'right', hideIfEmpty: true }
    }),
    createElement('property_field', {
      x: 14, y: 168, width: widthMm - 28, height: 8, binding: 'property.district', zIndex: 2,
      style: { fontSize: 7.5, color: MUTED, align: 'right', hideIfEmpty: true }
    }),
    // ميزتان فقط: المساحة والغرض
    createElement('shape', {
      x: 14, y: 180, width: (widthMm - 28 - 4) / 2, height: 14, zIndex: 2,
      style: { background: '#eef4f2', borderColor: BORDER, borderWidth: 0.5, radius: 7 }
    }),
    createElement('property_field', {
      x: 17, y: 183, width: (widthMm - 28 - 4) / 2 - 6, height: 10, binding: 'property.area', zIndex: 4,
      style: { fontSize: 8, fontWeight: 700, color: GREEN_DARK, align: 'center', hideIfEmpty: true }
    }),
    createElement('shape', {
      x: 14 + (widthMm - 28 - 4) / 2 + 4, y: 180, width: (widthMm - 28 - 4) / 2, height: 14, zIndex: 2,
      style: { background: '#eef4f2', borderColor: BORDER, borderWidth: 0.5, radius: 7 }
    }),
    createElement('property_field', {
      x: 17 + (widthMm - 28 - 4) / 2 + 4, y: 183, width: (widthMm - 28 - 4) / 2 - 6, height: 10, binding: 'property.purpose', zIndex: 4,
      style: { fontSize: 8, fontWeight: 700, color: GREEN_DARK, align: 'center', hideIfEmpty: true }
    }),
    // تواصل: واتساب
    createElement('office_field', {
      x: 14, y: heightMm - 30, width: widthMm - 28, height: 10, binding: 'office.whatsapp', zIndex: 2,
      style: { fontSize: 8.5, fontWeight: 700, color: GREEN, align: 'right', hideIfEmpty: true }
    }),
    ...footer(widthMm, heightMm, theme, false, 10)
  ];
  return assemble('opportunity-brief', t('template.opportunity-brief'), pageSize, theme, els);
}

/** ═══ 5) الانقسام الفاخر — A4 عرضي صورة كاملة + لوحة ═══ */
function splitBrochure(): OfferDocument {
  const pageSize: PageSizeId = 'a4-landscape';
  const { widthMm, heightMm } = getPageSize(pageSize);
  const theme = { ...defaultTheme(), primary: NAVY, accent: GOLD, surface: WHITE };
  const panelX = 108;
  const panelW = widthMm - panelX;
  const elList: DocElement[] = [
    // البطل: صورة تملأ الارتفاع
    createElement('image', {
      x: 0, y: 0, width: panelX, height: heightMm, binding: 'property.image', zIndex: 0,
      style: { objectFit: 'cover', background: '#e8eaf0', hideIfEmpty: false }
    }),
    // شعار المكتب فوق الصورة
    createElement('office_field', {
      x: 8, y: 8, width: 70, height: 8, binding: 'office.name', zIndex: 3,
      style: { fontSize: 7, color: WHITE, align: 'right', fontWeight: 700, hideIfEmpty: true }
    }),
    // اللوحة الجانبية
    createElement('shape', {
      x: panelX, y: 0, width: panelW, height: heightMm, zIndex: 1,
      style: { background: NAVY, radius: 0 }
    }),
    createElement('property_field', {
      x: panelX + 10, y: 18, width: panelW - 20, height: 10, binding: 'property.purpose', zIndex: 3,
      style: { fontSize: 9, fontWeight: 700, color: '#f0d9a8', align: 'right', hideIfEmpty: true }
    }),
    createElement('heading', {
      x: panelX + 10, y: 32, width: panelW - 20, height: 26, binding: 'property.title', zIndex: 3,
      style: { fontSize: 17, fontWeight: 700, color: WHITE, align: 'right', hideIfEmpty: true }
    }),
    createElement('property_field', {
      x: panelX + 10, y: 64, width: panelW - 20, height: 14, binding: 'property.price', zIndex: 3,
      style: { fontSize: 15, fontWeight: 700, color: GOLD, align: 'right', hideIfEmpty: true }
    }),
    createElement('divider', {
      x: panelX + 10, y: 86, width: 36, height: 0.8, zIndex: 3,
      style: { background: GOLD }
    }),
    // حقول مفتاحية عمودية
    ...[
      { binding: 'property.area' as const, y: 98 },
      { binding: 'property.city' as const, y: 116 },
      { binding: 'property.district' as const, y: 134 },
      { binding: 'property.propertyType' as const, y: 152 }
    ].flatMap((s): DocElement[] => [
      createElement('property_field', {
        x: panelX + 10, y: s.y, width: panelW - 20, height: 12, binding: s.binding, zIndex: 3,
        style: { fontSize: 10, fontWeight: 700, color: 'rgba(255,255,255,0.95)', align: 'right', hideIfEmpty: true }
      })
    ]),
    // الوصف قصير
    createElement('text', {
      x: panelX + 10, y: 172, width: panelW - 20, height: 22, binding: 'property.description', zIndex: 3,
      style: { fontSize: 8.5, color: 'rgba(255,255,255,0.85)', align: 'right', hideIfEmpty: true }
    })
  ];
  // تواصل أسفل اللوحة
  elList.push(
    createElement('divider', {
      x: panelX + 10, y: heightMm - 34, width: panelW - 20, height: 0.4, zIndex: 3,
      style: { background: 'rgba(255,255,255,0.3)' }
    }),
    createElement('office_field', {
      x: panelX + 10, y: heightMm - 28, width: panelW - 20, height: 10, binding: 'office.phone', zIndex: 3,
      style: { fontSize: 10, fontWeight: 700, color: WHITE, align: 'right', hideIfEmpty: true }
    }),
    createElement('office_field', {
      x: panelX + 10, y: heightMm - 16, width: panelW - 20, height: 8, binding: 'office.falLicense', zIndex: 3,
      style: { fontSize: 6.5, color: 'rgba(255,255,255,0.7)', align: 'right', hideIfEmpty: true }
    })
  );
  return assemble('modern-split', t('template.modern-split'), pageSize, theme, elList);
}

/** ═══ 6) ميزات الوحدة — شبكة ميزات ═══ */
function featureBrochure(): OfferDocument {
  const pageSize: PageSizeId = 'a4-portrait';
  const { widthMm, heightMm } = getPageSize(pageSize);
  const theme = { ...defaultTheme(), primary: NAVY, accent: GOLD, surface: WHITE };
  const els: DocElement[] = [
    createElement('image', {
      x: 0, y: 0, width: widthMm, height: 118, binding: 'property.image', zIndex: 0,
      style: { objectFit: 'cover', background: '#e8eaf0', hideIfEmpty: false }
    }),
    createElement('office_field', {
      x: 10, y: 6, width: 70, height: 8, binding: 'office.name', zIndex: 3,
      style: { fontSize: 7, color: WHITE, align: 'right', fontWeight: 700, hideIfEmpty: true }
    }),
    createElement('heading', {
      x: 10, y: 128, width: widthMm - 20, height: 18, binding: 'property.title', zIndex: 2,
      style: { fontSize: 18, fontWeight: 700, color: NAVY, align: 'right', hideIfEmpty: true }
    }),
    createElement('property_field', {
      x: 10, y: 150, width: widthMm - 20, height: 10, binding: 'property.city', zIndex: 2,
      style: { fontSize: 9, fontWeight: 700, color: MUTED, align: 'right', hideIfEmpty: true }
    }),
    // شبكة الميزات: 2×3
    ...([
      { binding: 'property.propertyType' as const },
      { binding: 'property.area' as const },
      { binding: 'property.purpose' as const },
      { binding: 'property.frontage' as const },
      { binding: 'property.streetWidth' as const },
      { binding: 'property.pricePerMeter' as const }
    ].flatMap((s, i): DocElement[] => {
      const col = i % 2;
      const row = Math.floor(i / 2);
      const w = (widthMm - 20 - 4) / 2;
      const x = 10 + col * (w + 4);
      const y = 170 + row * 20;
      return [
        createElement('shape', {
          x, y, width: w, height: 16, zIndex: 2,
          style: { background: '#f2f5fa', borderColor: BORDER, borderWidth: 0.6, radius: 8 }
        }),
        createElement('property_field', {
          x: x + 3, y: y + 3, width: w - 6, height: 12, binding: s.binding, zIndex: 3,
          style: { fontSize: 8.5, fontWeight: 700, color: NAVY, align: 'center', hideIfEmpty: true }
        })
      ];
    })),
    // السعر بارز تحت الشبكة
    createElement('shape', {
      x: 10, y: 240, width: 100, height: 22, zIndex: 2,
      style: { background: NAVY, radius: 10 }
    }),
    createElement('property_field', {
      x: 14, y: 244, width: 92, height: 14, binding: 'property.price', zIndex: 3,
      style: { fontSize: 12, fontWeight: 700, color: GOLD, align: 'center', hideIfEmpty: true }
    }),
    ...footer(widthMm, heightMm, theme)
  ];
  return assemble('unit-catalog', t('template.unit-catalog'), pageSize, theme, els);
}

/** ═══ 7) ملف المعاينة — رمز رمز QR وبيانات الإعلان ═══ */
function previewDossier(): OfferDocument {
  const pageSize: PageSizeId = 'a4-portrait';
  const { widthMm, heightMm } = getPageSize(pageSize);
  const theme = { ...defaultTheme(), primary: NAVY, accent: GOLD, surface: WHITE };
  const els: DocElement[] = [
    createElement('image', {
      x: 0, y: 0, width: widthMm, height: 132, binding: 'property.image', zIndex: 0,
      style: { objectFit: 'cover', background: '#e8eaf0', hideIfEmpty: false }
    }),
    // رمز QR كبير — الإعلان الإلكتروني
    createElement('qr', {
      x: widthMm - 40, y: 20, width: 24, height: 24, binding: 'property.adLink', zIndex: 4,
      style: { background: WHITE, borderColor: NAVY, borderWidth: 1, radius: 4, hideIfEmpty: true }
    }),
    // شريحة بيضاء تحت الصورة: العنوان والموقع
    createElement('heading', {
      x: 10, y: 142, width: widthMm - 20, height: 18, binding: 'property.title', zIndex: 2,
      style: { fontSize: 18, fontWeight: 700, color: NAVY, align: 'right', hideIfEmpty: true }
    }),
    createElement('property_field', {
      x: 10, y: 164, width: 100, height: 10, binding: 'property.city', zIndex: 2,
      style: { fontSize: 9, fontWeight: 700, color: MUTED, align: 'right', hideIfEmpty: true }
    }),
    createElement('property_field', {
      x: 112, y: 164, width: 88, height: 10, binding: 'property.district', zIndex: 2,
      style: { fontSize: 9, color: MUTED, align: 'right', hideIfEmpty: true }
    }),
    // السعر + واتساب
    createElement('shape', {
      x: 10, y: 182, width: 98, height: 22, zIndex: 2,
      style: { background: '#fdf6e7', borderColor: BORDER_GOLD, borderWidth: 0.8, radius: 10 }
    }),
    createElement('property_field', {
      x: 14, y: 186, width: 90, height: 14, binding: 'property.price', zIndex: 3,
      style: { fontSize: 12, fontWeight: 700, color: GOLD_DEEP, align: 'center', hideIfEmpty: true }
    }),
    createElement('office_field', {
      x: 112, y: 187, width: 88, height: 12, binding: 'office.whatsapp', zIndex: 3,
      style: { fontSize: 9, fontWeight: 700, color: NAVY, align: 'right', hideIfEmpty: true }
    }),
    // ميزات: مساحة + غرض
    ...statRow(214, 10, widthMm - 20, 2, [
      { binding: 'property.area', color: NAVY },
      { binding: 'property.purpose', color: NAVY },
      { binding: 'property.propertyType', color: NAVY }
    ], { boxBg: '#f2f5fa', boxBorder: BORDER, radius: 8 }),
    // بيانات الإعلان
    createElement('property_field', {
      x: 10, y: 244, width: 92, height: 8, binding: 'property.adNumber', zIndex: 3,
      style: { fontSize: 7, color: MUTED, align: 'right', hideIfEmpty: true }
    }),
    createElement('property_field', {
      x: 104, y: 244, width: 96, height: 8, binding: 'property.falAdLicense', zIndex: 3,
      style: { fontSize: 7, color: MUTED, align: 'right', hideIfEmpty: true }
    }),
    ...footer(widthMm, heightMm, theme)
  ];
  return assemble('viewing-dossier', t('template.viewing-dossier'), pageSize, theme, els);
}

/** ═══ 8) العرض التجاري — شريط مؤشرات ═══ */
function commercialBrochure(): OfferDocument {
  const pageSize: PageSizeId = 'a4-portrait';
  const { widthMm, heightMm } = getPageSize(pageSize);
  const theme = { ...defaultTheme(), primary: NAVY_DARK, accent: GOLD, surface: WHITE };
  const els: DocElement[] = [
    createElement('image', {
      x: 0, y: 0, width: widthMm, height: 124, binding: 'property.image', zIndex: 0,
      style: { objectFit: 'cover', background: '#e8eaf0', hideIfEmpty: false }
    }),
    createElement('office_field', {
      x: 10, y: 6, width: 70, height: 8, binding: 'office.name', zIndex: 3,
      style: { fontSize: 7, color: WHITE, align: 'right', fontWeight: 700, hideIfEmpty: true }
    }),
    // شريط تعتيم أسفل الصورة للعنوان
    createElement('shape', {
      x: 0, y: 94, width: widthMm, height: 30, zIndex: 1,
      style: { background: 'rgba(19,32,62,0.55)', radius: 0 }
    }),
    createElement('heading', {
      x: 10, y: 100, width: widthMm - 20, height: 16, binding: 'property.title', zIndex: 2,
      style: { fontSize: 16, fontWeight: 700, color: WHITE, align: 'right', hideIfEmpty: true }
    }),
    createElement('property_field', {
      x: 10, y: 116, width: widthMm - 20, height: 8, binding: 'property.purpose', zIndex: 2,
      style: { fontSize: 8, fontWeight: 700, color: '#f0d9a8', align: 'right', hideIfEmpty: true }
    }),
    // شريط المؤشرات التجارية: أربعة tiles
    ...([
      { binding: 'property.frontage' as const },
      { binding: 'property.streetWidth' as const },
      { binding: 'property.area' as const },
      { binding: 'property.pricePerMeter' as const }
    ].flatMap((s, i): DocElement[] => {
      const w = (widthMm - 20 - 3 * 4) / 4;
      const x = 10 + i * (w + 4);
      return [
        createElement('shape', {
          x, y: 136, width: w, height: 26, zIndex: 2,
          style: { background: OFFWHITE, borderColor: BORDER, borderWidth: 0.6, radius: 8 }
        }),
        createElement('property_field', {
          x: x + 2, y: 144, width: w - 4, height: 12, binding: s.binding, zIndex: 3,
          style: { fontSize: 9, fontWeight: 700, color: NAVY_DARK, align: 'center', hideIfEmpty: true }
        })
      ];
    })),
    // السعر
    createElement('shape', {
      x: 10, y: 172, width: 100, height: 22, zIndex: 2,
      style: { background: NAVY_DARK, radius: 10 }
    }),
    createElement('property_field', {
      x: 14, y: 176, width: 92, height: 14, binding: 'property.price', zIndex: 3,
      style: { fontSize: 12, fontWeight: 700, color: GOLD, align: 'center', hideIfEmpty: true }
    }),
    createElement('property_field', {
      x: 116, y: 177, width: 84, height: 12, binding: 'property.city', zIndex: 3,
      style: { fontSize: 9, fontWeight: 700, color: NAVY_DARK, align: 'right', hideIfEmpty: true }
    }),
    // الوصف
    createElement('text', {
      x: 10, y: 206, width: widthMm - 20, height: 46, binding: 'property.description', zIndex: 3,
      style: { fontSize: 9.5, color: MUTED, align: 'right', hideIfEmpty: true }
    }),
    ...footer(widthMm, heightMm, theme)
  ];
  return assemble('commercial-offer', t('template.commercial-offer'), pageSize, theme, els);
}

/** ═══ 9) النمط التحريري — أبيض هادئ بفاصل فاخر ═══ */
function editorialBrochure(): OfferDocument {
  const pageSize: PageSizeId = 'a4-portrait';
  const { widthMm, heightMm } = getPageSize(pageSize);
  const theme = { ...defaultTheme(), primary: CHARCOAL, accent: GOLD, surface: WHITE };
  const els: DocElement[] = [
    // شريط علوي رفيع
    createElement('shape', {
      x: 0, y: 0, width: widthMm, height: 3, zIndex: 0,
      style: { background: GOLD, radius: 0 }
    }),
    createElement('office_field', {
      x: 14, y: 12, width: 80, height: 8, binding: 'office.name', zIndex: 2,
      style: { fontSize: 7.5, color: MUTED, align: 'right', fontWeight: 700, hideIfEmpty: true }
    }),
    // عنوان تحريري كبير
    createElement('heading', {
      x: 14, y: 26, width: widthMm - 28, height: 24, binding: 'property.title', zIndex: 2,
      style: { fontSize: 22, fontWeight: 700, color: CHARCOAL, align: 'right', hideIfEmpty: true }
    }),
    createElement('divider', {
      x: 14, y: 54, width: 44, height: 1, zIndex: 2,
      style: { background: GOLD }
    }),
    // صورة بإطار رفيع
    createElement('image', {
      x: 14, y: 64, width: widthMm - 28, height: 130, binding: 'property.image', zIndex: 1,
      style: { objectFit: 'cover', background: '#eceef1', borderColor: BORDER, borderWidth: 0.6, radius: 4, hideIfEmpty: false }
    }),
    // السعر والموقع سطر واحد
    createElement('property_field', {
      x: 14, y: 204, width: 90, height: 12, binding: 'property.price', zIndex: 2,
      style: { fontSize: 13, fontWeight: 700, color: GOLD_DEEP, align: 'right', hideIfEmpty: true }
    }),
    createElement('property_field', {
      x: 106, y: 206, width: 90, height: 10, binding: 'property.city', zIndex: 2,
      style: { fontSize: 9, fontWeight: 700, color: CHARCOAL, align: 'right', hideIfEmpty: true }
    }),
    // الوصف — النجم
    createElement('text', {
      x: 14, y: 226, width: widthMm - 28, height: 46, binding: 'property.description', zIndex: 2,
      style: { fontSize: 10, color: INK, align: 'right', hideIfEmpty: true }
    }),
    ...footer(widthMm, heightMm, theme, false, 14)
  ];
  return assemble('marketing-report', t('template.marketing-report'), pageSize, theme, els);
}

/** ═══ 10) البوستر الليلي — full-bleed درامي ═══ */
function nightPoster(): OfferDocument {
  const pageSize: PageSizeId = 'flyer';
  const { widthMm, heightMm } = getPageSize(pageSize);
  const theme = { ...defaultTheme(), primary: NAVY_DARK, accent: GOLD, surface: NAVY_DARK };
  const els: DocElement[] = [
    // صورة full-bleed للصفحة كاملة
    createElement('image', {
      x: 0, y: 0, width: widthMm, height: heightMm, binding: 'property.image', zIndex: 0,
      style: { objectFit: 'cover', background: '#1a2742', hideIfEmpty: false }
    }),
    // تعتيم سفلي للقراءة
    createElement('shape', {
      x: 0, y: 96, width: widthMm, height: 84, zIndex: 1,
      style: { background: 'rgba(13,22,44,0.62)', radius: 0 }
    }),
    createElement('office_field', {
      x: 8, y: 8, width: widthMm - 16, height: 8, binding: 'office.name', zIndex: 2,
      style: { fontSize: 7, color: 'rgba(255,255,255,0.95)', align: 'center', fontWeight: 700, hideIfEmpty: true }
    }),
    createElement('property_field', {
      x: 8, y: 104, width: widthMm - 16, height: 8, binding: 'property.purpose', zIndex: 2,
      style: { fontSize: 8, fontWeight: 700, color: '#f0d9a8', align: 'center', hideIfEmpty: true }
    }),
    createElement('heading', {
      x: 8, y: 114, width: widthMm - 16, height: 20, binding: 'property.title', zIndex: 2,
      style: { fontSize: 14, fontWeight: 700, color: WHITE, align: 'center', hideIfEmpty: true }
    }),
    createElement('property_field', {
      x: 8, y: 138, width: widthMm - 16, height: 12, binding: 'property.price', zIndex: 2,
      style: { fontSize: 15, fontWeight: 700, color: GOLD, align: 'center', hideIfEmpty: true }
    }),
    createElement('property_field', {
      x: 8, y: 154, width: widthMm - 16, height: 8, binding: 'property.city', zIndex: 2,
      style: { fontSize: 8.5, fontWeight: 700, color: 'rgba(255,255,255,0.95)', align: 'center', hideIfEmpty: true }
    }),
    // تواصل أسفل البوستر
    createElement('office_field', {
      x: 8, y: 166, width: widthMm - 16, height: 10, binding: 'office.phone', zIndex: 2,
      style: { fontSize: 9.5, fontWeight: 700, color: WHITE, align: 'center', hideIfEmpty: true }
    }),
    ...footer(widthMm, heightMm, theme, true)
  ];
  return assemble('emerald-card', t('template.emerald-card'), pageSize, theme, els);
}

export const DOC_TEMPLATES: DocTemplateDef[] = [
  {
    id: 'royal-sheet',
    get name() { return t('template.royal-sheet'); },
    get description() { return systemT('design.ui.153'); },
    pageSize: 'a4-portrait',
    build: heroBrochure
  },
  {
    id: 'investment-brief',
    get name() { return t('template.investment-brief'); },
    get description() { return systemT('design.ui.154'); },
    pageSize: 'a4-portrait',
    build: investmentBrief
  },
  {
    id: 'gold-luxury',
    get name() { return t('template.gold-luxury'); },
    get description() { return systemT('design.ui.155'); },
    pageSize: 'a4-portrait',
    build: goldBrochure
  },
  {
    id: 'opportunity-brief',
    get name() { return t('template.opportunity-brief'); },
    get description() { return systemT('design.ui.156'); },
    pageSize: 'a5-portrait',
    build: opportunityCard
  },
  {
    id: 'modern-split',
    get name() { return t('template.modern-split'); },
    get description() { return systemT('design.ui.157'); },
    pageSize: 'a4-landscape',
    build: splitBrochure
  },
  {
    id: 'unit-catalog',
    get name() { return t('template.unit-catalog'); },
    get description() { return systemT('design.ui.158'); },
    pageSize: 'a4-portrait',
    build: () => withDetailsPage(featureBrochure(), 'design.projectDetails')
  },
  {
    id: 'viewing-dossier',
    get name() { return t('template.viewing-dossier'); },
    get description() { return systemT('design.ui.159'); },
    pageSize: 'a4-portrait',
    build: previewDossier
  },
  {
    id: 'commercial-offer',
    get name() { return t('template.commercial-offer'); },
    get description() { return systemT('design.ui.160'); },
    pageSize: 'a4-portrait',
    build: commercialBrochure
  },
  {
    id: 'marketing-report',
    get name() { return t('template.marketing-report'); },
    get description() { return systemT('design.ui.161'); },
    pageSize: 'a4-portrait',
    build: () => withDetailsPage(editorialBrochure(), 'design.propertyDetails')
  },
  {
    id: 'emerald-card',
    get name() { return t('template.emerald-card'); },
    get description() { return systemT('design.ui.162'); },
    pageSize: 'flyer',
    build: nightPoster
  }
];

export function findDocTemplate(id?: string | null): DocTemplateDef | undefined {
  if (!id) return undefined;
  return DOC_TEMPLATES.find((t) => t.id === id);
}

export function getDocTemplate(id: string): DocTemplateDef {
  return findDocTemplate(id) || DOC_TEMPLATES[0];
}

export function templateDisplayName(id?: string | null): string {
  if (!id) return '—';
  const found = findDocTemplate(id);
  if (found) return found.name;
  if (id === 'blank' || id === 'custom') return systemT('design.ui.163');
  if (id === 'template1' || id === 'template2' || id === 'template3') return systemT('design.ui.164');
  return id;
}

export function clonePage(page: DocPage, name?: string): DocPage {
  return {
    id: newId('page'),
    name: name || `${page.name} (${t('design.copy')})`,
    systemName: name ? false : page.systemName,
    order: page.order + 1,
    elements: page.elements.map((el) => ({
      ...el,
      id: newId('el'),
      style: { ...el.style }
    }))
  };
}

function withDetailsPage(doc: OfferDocument, headingKey: string): OfferDocument {
  const size = getPageSize(doc.pageSize);
  doc.pages.push({ id: newId('page'), name: '2', systemName: true, order: 1, elements: [
    createElement('heading', { x: 12, y: 14, width: size.widthMm - 24, height: 18, content: '', systemTextKey: headingKey }),
    createElement('specs_table', { x: 12, y: 40, width: size.widthMm - 24, height: 100 }),
    createElement('text', { x: 12, y: 150, width: size.widthMm - 24, height: 100, binding: 'property.description', content: '', style: { hideIfEmpty: true } }),
    ...footer(size.widthMm, size.heightMm, doc.theme)
  ] });
  return doc;
}
