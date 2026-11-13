/**
 * فهرس النماذج الجاهزة المدمجة في التطبيق.
 * هذه قوالب عرض/طباعة فقط — ليست سجلات تشغيلية ولا تدخل إحصاءات لوحة العمل.
 */
export type BuiltInTemplateKind =
  | 'offer_print'
  | 'brochure'
  | 'invoice'
  | 'finance_report'
  | 'campaign_form';

export type BuiltInTemplate = {
  id: string;
  kind: BuiltInTemplateKind;
  name: string;
  desc: string;
  where: string;
};

export const BUILT_IN_TEMPLATE_KIND_LABELS: Record<BuiltInTemplateKind, string> = {
  offer_print: 'نماذج العروض المطبوعة',
  brochure: 'نماذج البروشور',
  invoice: 'نماذج الفواتير',
  finance_report: 'نماذج التقارير المالية',
  campaign_form: 'نماذج الحملات'
};

/** قوالب مدمجة في الكود؛ لا تُزرع كعملاء أو عروض في قاعدة البيانات. */
export const BUILT_IN_TEMPLATES: BuiltInTemplate[] = [
  {
    id: 'royal-sheet',
    kind: 'offer_print',
    name: 'العرض التنفيذي',
    desc: 'صفحة عرض رسمية ببيانات العقار والمكتب.',
    where: 'تصميم وطباعة العروض'
  },
  {
    id: 'investment-brief',
    kind: 'offer_print',
    name: 'الملف الاستثماري',
    desc: 'ملخص استثماري للمساحات والأسعار.',
    where: 'تصميم وطباعة العروض'
  },
  {
    id: 'residential-profile',
    kind: 'brochure',
    name: 'بروفايل السكن',
    desc: 'نموذج بروشور سكني للطباعة أو الحفظ في المكتبة.',
    where: 'تصميم وطباعة العروض ← مكتبة العروض'
  },
  {
    id: 'units-catalog',
    kind: 'brochure',
    name: 'كتالوج الوحدات',
    desc: 'عرض وحدات متعددة في ورقة واحدة.',
    where: 'تصميم وطباعة العروض'
  },
  {
    id: 'finance-invoice-pdf',
    kind: 'invoice',
    name: 'فاتورة المكتب',
    desc: 'إخراج فاتورة PDF بهوية المكتب من الإعدادات.',
    where: 'المالية والعقود ← الفواتير'
  },
  {
    id: 'finance-report-pdf',
    kind: 'finance_report',
    name: 'التقرير المالي',
    desc: 'تقرير ملخص للفترة المحددة بصيغة PDF.',
    where: 'المالية والعقود ← التقارير'
  },
  {
    id: 'campaign-record',
    kind: 'campaign_form',
    name: 'سجل حملة تسويقية',
    desc: 'نموذج إدخال مشاهدات واستفسارات ومعاينات وصفقات.',
    where: 'الحملات التسويقية'
  }
];
