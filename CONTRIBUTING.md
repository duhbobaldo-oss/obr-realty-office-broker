<div dir="rtl">

# دليل المساهمة في مكتب الوسيط العقاري - OBR

شكرًا لاهتمامك بتطوير التطبيق. المشروع مفتوح للمساهمة في إصلاح الأخطاء وإضافة الميزات وتحسين الواجهة والترجمة والتوثيق.

## خطوات المساهمة

1. افتح Issue يصف المشكلة أو الميزة المقترحة، أو علّق على Issue قائم قبل البدء في عمل كبير.
2. اعمل Fork للمستودع، وأنشئ فرعًا باسم واضح مثل `fix/market-index-rounding` أو `feature/kuwait-profile`.
3. نفّذ التعديل بأصغر تغيير ممكن، مع الحفاظ على الوظائف القائمة.
4. تأكد من نجاح البناء والاختبارات:
   ```bash
   npm install
   npm run build --workspace=desktop
   cd desktop/src-tauri && cargo test
   ```
5. افتح Pull Request يشرح ما تغيّر ولماذا، وكيف تحققت منه.

## قواعد مهمة

- **لغتان:** أي نص جديد في الواجهة يُضاف بالعربية والإنجليزية، مع مراعاة اتجاه RTL.
- **دول الخليج:** راعِ اختلاف العملة والمدن والإعدادات بين السعودية والإمارات والكويت وقطر والبحرين وعُمان.
- **لا بيانات حقيقية ولا أسرار:** لا تضف بيانات عملاء حقيقية أو كلمات مرور أو مفاتيح أو رموز وصول إلى الكود أو الاختبارات.
- **لا نجاح وهمي:** أي زر أو ميزة يجب أن تعمل فعليًا، ولا تُعرض رسالة نجاح قبل نجاح العملية.
- **البيانات محلية:** لا تجعل الوظائف الأساسية تعتمد على الإنترنت أو على حساب.

## الإبلاغ عن ثغرة أمنية

لا تنشر الثغرات الأمنية في Issue عام. أرسلها عبر الموقع الرسمي: https://obr.obr432.workers.dev/

## الرخصة

بإرسال مساهمتك توافق على ترخيصها بموجب [رخصة Apache 2.0](LICENSE) نفسها.

</div>

---

# Contributing to OBR Realty Office Broker

Thank you for your interest. Bug fixes, features, interface, translation and documentation improvements are welcome.

1. Open an issue describing the bug or feature before larger work.
2. Fork the repository and create a descriptive branch.
3. Keep changes focused and preserve existing behaviour.
4. Make sure `npm run build --workspace=desktop` and `cargo test` (in `desktop/src-tauri`) pass.
5. Open a pull request explaining what changed, why, and how you verified it.

Rules: every new interface text in Arabic and English (RTL-aware); respect differences between the Gulf countries; no real customer data or secrets; no simulated success; core features stay local and account-free.

Report security vulnerabilities privately through https://obr.obr432.workers.dev/, not in public issues.

By contributing you agree that your contribution is licensed under the [Apache License 2.0](LICENSE).
