//! Official OBR training batch (`obr-training-v1`) — relational seed data.
//! العروض العقارية (properties / property_images / property_updates) وسورها
//! محفوظة كما هي؛ بقية البيانات التشغيلية بيانات تدريبية معاد بناؤها.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use rusqlite::{params, Connection};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use super::demo::{batch_counts_map, DEMO_BATCH_ID};
use super::files::FileStore;

const BATCH: &str = DEMO_BATCH_ID;
const TRAINING: i64 = 1;

/// جذور صور العروض التسويقية الحقيقية المضمّنة مع التثبيت (resources/demo).
/// تُضبط عند الإقلاع؛ في Development تشمل مجلد المصدر داخل src-tauri.
static DEMO_IMAGE_SOURCES: RwLock<Vec<PathBuf>> = RwLock::new(Vec::new());

pub fn set_demo_image_sources(dirs: Vec<PathBuf>) {
    if let Ok(mut guard) = DEMO_IMAGE_SOURCES.write() {
        *guard = dirs;
    }
}

fn find_demo_image(file_name: &str) -> Option<PathBuf> {
    if let Ok(guard) = DEMO_IMAGE_SOURCES.read() {
        for dir in guard.iter() {
            let candidate = dir.join(file_name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn svg_data_url(accent: &str, label: &str) -> String {
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="800" height="600" viewBox="0 0 800 600">
<rect width="800" height="600" fill="{accent}"/>
<text x="400" y="300" fill="white" font-size="32" text-anchor="middle" font-family="sans-serif">{label}</text>
<text x="400" y="340" fill="gold" font-size="16" text-anchor="middle" font-family="sans-serif">OBR Training</text>
</svg>"#,
        accent = accent,
        label = label
    );
    format!(
        "data:image/svg+xml;base64,{}",
        STANDARD.encode(svg.as_bytes())
    )
}

fn store_training_image(
    store: &FileStore,
    user_id: &str,
    entity_id: &str,
    file_id: &str,
    name: &str,
    svg_label: &str,
    accent: &str,
    tx: &rusqlite::Transaction,
) -> Result<String, String> {
    let svg = svg_data_url(accent, svg_label);
    let bytes = svg.as_bytes();
    let (relative, stored) = store.store_bytes(name, bytes, "training/obr-training-v1")?;
    tx.execute(
        "INSERT INTO file_assets (id, user_id, entity_type, entity_id, original_name, stored_name, relative_path, mime_type, size_bytes, seed_batch_id, is_training)
         VALUES (?1,?2,'property',?3,?4,?5,?6,'image/svg+xml',?7,?8,?9)",
        params![file_id, user_id, entity_id, name, stored, relative, bytes.len() as i64, BATCH, TRAINING],
    )
    .map_err(|e| e.to_string())?;
    Ok(svg)
}

/// تخزين صورة عرض تسويقي حقيقية (PNG) كملف فعلي تحت properties/ —
/// نفس مسار صور العقارات العادية، وتُقرأ عبر property_image_read.
fn store_real_training_image(
    store: &FileStore,
    user_id: &str,
    entity_id: &str,
    file_id: &str,
    source: &Path,
    tx: &rusqlite::Transaction,
) -> Result<String, String> {
    let bytes = std::fs::read(source).map_err(|e| format!("فشل قراءة صورة التدريب: {e}"))?;
    if bytes.is_empty() {
        return Err("صورة التدريب فارغة".into());
    }
    let name = source
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "obr-training.png".into());
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png")
        .to_ascii_lowercase();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => "image/jpeg",
    };
    let (relative, stored) = store.store_bytes(&name, &bytes, "properties")?;
    tx.execute(
        "INSERT INTO file_assets (id, user_id, entity_type, entity_id, original_name, stored_name, relative_path, mime_type, size_bytes, seed_batch_id, is_training)
         VALUES (?1,?2,'property',?3,?4,?5,?6,?7,?8,?9,?10)",
        params![file_id, user_id, entity_id, name, stored, relative, mime, bytes.len() as i64, BATCH, TRAINING],
    )
    .map_err(|e| e.to_string())?;
    Ok(relative)
}

/// Inserts the full training batch. Caller must ensure batch does not already exist.
pub fn insert_training_batch(
    conn: &mut Connection,
    user_id: &str,
    app_data_dir: &Path,
) -> Result<BTreeMap<String, i64>, String> {
    let store = FileStore::new(app_data_dir)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    tx.execute(
        "INSERT INTO seed_batches(id, label, created_at, notes) VALUES (?1, ?2, datetime('now'), ?3)",
        params![
            BATCH,
            "بيانات تدريب OBR الرسمية",
            "obr-training-v1 — is_training=1 — للتسويق والتدريب فقط"
        ],
    )
    .map_err(|e| e.to_string())?;

    // ===== العملاء: مجموعة تدريبية — 4 عملاء مشترين =====
    // بيانات اتصال تدريبية موحدة: الجوال 0500000000 (غير قابل للاتصال فعليًا).
    let clients: [(&str, &str, &str, &str, &str, &str, &str, &str); 4] = [
        (
            "train-c-abdullah",
            "عبدالله",
            "abdullah.training@example.invalid",
            "مشتري — سكن",
            "الرياض",
            "buyer",
            "عميل تدريبي — مهتم بشراء عقار سكني",
            "-6 days",
        ),
        (
            "train-c-abdulrahman",
            "عبدالرحمن",
            "abdulrahman.training@example.invalid",
            "مشتري — سكن",
            "جدة",
            "buyer",
            "عميل تدريبي — يبحث عن فيلا سكنية",
            "-4 days",
        ),
        (
            "train-c-abdulmajeed",
            "عبدالمجيد",
            "abdulmajeed.training@example.invalid",
            "مشتري — سكن",
            "الدمام",
            "buyer",
            "عميل تدريبي — مهتم بعقار سكني",
            "-3 days",
        ),
        (
            "train-c-abdulkareem",
            "عبدالكريم",
            "abdulkareem.training@example.invalid",
            "مشتري — سكن",
            "الرياض",
            "buyer",
            "عميل تدريبي — يفاضل على عقار للشراء",
            "-1 days",
        ),
    ];

    for (id, name, email, company, city, rel, notes, last) in clients {
        tx.execute(
            "INSERT INTO clients (id, user_id, name, phone, whatsapp, email, company, relationship_type, city, source, status, category, interests, required_property_type, budget, notes, last_contact, follow_up_date, seed_batch_id, is_training)
             VALUES (?1,?2,?3,'0500000000','0500000000',?4,?5,?6,?7,'referral','active','A','عروض OBR التدريبية','فلة','1500000-4500000',?8, date('now', ?9), date('now','+5 days'), ?10, ?11)",
            params![id, user_id, name, email, company, rel, city, format!("{notes} — بيانات تدريبية"), last, BATCH, TRAINING],
        )
        .map_err(|e| format!("client {id}: {e}"))?;
    }

    let roles: [(&str, &str); 4] = [
        ("train-c-abdullah", "buyer"),
        ("train-c-abdulrahman", "buyer"),
        ("train-c-abdulmajeed", "buyer"),
        ("train-c-abdulkareem", "buyer"),
    ];
    for (cid, role) in roles {
        tx.execute(
            "INSERT INTO client_roles (id, client_id, role, seed_batch_id, is_training) VALUES (?1,?2,?3,?4,?5)",
            params![format!("train-r-{cid}-{role}"), cid, role, BATCH, TRAINING],
        )
        .map_err(|e| e.to_string())?;
    }

    struct PropSeed {
        id: &'static str,
        name: &'static str,
        city: &'static str,
        district: &'static str,
        ptype: &'static str,
        purpose: &'static str,
        owner: &'static str,
        area: f64,
        price: f64,
        lat: f64,
        lon: f64,
        status: &'static str,
        accent: &'static str,
        img_label: &'static str,
        /// صورة العرض التسويقي الحقيقية المضمّنة مع التثبيت (اختيارية — SVG احتياطي)
        demo_img: Option<&'static str>,
    }

    let props = [
        PropSeed {
            id: "train-p-land",
            name: "OBR أرض للبيع",
            city: "الرياض",
            district: "الياسمين",
            ptype: "أرض",
            purpose: "sale",
            owner: "train-c-abdullah",
            area: 750.0,
            price: 2_250_000.0,
            lat: 24.8302,
            lon: 46.6390,
            status: "available",
            accent: "#2d6a4f",
            img_label: "أرض",
            demo_img: Some("obr-poster.png"),
        },
        PropSeed {
            id: "train-p-villa",
            name: "OBR فيلا للبيع",
            city: "جدة",
            district: "الروضة",
            ptype: "فلة",
            purpose: "sale",
            owner: "train-c-abdulrahman",
            area: 420.0,
            price: 3_850_000.0,
            lat: 21.5433,
            lon: 39.1728,
            status: "negotiating",
            accent: "#1d3557",
            img_label: "فلة",
            demo_img: Some("obr-brochure.png"),
        },
        PropSeed {
            id: "train-p-apt",
            name: "OBR شقة للبيع",
            city: "الدمام",
            district: "الفيصلية",
            ptype: "شقة",
            purpose: "sale",
            owner: "train-c-abdulmajeed",
            area: 165.0,
            price: 980_000.0,
            lat: 26.3927,
            lon: 49.9777,
            status: "available",
            accent: "#6a4c93",
            img_label: "شقة",
            demo_img: Some("obr-box.png"),
        },
    ];

    for p in props {
        let ppm = p.price / p.area;
        // الصورة الحقيقية المضمّنة أولًا؛ SVG الاحتياطي يُحفظ كصورة إضافية في المعرض
        let real_img = p
            .demo_img
            .and_then(find_demo_image)
            .map(|src| {
                store_real_training_image(
                    &store,
                    user_id,
                    p.id,
                    &format!("train-file-{}", p.id),
                    &src,
                    &tx,
                )
            })
            .transpose()?;
        let img = match &real_img {
            Some(rel) => rel.clone(),
            None => store_training_image(
                &store,
                user_id,
                p.id,
                &format!("train-file-{}", p.id),
                &format!("{}.svg", p.id),
                p.img_label,
                p.accent,
                &tx,
            )?,
        };
        tx.execute(
            "INSERT INTO properties (id, user_id, name, city, district, location, property_type, purpose, source_type, owner_client_id, area, price, price_per_meter, latitude, longitude, description, commission_agreement, fal_ad_license, status, main_image, listing_expiry_date, seed_batch_id, is_training)
             VALUES (?1,?2,?3,?4,?5,?5,?6,?7,'direct_owner',?8,?9,?10,?11,?12,?13,'عرض تدريبي OBR — is_training','2.5%','7200000001',?14,?15, date('now','+90 days'), ?16, ?17)",
            params![
                p.id, user_id, p.name, p.city, p.district, p.ptype, p.purpose, p.owner, p.area, p.price, ppm, p.lat, p.lon, p.status, img, BATCH, TRAINING
            ],
        )
        .map_err(|e| format!("property {}: {e}", p.id))?;

        tx.execute(
            "INSERT INTO property_images (id, property_id, image_url, seed_batch_id, is_training) VALUES (?1,?2,?3,?4,?5)",
            params![format!("train-img-{}", p.id), p.id, img, BATCH, TRAINING],
        )
        .map_err(|e| e.to_string())?;

        if real_img.is_some() {
            let svg_img = store_training_image(
                &store,
                user_id,
                p.id,
                &format!("train-file-svg-{}", p.id),
                &format!("{}.svg", p.id),
                p.img_label,
                p.accent,
                &tx,
            )?;
            tx.execute(
                "INSERT INTO property_images (id, property_id, image_url, seed_batch_id, is_training) VALUES (?1,?2,?3,?4,?5)",
                params![format!("train-img-svg-{}", p.id), p.id, svg_img, BATCH, TRAINING],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    // ===== العلاقات مع العروض الحالية (بدون تعديل العروض نفسها) =====
    let links = [
        ("train-l-1", "train-c-abdullah", "train-p-land", "interested"),
        ("train-l-2", "train-c-abdullah", "train-p-villa", "viewing_request"),
        ("train-l-3", "train-c-abdulrahman", "train-p-villa", "purchase_request"),
        ("train-l-4", "train-c-abdulrahman", "train-p-apt", "interested"),
        ("train-l-5", "train-c-abdulmajeed", "train-p-apt", "viewing_request"),
        ("train-l-6", "train-c-abdulmajeed", "train-p-land", "bid"),
        ("train-l-7", "train-c-abdulkareem", "train-p-villa", "interested"),
        ("train-l-8", "train-c-abdulkareem", "train-p-apt", "purchase_request"),
    ];
    for (id, cid, pid, ltype) in links {
        tx.execute(
            "INSERT INTO client_property_links (id, client_id, property_id, link_type, seed_batch_id, is_training) VALUES (?1,?2,?3,?4,?5,?6)",
            params![id, cid, pid, ltype, BATCH, TRAINING],
        )
        .map_err(|e| e.to_string())?;
    }

    // ===== سجل التواصل — أنواع الواجهة المعروفة: call / whatsapp / message / meeting / email / note =====
    let comms: [(&str, &str, &str, &str, &str, i64); 10] = [
        ("train-m-1", "train-c-abdullah", "call", "استفسار عن فيلا للسكن", "مكالمة تمهيدية — بيانات تدريبية", -30),
        ("train-m-2", "train-c-abdullah", "whatsapp", "إرسال عرض فيلا جدة", "أُرسل بروشور العرض عبر واتساب — بيانات تدريبية", -7),
        ("train-m-3", "train-c-abdulrahman", "whatsapp", "طلب معاينة الفيلا", "طلب موعد معاينة للفيلا — بيانات تدريبية", -5),
        ("train-m-4", "train-c-abdulrahman", "call", "متابعة عرض شقة الدمام", "مكالمة متابعة بعد إرسال العرض — بيانات تدريبية", -9),
        ("train-m-5", "train-c-abdulmajeed", "email", "طلب شراء أرض الرياض", "بريد يطلب تفاصيل الأرض والسعر — بيانات تدريبية", -10),
        ("train-m-6", "train-c-abdulmajeed", "meeting", "اجتماع شراء أرض الرياض", "اجتماع مناقشة شروط الشراء — بيانات تدريبية", -6),
        ("train-m-7", "train-c-abdulkareem", "whatsapp", "استفسار عن شقة الدمام", "استفسار عن المساحة والسعر — بيانات تدريبية", -8),
        ("train-m-8", "train-c-abdulkareem", "call", "متابعة طلب الشراء", "متابعة رغبة الشراء النهائية — بيانات تدريبية", -2),
        ("train-m-9", "train-c-abdullah", "meeting", "معاينة فيلا جدة", "معاينة ميدانية للفيلا — بيانات تدريبية", -4),
        ("train-m-10", "train-c-abdulrahman", "email", "تفاصيل التمويل العقاري", "إرسال خيارات التمويل المتاحة — بيانات تدريبية", -3),
    ];
    for (id, cid, ctype, subject, notes, days) in comms {
        tx.execute(
            "INSERT INTO communications (id, user_id, client_id, type, subject, notes, created_at, seed_batch_id, is_training) VALUES (?1,?2,?3,?4,?5,?6, datetime('now', ?7), ?8, ?9)",
            params![id, user_id, cid, ctype, subject, notes, format!("{days} days"), BATCH, TRAINING],
        )
        .map_err(|e| e.to_string())?;
    }

    let updates = [
        ("train-u-1", "train-p-land", "تحديث السعر", "مراجعة طلب المالك — تدريبي"),
        ("train-u-2", "train-p-villa", "معاينة", "زيارة ميدانية — تدريبي"),
        ("train-u-3", "train-p-apt", "تسويق", "نشر على المنصات — تدريبي"),
    ];
    for (id, pid, title, desc) in updates {
        tx.execute(
            "INSERT INTO property_updates (id, property_id, user_id, title, description, seed_batch_id, is_training) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![id, pid, user_id, title, desc, BATCH, TRAINING],
        )
        .map_err(|e| e.to_string())?;
    }

    // ===== المؤشر الخاص / بيانات السوق — أرقام تعليمية واضحة وليست أسعار سوق رسمية =====
    let cities: [(&str, [&str; 5]); 5] = [
        ("الرياض", ["الياسمين", "النرجس", "الملقا", "العقيق", "حطين"]),
        ("جدة", ["الروضة", "الزهراء", "الشاطئ", "الحمدانية", "السلامة"]),
        ("الدمام", ["الفيصلية", "الشاطئ", "النور", "الأمانة", "الجلوية"]),
        ("مكة المكرمة", ["العزيزية", "الشوقية", "الزاهر", "النسيم", "العوالي"]),
        ("المدينة المنورة", ["العقيق", "قباء", "الأزهري", "ينبع", "العريض"]),
    ];
    // (النوع، المساحة، سعر المتر الأساسي) — أرقام تعليمية مدورة
    let types = [("أرض", 600.0, 2500.0), ("فلة", 400.0, 8000.0), ("شقة", 140.0, 6000.0)];
    let date_offs = [-15, -45, -90];
    let mut m_idx = 0u32;
    for (city, districts) in cities {
        for (di, district) in districts.iter().enumerate() {
            let (ptype, area, ppm_base) = types[di % types.len()];
            for (d_off, &days) in date_offs.iter().enumerate() {
                m_idx += 1;
                let id = format!("train-mkt-{m_idx:03}");
                // تفاوت تعليمي بسيط حسب الحي والتاريخ — مقصّد ومدور
                let jitter = ((di as i64 * 150) + (d_off as i64 * 100)) as f64;
                let ppm = ((ppm_base + jitter) / 50.0).round() * 50.0;
                let sale = (area * ppm / 500.0).round() * 500.0;
                let ppm = sale / area;
                tx.execute(
                    "INSERT INTO market_reference_data (id, user_id, city, district, property_type, usage_type, area, sale_price, price_per_meter_sale, entry_date, data_source, notes, seed_batch_id, is_training)
                     VALUES (?1,?2,?3,?4,?5,'سكني',?6,?7,?8, date('now', ?9), 'بيانات تدريبية', 'مؤشر تدريبي — أرقام تعليمية وليست أسعار سوق رسمية', ?10, ?11)",
                    params![id, user_id, city, district, ptype, area, sale, ppm, format!("{days} days"), BATCH, TRAINING],
                )
                .map_err(|e| e.to_string())?;
            }
        }
    }

    // ===== الحملات التسويقية — 4 نماذج تدريبية بأسماء عربية واضحة؛ 3 مرتبطة بالعروض الثلاثة الحالية و1 نموذج عام بدون عرض =====
    let campaigns: [(&str, &str, Option<&str>, &str, i64, i64, i64, i64, f64); 4] = [
        ("train-camp-1", "حملة تسويق شقة للبيع", Some("train-p-apt"), "whatsapp", 620, 34, 7, 1, 0.0),
        ("train-camp-2", "حملة عرض فيلا سكنية", Some("train-p-villa"), "aqar", 1450, 22, 4, 1, 250.0),
        ("train-camp-3", "حملة تسويق أرض سكنية", Some("train-p-land"), "twitter", 3300, 41, 6, 1, 400.0),
        ("train-camp-4", "حملة إعلان عقار للإيجار", None, "other", 340, 12, 2, 0, 0.0),
    ];
    for (id, name, pid, platform, views, inq, viewings, deals, cost) in campaigns {
        tx.execute(
            "INSERT INTO marketing_campaigns (id, user_id, name, property_id, platform, ad_link, start_date, end_date, views, inquiries, viewings, deals, cost, notes, seed_batch_id, is_training)
             VALUES (?1,?2,?3,?4,?5,'https://example.invalid/ads', date('now','-28 days'), date('now','-7 days'), ?6,?7,?8,?9,?10,'حملة تدريبية — بيانات تعليمية', ?11, ?12)",
            params![id, user_id, name, pid, platform, views, inq, viewings, deals, cost, BATCH, TRAINING],
        )
        .map_err(|e| e.to_string())?;
    }

    // ===== المالية والعقود — 4 نماذج تدريبية فقط (صفقات وساطة بأسماء عربية واضحة) =====
    let ops: [(&str, &str, i64, f64, &str, &str, Option<&str>); 4] = [
        ("train-fo-1", "صفقة وساطة بيع أرض الرياض", -25, 2_250_000.0, "train-c-abdullah", "train-p-land", Some("train-camp-3")),
        ("train-fo-2", "صفقة وساطة بيع فيلا جدة", -18, 3_850_000.0, "train-c-abdulrahman", "train-p-villa", Some("train-camp-2")),
        ("train-fo-3", "صفقة وساطة بيع شقة الدمام", -12, 980_000.0, "train-c-abdulmajeed", "train-p-apt", Some("train-camp-1")),
        ("train-fo-4", "صفقة وساطة بيع أرض سكنية — الرياض", -7, 1_150_000.0, "train-c-abdulkareem", "train-p-land", Some("train-camp-4")),
    ];
    for (id, title, days, value, cid, pid, camp) in ops {
        tx.execute(
            "INSERT INTO finance_operations (id, user_id, title, occurs_on, deal_value, client_id, property_id, campaign_id, notes, seed_batch_id, is_training)
             VALUES (?1,?2,?3, date('now', ?4), ?5, ?6, ?7, ?8, 'صفقة وساطة تدريبية — عملة SAR', ?9, ?10)",
            params![id, user_id, title, format!("{days} days"), value, cid, pid, camp, BATCH, TRAINING],
        )
        .map_err(|e| e.to_string())?;
    }

    // الفواتير/الدفعات/العمولات/المصاريف: تُزرع فقط من استخدام المستخدم — لا نماذج قديمة.

    // ===== البروشرات — مرتبطة بالعروض الحالية =====
    let brochure_specs = [
        ("train-bro-land", "train-p-land", "royal-sheet", "بروشور تدريبي — أرض الرياض"),
        ("train-bro-villa", "train-p-villa", "gold-luxury", "بروشور تدريبي — فيلا جدة"),
        ("train-bro-apt", "train-p-apt", "modern-split", "بروشور تدريبي — شقة الدمام"),
    ];
    for (id, pid, tpl, title) in brochure_specs {
        let content = serde_json::json!({
            "version": 1,
            "templateId": tpl,
            "templateOrigin": tpl,
            "pageSize": "a4-portrait",
            "propertyId": pid,
            "fields": { "title": title },
            "theme": {}
        });
        tx.execute(
            "INSERT INTO brochures (id, user_id, property_id, template_id, title, content, seed_batch_id, is_training)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![id, user_id, pid, tpl, title, content.to_string(), BATCH, TRAINING],
        )
        .map_err(|e| e.to_string())?;
    }

    // ===== المواعيد — مرتبة ومفهومة =====
    let appts: [(&str, &str, i64, &str); 6] = [
        ("train-appt-1", "معاينة عقار — فيلا جدة (عبدالرحمن)", 2, "17:00"),
        ("train-appt-2", "اتصال متابعة — مشتري مهتم بأرض الرياض", 3, "11:00"),
        ("train-appt-3", "اجتماع شراء — تقييم أرض الياسمين", 5, "14:00"),
        ("train-appt-4", "اجتماع مكتب عقاري — شراكة تسويقية", 7, "10:00"),
        ("train-appt-5", "معاينة عقار — شقة الدمام (عبدالله)", 4, "16:00"),
        ("train-appt-6", "متابعة عقد — بيع فيلا جدة", 9, "12:30"),
    ];
    for (id, title, days, time) in appts {
        tx.execute(
            "INSERT INTO office_appointments (id, user_id, title, occurs_on, starts_at, notes, seed_batch_id, is_training)
             VALUES (?1,?2,?3, date('now', ?4), ?5, 'موعد تدريبي', ?6, ?7)",
            params![id, user_id, title, format!("+{days} days"), time, BATCH, TRAINING],
        )
        .map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;
    batch_counts_map(conn, BATCH)
}
