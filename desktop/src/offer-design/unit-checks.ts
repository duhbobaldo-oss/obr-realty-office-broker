/**
 * اختبارات وحدة لموديل المصمم والربط والتراجع.
 */
import { migrateToV2, createElement, emptyDocument, sortedPages } from './model';
import { resolveBinding, displayText, labeledFieldText, displayImage } from './bindings';
import { createHistory } from './history';
import { clonePage, DOC_TEMPLATES } from './templates-v2';
import { THEMES } from '../theme';
import { qrSvgMarkup } from './qr';
import { placeFillPop } from './placeFillPop';
import { applyImageToFrame, imageSlotForPage } from './imageSlots';
import { applyCommandArgAliases } from '../api';
import { formatMoneyAmount, formatRate } from '../campaigns';
import { monthCells } from '../schedule';
import { buildMailtoUrl, isAllowedExternalUrl } from '../mail';
import {
  buildOfferShareText,
  buildWhatsAppGreeting,
  buildWhatsAppUrl,
  communicationTypeLabel,
  isAllowedWhatsAppUrl,
  normalizeWhatsAppPhone
} from '../whatsapp';

export function runOfferDesignUnitChecks(): string[] {
  const errors: string[] = [];

  if (THEMES.length !== 3) errors.push('expected classic, archive, and system themes');
  if (!THEMES.some((t) => t.id === 'classic' && t.name === 'كلاسيكي')) errors.push('classic theme missing');
  if (!THEMES.some((t) => t.id === 'archive' && t.name === 'أرشيف المكتب')) errors.push('archive theme missing');
  if (!THEMES.some((t) => t.id === 'system' && t.name === 'نظامي')) errors.push('system theme missing');
  if (THEMES.some((t) => ['corporate-blue', 'neutral', 'emerald'].includes(t.id))) {
    errors.push('removed app themes still listed');
  }

  if (DOC_TEMPLATES.length !== 10) errors.push('expected 10 professional templates');
  const names = DOC_TEMPLATES.map((t) => t.name);
  if (new Set(names).size !== 10) errors.push('template names must be unique');
  const amateur = /ورقة|بطاقة|ملكية|صفحة فارغة|قالب البداية|صك|سند/;
  if (names.some((n) => amateur.test(n))) errors.push('amateur template name');
  if (DOC_TEMPLATES.some((t) => t.id === 'blank')) errors.push('blank starter template must be removed');
  for (const t of DOC_TEMPLATES) {
    const doc = t.build();
    if (!doc.pages[0]?.elements?.length) errors.push(`template ${t.id} has no elements`);
    if (doc.templateOrigin !== t.id) errors.push(`template origin mismatch ${t.id}`);
  }

  const v2 = migrateToV2({
    version: 2,
    title: 'ت',
    propertyId: 'p1',
    pageSize: 'a4-portrait',
    theme: emptyDocument().theme,
    pages: [
      { id: 'a', name: 'صفحة 1', order: 0, elements: [] },
      { id: 'b', name: 'صفحة 2', order: 1, elements: [createElement('text', { content: 'س' })] }
    ],
    updatedAt: new Date().toISOString()
  });
  if (v2.pages.length !== 2) errors.push('migrate v2 pages');

  const v1 = migrateToV2({
    version: 1,
    templateId: 'royal-sheet',
    pageSize: 'a4-portrait',
    propertyId: 'p1',
    fields: { title: 'عرض', price: '1', falLicense: '' },
    theme: emptyDocument().theme,
    updatedAt: new Date().toISOString()
  });
  if (v1.version !== 2 || !v1.pages[0]?.elements?.length) errors.push('migrate v1');

  const fal = resolveBinding('property.falAdLicense', { fal_ad_license: '1200' }, null);
  if (fal !== '1200') errors.push('fal binding');

  const emptyFal = resolveBinding('property.falAdLicense', { fal_ad_license: null }, null);
  if (emptyFal !== '') errors.push('null fal should be empty');

  displayText('property.falAdLicense', '', { fal_ad_license: null }, null);

  const officeFal = resolveBinding(
    'office.falLicense',
    null,
    { country: 'SA', fal_license_number: '99', fal_license_type: 'brokerage_marketing' }
  );
  if (!officeFal.includes('99') || !officeFal.includes('وساطة')) errors.push('office fal');

  const shown = displayText('property.price', '', { price: 1500000, country: 'SA', currency: 'SAR' }, null);
  if (!shown || !shown.includes('SAR')) errors.push('price format');

  const typed = displayText('property.price', '500 ألف', { price: 1500000, country: 'SA', currency: 'SAR' }, null);
  if (typed !== '500 ألف') errors.push('manual fill overrides binding');

  const labeled = labeledFieldText('property.price', '500 ألف', null, null);
  if (labeled !== 'السعر : 500 ألف') errors.push('labeled price');

  const manualImg = displayImage('property.image', 'data:image/png;base64,abc', { main_image: 'http://prop.jpg' }, null);
  if (manualImg !== 'data:image/png;base64,abc') errors.push('manual image overrides binding');
  const boundImg = displayImage('property.image', '', { main_image: 'http://prop.jpg' }, null);
  if (boundImg !== 'http://prop.jpg') errors.push('bound property image');

  const frame = createElement('image', { width: 110, height: 82 });
  const applied = applyImageToFrame(frame, 'data:image/jpeg;base64,xx');
  if (applied.width !== 110 || applied.height !== 82) errors.push('image insert must not grow frame');
  if (applied.style.objectFit !== 'contain') errors.push('inserted image must contain');

  const a4hero = imageSlotForPage('a4-portrait', 'hero');
  if (a4hero.width > 186 || a4hero.height > 273) errors.push('hero slot exceeds A4 inner area');
  if (a4hero.width < 40 || a4hero.height < 30) errors.push('hero slot too small');


  const link = resolveBinding('property.adLink', { ad_link: 'https://example.com/ad' }, null);
  if (link !== 'https://example.com/ad') errors.push('ad link binding');

  const hist = createHistory({ n: 1 });
  hist.set({ n: 2 }, true);
  hist.set({ n: 3 }, true);
  const u1 = hist.undo();
  if (!u1 || u1.n !== 2) errors.push('undo');
  const r1 = hist.redo();
  if (!r1 || r1.n !== 3) errors.push('redo');

  const page = {
    id: 'p',
    name: 'صفحة 1',
    order: 0,
    elements: [createElement('property_field', { binding: 'property.falAdLicense', x: 10, y: 12, width: 40, height: 8 })]
  };
  const copy = clonePage(page, 'صفحة 2');
  if (copy.id === page.id) errors.push('clone page id');
  if (copy.elements[0].id === page.elements[0].id) errors.push('clone element id');
  copy.elements[0].x = 99;
  if (page.elements[0].x === 99) errors.push('clone isolation');

  const doc = emptyDocument();
  doc.pages.push({
    id: 'p2',
    name: 'صفحة 2',
    order: 1,
    elements: [createElement('heading', { content: 'ب' })]
  });
  if (sortedPages(doc).length !== 2) errors.push('sorted pages');

  const qr = qrSvgMarkup('https://example.com');
  if (!qr || !qr.includes('<svg') || !qr.includes('<rect') || !qr.includes('fill="#000000"')) {
    errors.push('qr svg');
  }
  if (!qr?.includes('fill="#ffffff"')) errors.push('qr quiet zone');
  if (qr?.includes('preserveAspectRatio="none"')) errors.push('qr stretched');
  if (qrSvgMarkup('') !== null) errors.push('empty qr');

  const sensitive = resolveBinding('property.price', { price: 1, country: 'SA', currency: 'SAR', owner_phone: '055', internal_price: 2 }, null);
  if (String(sensitive).includes('055')) errors.push('owner phone leaked');

  const under = placeFillPop({
    elLeft: 40,
    elTop: 20,
    elWidth: 80,
    elHeight: 24,
    minLeft: 0,
    maxRight: 400,
    minTop: 0,
    maxBottom: 300,
    minWidth: 220,
    popHeight: 48,
    gap: 8
  });
  if (under.left !== 40 || under.top !== 52 || under.width !== 220) errors.push('fill pop under element');

  const rightEdge = placeFillPop({
    elLeft: 360,
    elTop: 20,
    elWidth: 30,
    elHeight: 20,
    minLeft: 0,
    maxRight: 400,
    minTop: 0,
    maxBottom: 300,
    minWidth: 220,
    popHeight: 48,
    gap: 8
  });
  if (rightEdge.left + rightEdge.width > 400) errors.push('fill pop overflow right');
  if (rightEdge.left > 200) errors.push('fill pop drifted far from right-edge field');

  const bottom = placeFillPop({
    elLeft: 20,
    elTop: 260,
    elWidth: 80,
    elHeight: 30,
    minLeft: 0,
    maxRight: 400,
    minTop: 0,
    maxBottom: 300,
    minWidth: 220,
    popHeight: 48,
    gap: 8
  });
  if (bottom.top >= 260) errors.push('fill pop should flip above near page bottom');
  if (bottom.left !== 20) errors.push('fill pop should stay under left-side field');

  try {
    const mail = buildMailtoUrl('OBR66@outlook.com', 'موضوع', 'نص');
    if (!mail.startsWith('mailto:OBR66@outlook.com?')) errors.push('mailto url');
    if (!isAllowedExternalUrl(mail)) errors.push('mailto allowed');
  } catch {
    errors.push('mailto url threw');
  }
  try {
    buildMailtoUrl('not-an-email', 'x', 'y');
    errors.push('invalid email accepted');
  } catch {
    /* expected */
  }
  if (isAllowedExternalUrl('javascript:alert(1)')) errors.push('javascript url allowed');
  if (isAllowedExternalUrl('mailto:x\nCC:evil')) errors.push('mailto newline allowed');

  const local = normalizeWhatsAppPhone('050 123 4567');
  if (!('phone' in local) || local.phone !== '966501234567') errors.push('saudi local phone');
  const intl = normalizeWhatsAppPhone('+966-5-0123-4567');
  if (!('phone' in intl) || intl.phone !== '966501234567') errors.push('saudi intl phone');
  if (!('error' in normalizeWhatsAppPhone('123'))) errors.push('short phone accepted');
  if (!('error' in normalizeWhatsAppPhone('0112345678'))) errors.push('landline accepted');
  const greeting = buildWhatsAppGreeting('مكتب الاختبار');
  if (!greeting.includes('السلام عليكم') || !greeting.includes('مكتب الاختبار')) errors.push('whatsapp greeting');
  const offerText = buildOfferShareText({
    officeName: 'مكتب الاختبار',
    offerName: 'فيلا حديثة',
    city: 'الرياض',
    district: 'العقيق',
    price: 3850000,
    area: 420
  });
  if (!offerText.includes('العرض: فيلا حديثة') || offerText.includes('PDF') || offerText.includes('الصق')) {
    errors.push('offer text must stay text-only');
  }
  const waUrl = buildWhatsAppUrl('966501234567', greeting);
  if (!waUrl.startsWith('whatsapp://send?phone=966501234567')) errors.push('whatsapp url');
  if (!isAllowedWhatsAppUrl(waUrl)) errors.push('whatsapp url rejected');
  if (isAllowedWhatsAppUrl('javascript:alert(1)')) errors.push('js allowed as whatsapp');
  if (communicationTypeLabel('whatsapp_opened', [{ value: 'whatsapp', label: 'واتساب' }]) !== 'واتساب') {
    errors.push('opened type leaked');
  }
  const mapped = applyCommandArgAliases('clients_add_communication', { type: 'whatsapp', notes: 'تم فتح محادثة واتساب' });
  if (mapped.commType !== 'whatsapp') errors.push('commType alias');
  if (mapped.subject !== '') errors.push('subject default');
  if (formatRate(null) !== '—' || formatRate(undefined) !== '—') errors.push('null rate must be dash');
  if (formatRate(0.1) !== '10%') errors.push('inquiry rate display');
  if (formatMoneyAmount(null) !== '—') errors.push('null cost metric must be dash');
  const sept = monthCells(2026, 8);
  if (sept.length !== 42) errors.push('calendar month cells');
  if (sept[0].iso !== '2026-08-29' || sept[3].iso !== '2026-09-01') errors.push('saturday-start september 2026');

  return errors;
}
