# OBR Campaign Tracking Endpoint

خدمة HTTPS خارجية لتسجيل **نقرات رابط التتبع** وإعادة التوجيه (302) إلى رابط الإعلان الأصلي.

- **ليست** جزءًا من Runtime تطبيق OBR Desktop.
- **Production (الافتراضي في OBR):** `https://obr-track.obr432.workers.dev`

## البروتوكول v2 — بلا مفتاح (المستخدم من OBR Desktop)

القاعدة: `Desktop = untrusted public client` / `Server = owns secrets`.
التطبيق **لا يحمل أي Secret**؛ يحدد نفسه بترويستي:

- `X-OBR-Installation` — installation_id (تحديد لحدود الاستخدام، وليس مصادقة)
- `X-OBR-Product` — معرف المنتج

| Method | Path | الغرض |
|--------|------|--------|
| GET | `/health` | صحة الخدمة |
| POST | `/v2/links` | إنشاء رابط — Body: `{ "destination_url": "https://...", "installation_id": "...", "product_id": "..." }` — **الرمز يُولَّد على الخادم** (48 hex) |
| PATCH | `/v2/links/:code` | تحديث الوجهة / تعطيل وتمكين — Body: `{ "original_ad_url"?, "enabled"? }` — للمالك فقط |
| GET | `/v2/links/:code/stats` | إحصاءات — للمالك فقط |
| GET | `/t/:code` | تسجيل النقرة ثم 302 إلى الوجهة (عام، بلا هوية) |

حمايات الخادم (v2):

- ملكية الرابط: `owner_installation` يُسجل عند الإنشاء؛ PATCH و stats يُرفضان بـ 403 من تثبيت آخر.
- حدود معدل في الذاكرة لكل IP ولكل تثبيت (429 عند التجاوز).
- حصة قصوى من الروابط لكل تثبيت.
- التحقق من تطابق `installation_id` / `product_id` بين الترويسات والجسم.
- الوجهة يجب أن تكون `https://`.

## توافق قديم (v1)

المسارات `/v1/...` و`/api/...` (مع `X-OBR-Key`) بقيت للتوافق الخلفي فقط ولم يعُد OBR Desktop يستخدمها.
`HASH_SALT` / `OBR_API_KEY` أسرار خاصة بالخادم فقط — لا تُضمَّن داخل تطبيق OBR Desktop إطلاقًا.

## تشغيل محلي

```bash
cd tracking-endpoint
npm install
npm start        # http://127.0.0.1:8787
npm test
```

اختبار حي شامل من تطبيق Desktop:

```bash
cd desktop/src-tauri
set OBR_TRACKING_ENDPOINT=http://127.0.0.1:8787
cargo test live_v2_tracking_loop -- --ignored --nocapture
```
