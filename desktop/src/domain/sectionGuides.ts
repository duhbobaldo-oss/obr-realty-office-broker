/** شروحات مختصرة لأقسام التطبيق — تظهر في الصفحة وفي مركز التعليمات (عربي / إنجليزي). */

import { getUiLanguage } from './locale';

export type SectionGuide = {
  id: string;
  title: string;
  summary: string;
  steps: string[];
  /** فقرات معنونة (مثل تعريفات المعادلات) بدل قائمة مرقّمة */
  blocks?: { title: string; body: string }[];
};

type Bilingual = { ar: string; en: string };

type RawSectionGuide = {
  id: string;
  title: Bilingual;
  summary: Bilingual;
  steps: Bilingual[];
  blocks?: { title: Bilingual; body: Bilingual }[];
};

const RAW_SECTION_GUIDES: RawSectionGuide[] = [
  {
    id: 'dashboard',
    title: { ar: 'لوحة العمل', en: 'Workspace' },
    summary: {
      ar: 'ملخص يومك: أعداد العروض والعملاء، المواعيد، وأبرز النشاط دون بيانات تدريبية.',
      en: 'Your day at a glance: listing and client counts, appointments, and key activity — including training data until you remove it.'
    },
    steps: [
      {
        ar: 'راجع الأرقام أولاً لتعرف ما يحتاج متابعة.',
        en: 'Review the numbers first to see what needs follow-up.'
      },
      {
        ar: 'افتح المواعيد من التقويم أو الساعة لإضافة موعد محفوظ محليًا.',
        en: 'Open appointments from the calendar or clock to add an appointment saved locally.'
      },
      {
        ar: 'انتقل لأي قسم من القائمة الجانبية أو من البطاقات.',
        en: 'Go to any section from the sidebar or from the cards.'
      }
    ]
  },
  {
    id: 'market',
    title: { ar: 'المؤشر الخاص', en: 'Private Index' },
    summary: {
      ar: 'مرجع أسعارك الخاص — مبني على ما تدخله أنت فقط.',
      en: 'Your private price reference — built only from what you enter.'
    },
    steps: [
      {
        ar: 'أضف سجلات يدويًا أو احفظ نموذج Excel ثم استورده بعد المعاينة والتأكيد.',
        en: 'Add records manually, or save the Excel template then import after preview and confirmation.'
      },
      {
        ar: 'لكل سجل: سعر البيع الإجمالي، سعر المتر، قيمة الإيجار، وسعر المتر للإيجار.',
        en: 'For each record: total sale price, price per square meter, rent value, and rent per square meter.'
      },
      {
        ar: 'لوحة المؤشر — مرجع أسعارك الخاص تلخّص المدن. صدّر كل السجلات عبر «حفظ السجلات كـ Excel».',
        en: 'The index board — your private price reference — summarizes cities. Export all records via “Save records as Excel”.'
      },
      {
        ar: 'عدّل أو احذف أي سجل من بطاقته.',
        en: 'Edit or delete any record from its card.'
      }
    ]
  },
  {
    id: 'properties',
    title: { ar: 'العروض العقارية', en: 'Listings' },
    summary: {
      ar: 'سجل عروضك للبيع أو الإيجار أو الاستثمار مع الحالة والسعر والصور.',
      en: 'Record your sale, rent, or investment listings with status, price, and photos.'
    },
    steps: [
      {
        ar: 'أضف عرضًا جديدًا واملأ بيانات العقار والمالك.',
        en: 'Add a new listing and fill in the property and owner details.'
      },
      {
        ar: 'حدّث الحالة (متاح، قيد التفاوض، محجوز...).',
        en: 'Update the status (available, under negotiation, reserved…).'
      },
      {
        ar: 'شارك العرض عبر واتساب من البطاقة أو من ملف العميل.',
        en: 'Share the listing via WhatsApp from the card or from the client profile.'
      }
    ]
  },
  {
    id: 'maps',
    title: { ar: 'الخرائط العقارية', en: 'Property Maps' },
    summary: {
      ar: 'ضع موقع العرض على الخريطة واحفظه في حسابك فقط. مرّر الماوس على الدبوس المحفوظ لرؤية بطاقة مختصرة.',
      en: 'Place the listing on the map and save it to your account only. Hover a saved pin to see a short card.'
    },
    steps: [
      {
        ar: 'انقر على الخريطة داخل المدينة لوضع دبوس برتقالي قابل للسحب.',
        en: 'Click the map inside the city to place a draggable orange pin.'
      },
      {
        ar: 'اكتب اسمًا ثم «حفظ في عروضي»، أو اربط الدبوس بعرض موجود ثم احفظ.',
        en: 'Enter a name then “Save to my listings”, or link the pin to an existing listing and save.'
      },
      {
        ar: 'بعد الحفظ يظهر الدبوس ثابتًا؛ مرّر عليه لترى الاسم والسعر والحالة.',
        en: 'After saving, the pin stays fixed; hover it to see name, price, and status.'
      },
      {
        ar: 'لا يُحفظ شيء قبل الضغط على زر الحفظ من اللوحة اليسرى.',
        en: 'Nothing is saved until you press the save button on the left panel.'
      }
    ]
  },
  {
    id: 'clients',
    title: { ar: 'العملاء والعلاقات', en: 'Clients & Relations' },
    summary: {
      ar: 'جهات الاتصال: مشتري، مستأجر، مالك، مستثمر… مع التواصل والمتابعة.',
      en: 'Contacts: buyer, tenant, owner, investor… with communication and follow-up.'
    },
    steps: [
      {
        ar: 'أضف عميلًا وحدّد الدور والمصدر والحالة.',
        en: 'Add a client and set role, source, and status.'
      },
      {
        ar: 'سجّل تواصلًا وحدد موعد متابعة.',
        en: 'Log a contact and set a follow-up date.'
      },
      {
        ar: 'اربط عروضًا أو افتح واتساب من ملف العميل.',
        en: 'Link listings or open WhatsApp from the client profile.'
      }
    ]
  },
  {
    id: 'campaigns',
    title: { ar: 'الحملات التسويقية', en: 'Marketing Campaigns' },
    summary: {
      ar: 'تتبّع مشاهدات واستفسارات ومعاينات وصفقات كل حملة وتكلفتها، مع رابط تتبع اختياري لمشاركاتك خارج منصة الإعلان.',
      en: 'Track views, inquiries, viewings, and deals for each campaign and its cost, with an optional tracking link for shares outside the ad platform.'
    },
    blocks: [
      {
        title: { ar: 'تتبع رابط الإعلان', en: 'Ad link tracking' },
        body: {
          ar: 'عند إنشاء حملة، الصق رابط إعلانك المنشور في منصة مثل عقار أو بيوت أو حراج، ثم أنشئ رابط تتبع من OBR. رابط التتبع لا يُنشر داخل نفس منصة الإعلان، بل يُستخدم عند مشاركة الإعلان عبر واتساب، سناب، إنستغرام، X، الرسائل، موقع المكتب، أو QR في البروشور. عند ضغط العميل على رابط OBR: تُسجَّل النقرة أولًا، ثم يفتح إعلانك الأصلي مباشرة. عدد المشاهدات اليدوي يبقى منفصلًا ويدخله المكتب.',
          en: 'When creating a campaign, paste your published ad link from a platform such as Aqar, Bayut, or Haraj, then create a tracking link in OBR. The tracking link is not posted inside the same ad platform; use it when sharing via WhatsApp, Snapchat, Instagram, X, messages, your office site, or a brochure QR. When a client taps the OBR link, the click is recorded first, then your original ad opens. Manual view counts stay separate and are entered by the office.'
        }
      }
    ],
    steps: [
      {
        ar: 'أنشئ حملة واربطها بعرض إن أردت.',
        en: 'Create a campaign and link it to a listing if you want.'
      },
      {
        ar: 'الصق رابط الإعلان ثم اضغط «إنشاء رابط تتبع»، وشارك رابط OBR عبر واتساب أو سناب أو الرسائل أو QR.',
        en: 'Paste the ad link, tap “Create tracking link”, then share the OBR link via WhatsApp, Snapchat, messages, or QR.'
      },
      {
        ar: 'أدخل المشاهدات والاستفسارات يدويًا؛ التطبيق يحسب معدلات التحويل والتكلفة. نقرات رابط OBR منفصلة.',
        en: 'Enter views and inquiries manually; the app calculates conversion rates and cost. OBR link clicks stay separate.'
      },
      {
        ar: 'إذا كان المقام صفرًا يظهر «—» بدل اختراع رقم.',
        en: 'If the denominator is zero, “—” is shown instead of inventing a number.'
      }
    ]
  },
  {
    id: 'finance',
    title: { ar: 'المالية والعقود', en: 'Finance & contracts' },
    summary: {
      ar: 'فواتير وعمليات وعمولات ومصروفات ودفعات وتقارير لهوية مكتبك، مع سجل العقود وملخصها وطباعتها.',
      en: 'Invoices, transactions, commissions, expenses, payments, and reports under your office identity, plus the contracts register with its summary and printing.'
    },
    blocks: [
      {
        title: { ar: 'مسودة العقد وطباعتها', en: 'Contract drafts and printing' },
        body: {
          ar: 'من تبويب العقود اضغط «مسودة جديدة» ثم «معاينة» لمراجعة النوع والأطراف والعقار والمدة والعمولة والتواريخ، و«طباعة» لتجهيز PDF بهوية OBR وفتحه في عارض النظام. العقود غير الرسمية تحمل وسم «مسودة غير موثقة».',
          en: 'From the contracts tab press “New draft”, then “Preview” to review type, parties, property, duration, commission, and dates, and “Print” to build an OBR-branded PDF opened in the system viewer. Unofficial contracts carry an “Unverified draft” badge.'
        }
      },
      {
        title: { ar: 'استيراد PDF وتوثيق الوساطة', en: 'PDF import and brokerage documentation' },
        body: {
          ar: '«استيراد PDF موثق» يحفظ الملف الأصلي دون تعديل مع معاينته وفتحه من الجدول أو التفاصيل. توثيق عقد الوساطة/التسويق يتم يدويًا عبر زر «متابعة توثيق عقد الوساطة» في البوابة الرسمية للهيئة العامة للعقار — لا توجد بيانات دخول أو ربط API.',
          en: '“Import documented PDF” keeps the original file unmodified with in-app preview and opening from the table or details. Brokerage/marketing contracts are documented manually via “Continue brokerage contract documentation” on the Real Estate General Authority portal — no login data or API linkage.'
        }
      }
    ],
    steps: [
      {
        ar: 'زر + أعلى القائمة يفتح إضافة مرتبة: فاتورة، عملية، عمولة، مصروف، دفعة، أو عقد جديد.',
        en: 'The + button above the list opens an ordered add menu: invoice, transaction, commission, expense, payment, or a new contract.'
      },
      {
        ar: 'سجّل العملية أو الفاتورة ثم حدّث حالة التحصيل.',
        en: 'Record the transaction or invoice, then update collection status.'
      },
      {
        ar: 'وزّع نسب العمولة بحيث يكون المجموع 100٪.',
        en: 'Allocate commission shares so they total 100%.'
      },
      {
        ar: 'ملخص العقود يظهر كجدول في تبويب العقود وفي نظرة عامة المالية؛ أنشئ تقريرًا أو فاتورة للطباعة أو الحفظ من نافذة النظام.',
        en: 'The contracts summary appears as a table in the contracts tab and in the finance overview; create a report or invoice to print or save from the system dialog.'
      }
    ]
  },
  {
    id: 'brochures',
    title: { ar: 'مكتبة العروض', en: 'Offer Library' },
    summary: {
      ar: 'مخزن التصاميم والبروشورات التي حفظتها من محرر التصميم.',
      en: 'Storage for designs and brochures you saved from the design editor.'
    },
    steps: [
      {
        ar: 'افتح تصميمًا محفوظًا للمعاينة أو التعديل.',
        en: 'Open a saved design to preview or edit.'
      },
      {
        ar: 'احذف ما لا تحتاجه؛ الحذف من المكتبة لا يحذف العرض العقاري نفسه.',
        en: 'Delete what you don’t need; deleting from the library does not delete the listing itself.'
      }
    ]
  },
  {
    id: 'offer-design',
    title: { ar: 'تصميم وطباعة العروض', en: 'Offer Design & Print' },
    summary: {
      ar: 'اختر قالبًا، اربط عرضًا، عدّل النصوص والصور، ثم احفظ في المكتبة أو اطبع.',
      en: 'Choose a template, link a listing, edit text and images, then save to the library or print.'
    },
    steps: [
      {
        ar: 'اختر قالبًا ومقاس ورقة.',
        en: 'Choose a template and paper size.'
      },
      {
        ar: 'اربط عرضًا لتعبئة الحقول تلقائيًا قدر الإمكان.',
        en: 'Link a listing to auto-fill fields as much as possible.'
      },
      {
        ar: 'احفظ في المكتبة أو صدّر PDF / Word / صورة، أو اطبع عبر نافذة النظام.',
        en: 'Save to the library, export PDF / Word / image, or print via the system dialog.'
      }
    ]
  },
  {
    id: 'database',
    title: { ar: 'قاعدة البيانات', en: 'Database' },
    summary: {
      ar: 'تصدير واستيراد يدوي، ونسخ احتياطية مرقّمة بتاريخ يمكن استعادتها من الواجهة.',
      en: 'Manual export/import, and numbered dated backups you can restore from the UI.'
    },
    steps: [
      {
        ar: 'صدّر بياناتك يدويًا إلى ملف JSON عبر اختيار مكان الحفظ.',
        en: 'Export your data manually to a JSON file by choosing where to save it.'
      },
      {
        ar: 'استورد ملف JSON بنفس امتداد وصيغة التصدير لإضافة السجلات.',
        en: 'Import a JSON file in the same export format to add records.'
      },
      {
        ar: 'أنشئ نسخة احتياطية برقم وتاريخ عند الحاجة (تُحفظ في مجلد النسخ على الجهاز).',
        en: 'Create a numbered, dated backup when needed (saved in the device backups folder).'
      }
    ]
  },
  {
    id: 'settings',
    title: { ar: 'الإعدادات', en: 'Settings' },
    summary: {
      ar: 'هوية المكتب، المظهر، الترخيص، النماذج المدمجة، والبيانات التدريبية الاختيارية.',
      en: 'Office identity, appearance, license, built-in templates, and optional training data.'
    },
    steps: [
      {
        ar: 'حدّث اسم المكتب والترخيص / السجل التجاري وبيانات التواصل.',
        en: 'Update the office name, Fal license, and contact details.'
      },
      {
        ar: 'اختر المظهر المناسب لعينك.',
        en: 'Choose the appearance that suits you.'
      },
      {
        ar: '«تحميل بيانات تدريبية» اختياري ويمكن حذفه بالكامل دون المساس بسجلاتك.',
        en: '“Load training data” is optional and can be removed entirely without affecting your records.'
      }
    ]
  },
  {
    id: 'help',
    title: { ar: 'التعليمات', en: 'Help' },
    summary: {
      ar: 'دليل مختصر لأهم أقسام التطبيق وكيفية استخدامها يوميًا.',
      en: 'A short guide to the main app sections and how to use them day to day.'
    },
    steps: [
      {
        ar: 'اقرأ شرح القسم الذي تعمل عليه الآن.',
        en: 'Read the guide for the section you are working in now.'
      },
      {
        ar: 'ارجع هنا عند الشك قبل تجربة زرٍّ غير واضح.',
        en: 'Come back here when unsure before trying an unclear button.'
      }
    ]
  }
];

