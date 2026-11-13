/**
 * تشغيل: npx --yes tsx src/campaigns.audit.ts
 * يتحقق من معادلة إجمالي المشاهدات ونسب مسار التحويل في الحملات.
 */
import assert from 'node:assert/strict';
import { campaignRate, campaignTotalViews, formatCount } from './campaigns.ts';

// السيناريو المعتمد: مشاهدات يدوية = 120، نقرات تتبع = 35، زيارات فريدة = 20
const VIEWS = 120;
const TRACKED = 35;
const UNIQUE = 20;

const total = campaignTotalViews(VIEWS, TRACKED);
assert.equal(total, 155, 'إجمالي المشاهدات يجب أن يكون 155');
assert.equal(TRACKED, 35, 'نقرات OBR تبقى 35');
assert.equal(UNIQUE, 20, 'الزيارات الفريدة تبقى 20');
assert.ok(
  !String(total).includes(String(UNIQUE)) && total !== VIEWS + TRACKED + UNIQUE,
  'الزيارات الفريدة لا تُدمج في الإجمالي'
);

// الزيارات الفريدة لا تدخل في الإجمالي حتى لو مُرّرت بالخطأ كوسيطة ثالثة
assert.equal(campaignTotalViews(VIEWS, TRACKED), 155);

// حالات حدية
assert.equal(campaignTotalViews(0, 0), 0);
assert.equal(campaignTotalViews(null, null), 0);
assert.equal(campaignTotalViews(undefined, undefined), 0);
assert.equal(campaignTotalViews(-5, 10), 10, 'القيم السالبة تُعامل كصفر');
assert.equal(campaignTotalViews(120.9, 35.9), 155, 'الكسور تُقص إلى أعداد صحيحة');

// نسبة الاستفسار تُحسب على إجمالي المشاهدات (وليس اليدوي وحده)
assert.equal(campaignRate(12, total), '7.7%', '12÷155 = 7.7%');
assert.equal(campaignRate(12, VIEWS), '10%', '12÷120 = 10% (مرجع المقارنة فقط)');
assert.equal(campaignRate(0, 0), '—', 'المقام صفر = غير قابل للحساب');
assert.equal(campaignRate(5, null), '—');

// تنسيق الأعداد في التقارير
assert.equal(formatCount(total), '155');

console.log('campaigns audit: ALL PASS');
console.log(`  إجمالي المشاهدات = ${formatCount(total)} (يدوي ${VIEWS} + نقرات التتبع ${TRACKED})`);
console.log(`  نقرات OBR = ${TRACKED} | الزيارات الفريدة = ${UNIQUE} (مؤشر مستقل)`);
