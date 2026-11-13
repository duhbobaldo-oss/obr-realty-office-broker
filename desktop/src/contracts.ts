// إعدادات جهات توثيق العقود — المصدر الوحيد القابل للتعديل مستقبلًا.
// الواجهة لا تُضمّن أي رابط ثابت: تُقرأ جهة التوثيق من ملخص العقود (الخادم)
// وتُستخدم هذه القيم الافتراضية فقط عند تعذر الوصول للخادم.

export const BROKERAGE_PORTAL_BUTTON_LABEL = 'الخدمات الرسمية للوساطة العقارية';
export const DRAFT_UNVERIFIED_BADGE = 'مسودة غير موثقة';
export function brokeragePortalUrl(summary: any): string {
  return summary?.official_service?.url || '';
}

/** العقد غير الرسمي: مسودة/غير موثق أو مصدر يدوي — يستحق وسم «مسودة غير موثقة». */
export function isUnofficialContract(c: any): boolean {
  if (!c) return false;
  const status = String(c.status || '');
  if (status === 'draft' || status === 'unverified') return true;
  const source = String(c.source || '');
  return source === 'manual_draft' || source === '';
}