function pick(row: Bilingual): string {
  return getUiLanguage() === 'en' ? row.en : row.ar;
}

function localizeGuide(raw: RawSectionGuide): SectionGuide {
  const guide: SectionGuide = {
    id: raw.id,
    title: pick(raw.title),
    summary: pick(raw.summary),
    steps: raw.steps.map(pick)
  };
  if (raw.blocks && raw.blocks.length > 0) {
    guide.blocks = raw.blocks.map((b) => ({
      title: pick(b.title),
      body: pick(b.body)
    }));
  }
  return guide;
}

function localizedGuides(): SectionGuide[] {
  return RAW_SECTION_GUIDES.map(localizeGuide);
}

/** Always localized for the current UI language (re-read on each access / render). */
export const SECTION_GUIDES: SectionGuide[] = new Proxy([] as SectionGuide[], {
  get(_target, prop) {
    const list = localizedGuides();
    const value = Reflect.get(list, prop, list);
    return typeof value === 'function' ? (value as (...args: unknown[]) => unknown).bind(list) : value;
  },
  has(_target, prop) {
    return Reflect.has(localizedGuides(), prop);
  },
  ownKeys() {
    return Reflect.ownKeys(localizedGuides());
  },
  getOwnPropertyDescriptor(_target, prop) {
    return Reflect.getOwnPropertyDescriptor(localizedGuides(), prop);
  }
});

export function guideById(id: string): SectionGuide | undefined {
  const raw = RAW_SECTION_GUIDES.find((g) => g.id === id);
  return raw ? localizeGuide(raw) : undefined;
}
