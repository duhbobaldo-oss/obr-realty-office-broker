//! عقود OBR: مزود يدوي فقط. لا توجد بيانات دخول ولا أتمتة للمتصفح.
//! - عقود الوساطة/التسويق: الخدمة الرسمية لعقود الوساطة العقارية (الهيئة العامة للعقار).
//! - لا يُستخدم رابط إيجار لعقود الوساطة إطلاقًا.
//! - عقود إدارة الأملاك/الإيجار (مستقبلًا فقط): مزود مستقل لمنصة إيجار.
use base64::Engine;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use uuid::Uuid;
use std::path::Path;
use crate::db::Settings;

/// الخدمة الرسمية لعقود الوساطة العقارية — الهيئة العامة للعقار (إبرام عقود الوساطة العقارية).
pub const REGA_BROKERAGE_CONTRACTS_URL: &str = "https://eservicesredp.rega.gov.sa/main";

/// مزود مستقبلي مستقل لمنصة إيجار — يُستخدم فقط لعقود إدارة الأملاك/الإيجار عند إضافتها.
/// لا يُستخدم لعقود الوساطة/التسويق إطلاقًا.
pub const EJAR_PROPERTY_MANAGEMENT_URL: &str = "https://rega.gov.sa/rega-services/platforms/ejar/";

/// نقطة التمديد الموحدة لتكاملات العقود. كل مزود يصف قدرته فقط؛ لا يحمل
/// بيانات دخول ولا يتصل بأي واجهة غير معلنة.
pub trait ContractIntegrationProvider {
    fn provider_id(&self) -> &'static str;
    fn mode(&self) -> &'static str;
    fn documentation_url(&self) -> Option<&'static str>;
    fn enabled(&self) -> bool { true }
}

/// تجهيز المسودة في OBR ثم فتح البوابة الرسمية بقرار المستخدم.
pub struct ManualProvider;
impl ContractIntegrationProvider for ManualProvider {
    fn provider_id(&self) -> &'static str { "manual" }
    fn mode(&self) -> &'static str { "manual" }
    fn documentation_url(&self) -> Option<&'static str> { Some(REGA_BROKERAGE_CONTRACTS_URL) }
}

/// استيراد PDF فقط: لا OCR ولا تخمين للحقول غير المؤكدة.
pub struct PdfImportProvider;
impl ContractIntegrationProvider for PdfImportProvider {
    fn provider_id(&self) -> &'static str { "pdf_import" }
    fn mode(&self) -> &'static str { "pdf_import" }
    fn documentation_url(&self) -> Option<&'static str> { None }
}

/// مساعد البوابة: Mapping دلالي وتجهيز قيم قابلة للمراجعة والنسخ فقط.
/// لا يعرف عناصر HTML ولا ينفذ كتابة أو نقرًا داخل البوابة.
pub struct PortalAssistProvider;
impl ContractIntegrationProvider for PortalAssistProvider {
    fn provider_id(&self) -> &'static str { "portal_assist" }
    fn mode(&self) -> &'static str { "copy_assist_only" }
    fn documentation_url(&self) -> Option<&'static str> { Some(REGA_BROKERAGE_CONTRACTS_URL) }
}

/// واجهة محجوزة لتكامل REGA رسمي فقط. لا توجد أي طلبات شبكة في هذا الإصدار.
pub struct RegaApiProvider;
impl ContractIntegrationProvider for RegaApiProvider {
    fn provider_id(&self) -> &'static str { "rega_api" }
    fn mode(&self) -> &'static str { "future_official_api" }
    fn documentation_url(&self) -> Option<&'static str> { Some(REGA_BROKERAGE_CONTRACTS_URL) }
    fn enabled(&self) -> bool { false }
}
/// مزود مستقبلي مستقل لعقود إدارة الأملاك/الإيجار عبر منصة إيجار.
pub struct EjarPropertyManagementProvider;
impl ContractIntegrationProvider for EjarPropertyManagementProvider {
    fn provider_id(&self) -> &'static str { "ejar" }
    fn mode(&self) -> &'static str { "future" }
    fn documentation_url(&self) -> Option<&'static str> { Some(EJAR_PROPERTY_MANAGEMENT_URL) }
    fn enabled(&self) -> bool { false }
}

const PORTAL_FIELD_MAPPING: [(&str, &str, &str); 6] = [
    ("contract_type", "النوع", "contract_type"),
    ("parties", "الأطراف", "parties"),
    ("property_name", "العقار", "property_reference"),
    ("contract_period", "المدة", "contract_period"),
    ("commission", "العمولة", "commission"),
    ("contract_scope", "نطاق العقد", "contract_scope"),
];

fn provider_descriptor(provider: &dyn ContractIntegrationProvider) -> Value {
    json!({
        "id": provider.provider_id(),
        "mode": provider.mode(),
        "enabled": provider.enabled(),
        "documentation_url": provider.documentation_url(),
    })
}

fn text(v: &Value, key: &str) -> String { v.get(key).and_then(Value::as_str).unwrap_or("").trim().to_string() }
fn num(v: &Value, key: &str) -> Option<f64> { v.get(key).and_then(Value::as_f64) }
fn nullable(s: String) -> Option<String> { if s.is_empty() { None } else { Some(s) } }
fn json_field(v: &Value, key: &str, fallback: Value) -> String { serde_json::to_string(v.get(key).unwrap_or(&fallback)).unwrap_or_else(|_| fallback.to_string()) }
fn esc(s: &str) -> String { s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;") }
fn display(v: &Value, key: &str) -> String { text(v, key) }

pub fn list(conn: &Connection, user_id: &str) -> Result<Value, String> {
    let mut stmt = conn.prepare("SELECT c.*, p.name AS joined_property_name, cl.name AS joined_client_name, fa.original_name AS pdf_name, fa.relative_path AS pdf_path FROM contracts c LEFT JOIN properties p ON p.id=c.property_id LEFT JOIN clients cl ON cl.id=c.client_id LEFT JOIN file_assets fa ON fa.id=c.pdf_asset_id WHERE c.user_id=?1 ORDER BY c.updated_at DESC").map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![user_id], |r| {
        let pf: Value = serde_json::from_str(&r.get::<_,String>(19)?).unwrap_or(json!({}));
        let join_property: Option<String> = r.get("joined_property_name")?;
        let join_client: Option<String> = r.get("joined_client_name")?;
        Ok(json!({
        "id": r.get::<_,String>(0)?, "user_id": r.get::<_,String>(1)?, "contract_number": r.get::<_,Option<String>>(2)?, "contract_type": r.get::<_,Option<String>>(3)?, "contract_scope": r.get::<_,Option<String>>(4)?, "parties": serde_json::from_str::<Value>(&r.get::<_,String>(5)?).unwrap_or(json!([])),
        "property_id": r.get::<_,Option<String>>(6)?, "client_id": r.get::<_,Option<String>>(7)?, "campaign_id": r.get::<_,Option<String>>(8)?, "commission_id": r.get::<_,Option<String>>(9)?, "invoice_id": r.get::<_,Option<String>>(10)?, "start_date": r.get::<_,Option<String>>(11)?, "end_date": r.get::<_,Option<String>>(12)?, "status": r.get::<_,String>(13)?, "commission_amount": r.get::<_,Option<f64>>(14)?, "commission_mechanism": r.get::<_,Option<String>>(15)?, "license_number": r.get::<_,Option<String>>(16)?, "pdf_asset_id": r.get::<_,Option<String>>(17)?, "source": r.get::<_,String>(18)?, "provider_fields_json": pf, "extracted_text_excerpt": r.get::<_,Option<String>>(20)?, "created_at": r.get::<_,String>(21)?, "updated_at": r.get::<_,String>(22)?,
        // عرض أسماء العقار/العميل المستخرجة من PDF عندما لا يوجد معرف مربوط — طبقة عرض فقط
        "property_name": join_property.or_else(|| pf.get("property_name").and_then(Value::as_str).map(|s| s.to_string())),
        "client_name": join_client.or_else(|| pf.get("client_name").and_then(Value::as_str).map(|s| s.to_string())),
        "pdf_name": r.get::<_,Option<String>>("pdf_name")?, "pdf_path": r.get::<_,Option<String>>("pdf_path")?,
        "country": r.get::<_,String>("country")?, "currency": r.get::<_,String>("currency")?,
        "broker_license_type":r.get::<_,Option<String>>("broker_license_type")?,
        "broker_license_name":r.get::<_,Option<String>>("broker_license_name")?,
        "broker_license_number":r.get::<_,Option<String>>("broker_license_number")?
    }))
    }).map_err(|e| e.to_string())?;
    let mut items=Vec::new(); for row in rows { items.push(row.map_err(|e|e.to_string())?); } Ok(json!({"items":items,"total":items.len()}))
}

/// معاينة محافظة: تُعيد نصًا متاحًا فقط، ولا تملأ حقول العقد ما لم يظهر مفتاح صريح.
pub fn preview_pdf(data_url: &str) -> Result<Value, String> {
    let (prefix, encoded) = data_url.split_once(',').ok_or("ملف PDF غير صالح")?;
    if !prefix.contains("application/pdf") { return Err("يُقبل PDF فقط".into()); }
    let bytes = base64::engine::general_purpose::STANDARD.decode(encoded).map_err(|_| "تعذر قراءة ملف PDF")?;
    if !bytes.starts_with(b"%PDF-") { return Err("الملف ليس PDF صالحًا".into()); }

    // استخراج النص الحقيقي فقط (فك ضغط FlateDecode + ToUnicode عند توفره).
    // لا تُعرض bytes الداخلية للـPDF للمستخدم مطلقًا.
    let text = pdf_readable_text(&bytes).unwrap_or_default();
    let excerpt: String = if text.chars().count() > 500 { text.chars().take(500).collect() } else { text.clone() };

    let mut fields = serde_json::Map::new();
    if !text.is_empty() {
        let stops = ["رقم العقد", "النوع", "نطاق العقد", "الأطراف", "العقار", "العميل", "الحالة", "المدة", "تاريخ البداية", "تاريخ النهاية", "العمولة", "رقم الرخصة/الترخيص", "المصدر", "ملف PDF", "مسودة غير موثقة"];
        if let Some(v) = field_after(&text, "رقم العقد", &stops) { fields.insert("contract_number".into(), json!(v)); }
        if let Some(v) = field_after(&text, "النوع", &stops) { fields.insert("contract_type".into(), json!(v)); }
        if let Some(v) = field_after(&text, "نطاق العقد", &stops) { fields.insert("contract_scope".into(), json!(v)); }
        if let Some(v) = field_after(&text, "الأطراف", &stops) { fields.insert("parties".into(), json!([v])); }
        if let Some(v) = field_after(&text, "العقار", &stops) { fields.insert("property_name".into(), json!(v)); }
        if let Some(v) = field_after(&text, "العميل", &stops) { fields.insert("client_name".into(), json!(v)); }
        // علامات إنجليزية صريحة لرقم العقد (مصادر خارجية) — بلا تخمين
        for marker in ["Contract No", "Contract Number", "Agreement No"] {
            if fields.contains_key("contract_number") { break; }
            if let Some(pos) = text.find(marker) {
                let tail = text[pos + marker.len()..].trim_start_matches(|c: char| c == ':' || c.is_whitespace());
                let candidate: String = tail.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '/').collect();
                if candidate.len() >= 3 { fields.insert("contract_number".into(), json!(candidate)); }
            }
        }
    }
    let confirmed_fields = fields.keys().cloned().collect::<Vec<_>>();
    Ok(json!({
        "fields": fields,
        "extracted_text_excerpt": if excerpt.trim().is_empty(){Value::Null}else{json!(excerpt.trim())},
        "confirmed_fields": confirmed_fields
    }))
}

/// تجهيز قيم البوابة من Mapping دلالي موحّد. لا نقرأ الصفحة ولا نكتب فيها:
/// إذا تغيّرت البوابة، يبقى المستخدم صاحب المراجعة والإدخال ولا تُرسل بيانات خاطئة.
pub fn prepare_portal_assist(data: &Value) -> Result<Value, String> {
    if !data.is_object() { return Err("بيانات العقد غير صالحة.".into()); }
    let parties = data.get("parties").and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · "))
        .unwrap_or_default();
    let period = [text(data, "start_date"), text(data, "end_date")]
        .into_iter().filter(|value| !value.is_empty()).collect::<Vec<_>>().join(" ← ");
    let mechanism = match text(data, "commission_mechanism").as_str() {
        "percent" => "نسبة مئوية".to_string(),
        "fixed" => "مبلغ ثابت".to_string(),
        value => value.to_string(),
    };
    let commission = [num(data, "commission_amount").map(|value| value.to_string()).unwrap_or_default(), mechanism]
        .into_iter().filter(|value| !value.is_empty()).collect::<Vec<_>>().join(" ");
    let values = [
        text(data, "contract_type"),
        parties,
        text(data, "property_name"),
        period,
        commission,
        text(data, "contract_scope"),
    ];
    let fields = PORTAL_FIELD_MAPPING.iter().zip(values).filter_map(|((key, label, portal_field), value)| {
        if value.is_empty() { None } else { Some(json!({
            "key": key,
            "label": label,
            "portal_field": portal_field,
            "value": value,
        })) }
    }).collect::<Vec<_>>();
    Ok(json!({
        "provider": "portal_assist",
        "mode": "copy_assist_only",
        "requires_user_login": true,
        "requires_user_confirmation": true,
        "browser_automation": false,
        "credentials_stored": false,
        "captcha_bypass": false,
        "safe_fail_on_portal_change": true,
        "fields": fields,
    }))
}

pub fn save(conn: &Connection, app_data_dir: &Path, user_id: &str, data: &Value) -> Result<Value, String> {
    let id=text(data,"id"); let is_new=id.is_empty(); let id=if is_new {Uuid::new_v4().to_string()} else {id};
    let status=text(data,"status"); let status=if status.is_empty(){"draft".into()}else{status};
    if !["draft","unverified","documented","active","expired","cancelled"].contains(&status.as_str()) { return Err("حالة العقد غير صالحة".into()); }
    let source=text(data,"source"); let source=if source.is_empty(){"manual_draft".into()}else{source};
    let commission_mechanism = text(data, "commission_mechanism");
    if !commission_mechanism.is_empty() && !["percent", "fixed"].contains(&commission_mechanism.as_str()) {
        return Err("آلية العمولة يجب أن تكون نسبة مئوية أو مبلغًا ثابتًا".into());
    }
    let mut pdf_asset_id=text(data,"pdf_asset_id");
    // أسماء العقار/العميل المستخرجة من PDF (بلا معرفات) تُحفظ في provider_fields_json للعرض — بلا تغيير مخطط
    let mut provider_fields = data.get("provider_fields_json").cloned().unwrap_or(json!({}));
    if let Some(obj) = provider_fields.as_object_mut() {
        for key in ["property_name", "client_name"] {
            if let Some(v) = data.get(key).and_then(Value::as_str).filter(|s| !s.trim().is_empty()) {
                obj.insert(key.to_string(), json!(v.trim()));
            }
        }
    }
    if let Some(pdf) = data.get("pdf_data_url").and_then(Value::as_str).filter(|s| !s.is_empty()) {
        let (prefix,b64)=pdf.split_once(',').ok_or("ملف PDF غير صالح")?; if !prefix.contains("application/pdf") { return Err("يُقبل PDF فقط".into()); }
        let bytes=base64::engine::general_purpose::STANDARD.decode(b64).map_err(|_|"تعذر قراءة ملف PDF")?;
        if bytes.len()>20*1024*1024 { return Err("حجم PDF يتجاوز 20 ميغابايت".into()); }
        if !bytes.starts_with(b"%PDF-") { return Err("الملف ليس PDF صالحًا".into()); }
        let name=text(data,"pdf_name"); let name=if name.is_empty(){"contract.pdf".into()}else{name};
        let store=crate::db::files::FileStore::new(app_data_dir)?; let (rel,stored)=store.store_bytes(&name,&bytes,"contracts")?;
        let asset=Uuid::new_v4().to_string(); let checksum=format!("{:x}", md5_like(&bytes));
        conn.execute("INSERT INTO file_assets(id,user_id,entity_type,entity_id,original_name,stored_name,relative_path,mime_type,size_bytes,checksum) VALUES(?1,?2,'contract_pdf',?3,?4,?5,?6,'application/pdf',?7,?8)",params![asset,user_id,id,name,stored,rel,bytes.len() as i64,checksum]).map_err(|e|e.to_string())?; pdf_asset_id=asset;
    }
    let provider_fields_json = serde_json::to_string(&provider_fields).unwrap_or_else(|_| "{}".to_string());
    let values = params![id,user_id,nullable(text(data,"contract_number")),nullable(text(data,"contract_type")),nullable(text(data,"contract_scope")),json_field(data,"parties",json!([])),nullable(text(data,"property_id")),nullable(text(data,"client_id")),nullable(text(data,"campaign_id")),nullable(text(data,"commission_id")),nullable(text(data,"invoice_id")),nullable(text(data,"start_date")),nullable(text(data,"end_date")),status,num(data,"commission_amount"),nullable(commission_mechanism),nullable(text(data,"license_number")),nullable(pdf_asset_id),source,provider_fields_json,nullable(text(data,"extracted_text_excerpt"))];
    if is_new { conn.execute("INSERT INTO contracts(id,user_id,contract_number,contract_type,contract_scope,parties_json,property_id,client_id,campaign_id,commission_id,invoice_id,start_date,end_date,status,commission_amount,commission_mechanism,license_number,pdf_asset_id,source,provider_fields_json,extracted_text_excerpt) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21)",values).map_err(|e|e.to_string())?; } else { conn.execute("UPDATE contracts SET contract_number=?3,contract_type=?4,contract_scope=?5,parties_json=?6,property_id=?7,client_id=?8,campaign_id=?9,commission_id=?10,invoice_id=?11,start_date=?12,end_date=?13,status=?14,commission_amount=?15,commission_mechanism=?16,license_number=?17,pdf_asset_id=COALESCE(?18,pdf_asset_id),source=?19,provider_fields_json=?20,extracted_text_excerpt=?21,updated_at=datetime('now') WHERE id=?1 AND user_id=?2",values).map_err(|e|e.to_string())?; }
    conn.execute("INSERT INTO audit_logs(id,user_id,action,entity_type,entity_id,details) VALUES(?1,?2,?3,'contract',?4,?5)",params![Uuid::new_v4().to_string(),user_id,if is_new{"contract_create"}else{"contract_update"},id,"حفظ عقد يدوي"]).map_err(|e|e.to_string())?;
    Ok(json!({"id":id}))
}

pub fn delete(conn: &Connection, app_data_dir: &Path, user_id: &str, id: &str) -> Result<Value, String> {
    let asset: Option<(String, String)> = conn.query_row(
        "SELECT fa.id, fa.relative_path FROM contracts c JOIN file_assets fa ON fa.id = c.pdf_asset_id WHERE c.id=?1 AND c.user_id=?2",
        params![id, user_id], |r| Ok((r.get(0)?, r.get(1)?))
    ).optional().map_err(|e| e.to_string())?;
    if conn.execute("DELETE FROM contracts WHERE id=?1 AND user_id=?2", params![id, user_id]).map_err(|e| e.to_string())? == 0 { return Err("العقد غير موجود".into()); }
    if let Some((asset_id, relative)) = asset {
        let _ = conn.execute("DELETE FROM file_assets WHERE id=?1 AND user_id=?2", params![asset_id, user_id]);
        if relative.starts_with("contracts/") && !relative.contains("..") && !relative.contains('\\') { let _ = std::fs::remove_file(app_data_dir.join("files").join(relative)); }
    }
    conn.execute("INSERT INTO audit_logs(id,user_id,action,entity_type,entity_id,details) VALUES(?1,?2,'contract_delete','contract',?3,'حذف عقد')", params![Uuid::new_v4().to_string(), user_id, id]).map_err(|e| e.to_string())?;
    Ok(json!({"deleted": true}))
}
fn md5_like(b:&[u8])->u64 { b.iter().fold(0u64,|h,x|h.wrapping_mul(1099511628211)^(*x as u64)) }

/// مسار ملف PDF الأصلي لعقد يملكه المستخدم — مع تقييد صارم داخل مجلد الملفات.
fn contract_pdf_abs(conn: &Connection, app_data_dir: &Path, user_id: &str, id: &str) -> Result<(std::path::PathBuf, String), String> {
    let (rel, name): (Option<String>, Option<String>) = conn.query_row(
        "SELECT fa.relative_path, fa.original_name FROM contracts c LEFT JOIN file_assets fa ON fa.id=c.pdf_asset_id WHERE c.id=?1 AND c.user_id=?2",
        params![id, user_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ).map_err(|e| e.to_string())?;
    let rel = rel.filter(|s| !s.is_empty()).ok_or("لا يوجد PDF مرفق بهذا العقد.".to_string())?;
    if rel.contains("..") || rel.starts_with('/') || rel.starts_with('\\') || rel.contains(':') {
        return Err("مسار ملف غير صالح.".into());
    }
    let store = crate::db::files::FileStore::new(app_data_dir)?;
    let abs = store.absolute_from_relative(&rel);
    // تقييد canonical داخل جذر الملفات حتى لا يُفتح أي ملف خارجي.
    let root_c = store.root().canonicalize().map_err(|e| e.to_string())?;
    let abs_c = abs.canonicalize().map_err(|_| "ملف PDF غير موجود.".to_string())?;
    if !abs_c.starts_with(&root_c) || !abs_c.is_file() {
        return Err("ملف PDF غير موجود.".into());
    }
    Ok((abs_c, name.unwrap_or_else(|| "contract.pdf".into())))
}

/// معاينة PDF داخل التطبيق: تُعيد الملف الأصلي كما هو (قراءة فقط دون أي تعديل) كـ data URL.
pub fn pdf_data(conn: &Connection, app_data_dir: &Path, user_id: &str, id: &str) -> Result<Value, String> {
    let (abs, name) = contract_pdf_abs(conn, app_data_dir, user_id, id)?;
    let bytes = std::fs::read(&abs).map_err(|_| "تعذر قراءة ملف PDF.".to_string())?;
    if bytes.len() > 20 * 1024 * 1024 { return Err("حجم PDF يتجاوز 20 ميغابايت.".into()); }
    if !bytes.starts_with(b"%PDF-") { return Err("الملف ليس PDF صالحًا.".into()); }
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(json!({"pdf_name": name, "data_url": format!("data:application/pdf;base64,{encoded}")}))
}

/// فتح الملف الأصلي في عارض النظام (يتضمن الطباعة من العارض) — دون تعديل الملف.
pub fn open_pdf(conn: &Connection, app_data_dir: &Path, user_id: &str, id: &str) -> Result<Value, String> {
    let (abs, name) = contract_pdf_abs(conn, app_data_dir, user_id, id)?;
    crate::finance_pdf::open_pdf_file(&abs)?;
    Ok(json!({"opened": true, "pdf_name": name}))
}

/// مستند طباعة المسودة بهوية مكتب العميل من إعدادات بيانات المكتب — يُبنى خلفيًا ويُفتح كـ PDF في عارض النظام
/// (طباعة WebView غير موثوقة في Tauri، وهذا المسار هو نفسه المعتمد لفواتير المالية).
pub fn print_html(data: &Value, settings: &Option<Settings>) -> String {
    use crate::document_locale::text as tr;
    let parties = data.get("parties").and_then(Value::as_array).map(|a| {
        a.iter().filter_map(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ")
    }).filter(|s| !s.is_empty()).unwrap_or_default();
    let period = [text(data, "start_date"), text(data, "end_date")].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ← ");
    let commission_mode_raw = text(data, "commission_mechanism");
    let commission_mode = match commission_mode_raw.as_str() {
        "percent" => tr("contract.percent", settings),
        "fixed" => tr("contract.fixed", settings),
        value => value,
    };
    let commission = [num(data, "commission_amount").map(|n| n.to_string()).unwrap_or_default(), if commission_mode_raw == "percent" { "%".to_string() } else { text(data, "currency") }, commission_mode.to_string()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ");
    let status_raw = text(data, "status");
    let status = match status_raw.as_str() {
        "draft" | "unverified" => tr("contract.draft", settings),
        "documented" => tr("contract.documented", settings),
        "active" => tr("contract.active", settings),
        "expired" => tr("contract.expired", settings),
        "cancelled" => tr("contract.cancelled", settings),
        s if s.is_empty() => "",
        s => s,
    };
    let source = if text(data, "source") == "official_pdf" { tr("contract.officialPdf", settings) } else { tr("contract.manualDraft", settings) };
    let rows = [
        (tr("contract.number", settings), display(data, "contract_number")),
        (tr("doc.kind", settings), display(data, "contract_type")),
        (tr("contract.scope", settings), display(data, "contract_scope")),
        (tr("contract.parties", settings), parties),
        (tr("doc.property", settings), display(data, "property_name")),
        (tr("doc.client", settings), display(data, "client_name")),
        (tr("contract.period", settings), period),
        (tr("contract.start", settings), display(data, "start_date")),
        (tr("contract.end", settings), display(data, "end_date")),
        (tr("contract.commission", settings), commission),
        (tr("integration.status", settings), status.into()),
        (tr("contract.license", settings), display(data, "license_number")),
        (tr("contract.source", settings), source.into()),
        (tr("contract.pdf", settings), display(data, "pdf_name")),
    ];
    let rows_html: String = rows.iter().filter(|(_, v)| !v.trim().is_empty()).map(|(k, v)| format!("<tr><th>{}</th><td>{}</td></tr>", esc(k), esc(v))).collect();
    let badge = if matches!(text(data, "status").as_str(), "draft" | "unverified" | "") || text(data, "source") == "manual_draft" {
        format!("<p><strong>{}</strong></p>", tr("contract.draft", settings))
    } else { String::new() };
    let body = format!(
        "<h2>{title}</h2>\n{badge}\n<table>\n{rows}\n</table>",
        title = if display(data, "contract_number").is_empty() { tr("contract.preview", settings).to_string() } else { format!("{} {}", tr("contract.preview", settings), esc(&display(data, "contract_number"))) },
        badge = badge,
        rows = rows_html,
    );
    crate::finance_pdf::shell(&format!("{} {}", tr("contract.contract", settings), display(data, "contract_number")), settings, &body)
}

pub fn summary(conn:&Connection,user_id:&str)->Result<Value,String>{
    let (country, license_type): (String, String) = conn.query_row("SELECT country, COALESCE(fal_license_type,'') FROM settings WHERE user_id=?1", params![user_id], |r| Ok((r.get(0)?, r.get(1)?))).unwrap_or(("SA".into(), String::new()));
    let currency = crate::country::currency(&country)?;
    let official_service = crate::country::service(&country, &license_type)?;
    let manual = ManualProvider;
    let pdf_import = PdfImportProvider;
    let portal_assist = PortalAssistProvider;
    let rega_api = RegaApiProvider;
    conn.query_row("SELECT COUNT(*), SUM(status='active'), SUM(status='expired'), SUM(end_date IS NOT NULL AND end_date >= date('now') AND end_date <= date('now','+30 days')), SUM(status='draft' OR status='unverified'), COALESCE(SUM(commission_amount),0) FROM contracts WHERE user_id=?1 AND country=?2",params![user_id,country],|r|Ok(json!({"country":country,"currency":currency,"official_service":official_service,"total":r.get::<_,i64>(0)?,"active":r.get::<_,Option<i64>>(1)?.unwrap_or(0),"expired":r.get::<_,Option<i64>>(2)?.unwrap_or(0),"ending_soon":r.get::<_,Option<i64>>(3)?.unwrap_or(0),"drafts":r.get::<_,Option<i64>>(4)?.unwrap_or(0),"commission_total":r.get::<_,f64>(5)?,"integration":{"provider":manual.provider_id(),"mode":manual.mode(),"documentation_url":official_service["url"],"providers":[provider_descriptor(&manual),provider_descriptor(&pdf_import),provider_descriptor(&portal_assist),provider_descriptor(&rega_api)],"portal_assist":{"mapping":PORTAL_FIELD_MAPPING.iter().map(|(key,label,portal_field)|json!({"key":key,"label":label,"portal_field":portal_field})).collect::<Vec<_>>(),"browser_automation":false,"safe_fail_on_portal_change":true}}}))).map_err(|e|e.to_string())
}


// ===== استخراج نص PDF الحقيقي (بلا internals ولا تخمين) =====

const PDF_INTERNAL_MARKERS: [&str; 9] = ["PDF-1.", "/Title", "/Creator", "/Producer", "/CreationDate", "FlateDecode", "endstream", "stream", "/Filter"];

/// يستخرج النص القابل للقراءة من PDF: يفك تدفقات FlateDecode ويحل النصوص عبر ToUnicode إن وُجدت.
fn pdf_readable_text(bytes: &[u8]) -> Option<String> {
    use flate2::read::ZlibDecoder;
    use std::io::Read;

    // المسح يجري على bytes مباشرةً: String المفقود (lossy) يُغيّر مواضع البايتات
    // عند أول محتوى ثنائي، وهو ما كان يُبطل فك التدفقات بعد أول stream ثنائي.
    fn find_from(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
        if needle.is_empty() || from >= hay.len() { return None; }
        hay[from..].windows(needle.len()).position(|w| w == needle).map(|p| p + from)
    }
    fn rfind_before(hay: &[u8], needle: &[u8], before: usize) -> Option<usize> {
        if needle.is_empty() || before == 0 { return None; }
        let end = before.min(hay.len());
        hay[..end].windows(needle.len()).rposition(|w| w == needle)
    }

    let b: &[u8] = bytes;

    // 1) فك كل تدفقات FlateDecode
    let mut streams: Vec<String> = Vec::new();
    let mut cursor = 0usize;
    while let Some(abs) = find_from(b, b"stream", cursor) {
        if abs >= 3 && &b[abs - 3..abs] == b"end" { cursor = abs + b"stream".len(); continue; }
        let mut data_start = abs + b"stream".len();
        while data_start < b.len() && (b[data_start] == b'\r' || b[data_start] == b'\n') { data_start += 1; }
        let Some(data_end) = find_from(b, b"endstream", data_start) else { break };
        if data_start < data_end {
            let dict_start = rfind_before(b, b"<<", abs).unwrap_or(abs.saturating_sub(400));
            let dict = &b[dict_start..abs];
            if find_from(dict, b"FlateDecode", 0).is_some() {
                let mut dec = ZlibDecoder::new(&b[data_start..data_end]);
                let mut out = Vec::new();
                if dec.read_to_end(&mut out).is_ok() {
                    streams.push(String::from_utf8_lossy(&out).into_owned());
                }
            }
        }
        cursor = data_end + b"endstream".len();
        if cursor >= b.len() { break; }
    }
    if streams.is_empty() { return None; }

    // 2) خرائط ToUnicode (bfchar/bfrange) لتحل رموز الخط إلى Unicode
    let mut cmap: std::collections::HashMap<u32, Vec<char>> = std::collections::HashMap::new();
    let mut code_bytes = 2usize;
    for s in &streams {
        if !s.contains("beginbfchar") && !s.contains("beginbfrange") { continue; }
        for section in s.split("beginbfchar").skip(1) {
            let body = match section.find("endbfchar") { Some(e) => &section[..e], None => continue };
            let toks = tokenize_hex_pairs(body);
            let mut k = 0;
            while k + 1 < toks.len() {
                code_bytes = code_bytes.max(toks[k].len() / 2);
                if let (Some(code), Some(chars)) = (hex_code(&toks[k]), utf16be_chars(&toks[k + 1])) {
                    cmap.insert(code, chars);
                }
                k += 2;
            }
        }
        for section in s.split("beginbfrange").skip(1) {
            let body = match section.find("endbfrange") { Some(e) => &section[..e], None => continue };
            let toks = tokenize_hex_pairs(body);
            let mut k = 0;
            while k + 2 < toks.len() {
                code_bytes = code_bytes.max(toks[k].len() / 2);
                if let (Some(lo_c), Some(hi_c)) = (hex_code(&toks[k]), hex_code(&toks[k + 1])) {
                    if hi_c > lo_c && hi_c - lo_c < 65536 {
                        if let Some(base) = utf16be_chars(&toks[k + 2]) {
                            if let Some(&first) = base.first() {
                                for off in 0..=(hi_c - lo_c) {
                                    cmap.insert(lo_c + off, vec![char::from_u32(first as u32 + off).unwrap_or('\u{fffd}')]);
                                }
                            }
                        }
                    }
                }
                k += 3;
            }
        }
    }

    // 3) نصوص المحتوى: تُقرأ من كتل النص (BT…ET) فقط وتُحل عبر /ActualText ثم ToUnicode
    let mut lines: Vec<String> = Vec::new();
    for s in &streams {
        if s.contains("begincmap") || s.contains("beginbfchar") || s.contains("beginbfrange") { continue; }
        lines.extend(extract_text_blocks(s, &cmap, code_bytes));
    }
    let cleaned: Vec<String> = lines.into_iter()
        .filter(|t| !t.is_empty() && t.chars().count() <= 300 && t.chars().any(|c| c.is_alphanumeric()))
        .collect();
    if cleaned.is_empty() { return None; }
    let joined = cleaned.join("\n");
    if joined.chars().any(|c| !c.is_ascii() && c.is_alphabetic())
        || joined.chars().filter(|c| c.is_alphanumeric()).count() >= 4
    {
        Some(joined)
    } else {
        None
    }
}

/// استخراج أزواج <hex> <hex> من جسم خريطة ToUnicode
fn tokenize_hex_pairs(body: &str) -> Vec<String> {
    let b = body.as_bytes();
    let mut toks = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'<' => {
                let mut j = i + 1;
                let mut hex = String::new();
                while j < b.len() { let c = b[j] as char; if c == '>' { break; } if c.is_ascii_hexdigit() { hex.push(c); } j += 1; }
                if !hex.is_empty() { toks.push(hex); }
                i = j + 1;
            }
            _ => i += 1,
        }
    }
    toks
}

fn hex_code(hex: &str) -> Option<u32> { u32::from_str_radix(hex, 16).ok() }

/// dst هو UTF-16BE
fn utf16be_chars(hex: &str) -> Option<Vec<char>> {
    let bytes: Vec<u8> = (0..hex.len() / 2).filter_map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()).collect();
    if bytes.is_empty() { return None; }
    let units: Vec<u16> = bytes.chunks(2).filter(|c| c.len() == 2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
    let s: String = String::from_utf16(&units).ok()?;
    if s.is_empty() { None } else { Some(s.chars().collect()) }
}

/// عنصر داخل مصفوفة TJ: نص أو إزاحة.
enum TjItem { Text(String), Gap(f64) }

/// نطاقات صور الحروف العربية (Arabic Presentation Forms-B) → الحرف الأساسي المقابل.
/// الخطوط المُجزّأة (subset) تُصدّر رموز الحروف في هذه الصور، فتُعاد إلى حروفها الأساسية للعرض والقراءة فقط.
const ARABIC_FORM_RANGES: [(u32, u32, char); 45] = [
    (0xFE70, 0xFE70, '\u{064B}'), (0xFE72, 0xFE72, '\u{064C}'), (0xFE74, 0xFE74, '\u{064D}'),
    (0xFE76, 0xFE77, '\u{064E}'), (0xFE78, 0xFE79, '\u{064F}'), (0xFE7A, 0xFE7B, '\u{0650}'),
    (0xFE7C, 0xFE7D, '\u{0651}'), (0xFE7E, 0xFE7F, '\u{0652}'),
    (0xFE80, 0xFE80, '\u{0621}'), (0xFE81, 0xFE82, '\u{0622}'), (0xFE83, 0xFE84, '\u{0623}'),
    (0xFE85, 0xFE86, '\u{0624}'), (0xFE87, 0xFE88, '\u{0625}'), (0xFE89, 0xFE8C, '\u{0626}'),
    (0xFE8D, 0xFE8E, '\u{0627}'), (0xFE8F, 0xFE92, '\u{0628}'), (0xFE93, 0xFE94, '\u{0629}'),
    (0xFE95, 0xFE98, '\u{062A}'), (0xFE99, 0xFE9C, '\u{062B}'), (0xFE9D, 0xFEA0, '\u{062C}'),
    (0xFEA1, 0xFEA4, '\u{062D}'), (0xFEA5, 0xFEA8, '\u{062E}'), (0xFEA9, 0xFEAA, '\u{062F}'),
    (0xFEAB, 0xFEAC, '\u{0630}'), (0xFEAD, 0xFEAE, '\u{0631}'), (0xFEAF, 0xFEB0, '\u{0632}'),
    (0xFEB1, 0xFEB4, '\u{0633}'), (0xFEB5, 0xFEB8, '\u{0634}'), (0xFEB9, 0xFEBC, '\u{0635}'),
    (0xFEBD, 0xFEC0, '\u{0636}'), (0xFEC1, 0xFEC4, '\u{0637}'), (0xFEC5, 0xFEC8, '\u{0638}'),
    (0xFEC9, 0xFECC, '\u{0639}'), (0xFECD, 0xFED0, '\u{063A}'), (0xFED1, 0xFED4, '\u{0641}'),
    (0xFED5, 0xFED8, '\u{0642}'), (0xFED9, 0xFEDC, '\u{0643}'), (0xFEDD, 0xFEE0, '\u{0644}'),
    (0xFEE1, 0xFEE4, '\u{0645}'), (0xFEE5, 0xFEE8, '\u{0646}'), (0xFEE9, 0xFEEC, '\u{0647}'),
    (0xFEED, 0xFEEE, '\u{0648}'), (0xFEEF, 0xFEF0, '\u{0649}'), (0xFEF1, 0xFEF4, '\u{064A}'),
    (0xFEF5, 0xFEFC, '\u{0000}'), // تُعالج اللام-ألف في arabic_ligature_visual
];

/// اللام-ألف المركّبة (ligature) تُوسّع بزوج بترتيب بصري، ليصح ترتيبه بعد إعادة ترتيب السطر.
fn arabic_ligature_visual(c: char) -> Option<[char; 2]> {
    match c {
        '\u{FEF5}' | '\u{FEF6}' => Some(['\u{0622}', '\u{0644}']),
        '\u{FEF7}' | '\u{FEF8}' => Some(['\u{0623}', '\u{0644}']),
        '\u{FEF9}' | '\u{FEFA}' => Some(['\u{0625}', '\u{0644}']),
        '\u{FEFB}' | '\u{FEFC}' => Some(['\u{0627}', '\u{0644}']),
        _ => None,
    }
}

/// تحويل صورة الحرف إلى حرفه الأساسي — طبقة قراءة فقط، بلا أي تغيير في الملف أو البيانات.
fn push_base_arabic(c: char, out: &mut Vec<char>) {
    if let Some(pair) = arabic_ligature_visual(c) { out.extend(pair); return; }
    let cp = c as u32;
    for (lo, hi, base) in ARABIC_FORM_RANGES.iter() {
        if cp >= *lo && cp <= *hi && *base != '\u{0000}' { out.push(*base); return; }
    }
    out.push(c);
}

fn is_rtl_char(c: char) -> bool {
    matches!(c as u32,
        0x0590..=0x05FF | 0x0600..=0x06FF | 0x0700..=0x074F | 0x0750..=0x077F
        | 0x0780..=0x07BF | 0x08A0..=0x08FF | 0xFB1D..=0xFB4F | 0xFB50..=0xFDFF
        | 0xFE70..=0xFEFF)
}

/// إعادة ترتيب سطر عربي مكتوب بالترتيب البصري (كما يُرسم في PDF) إلى ترتيب منطقي مقروء.
fn visual_line_to_logical(s: &str) -> String {
    if !s.chars().any(is_rtl_char) { return s.to_string(); }
    let rev: Vec<char> = s.chars().rev().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0usize;
    while i < rev.len() {
        if !is_rtl_char(rev[i]) && !rev[i].is_whitespace() {
            let start = i;
            while i < rev.len() && !is_rtl_char(rev[i]) && !rev[i].is_whitespace() { i += 1; }
            out.extend(rev[start..i].iter().rev());
        } else {
            out.push(rev[i]);
            i += 1;
        }
    }
    out
}

/// نص /ActualText (UTF-16BE مع BOM) — أدق مصدر متاح للـUnicode داخل PDF.
fn utf16be_text(hex: &str) -> Option<String> {
    let bytes: Vec<u8> = (0..hex.len() / 2).filter_map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()).collect();
    if bytes.len() < 2 { return None; }
    let body: &[u8] = if bytes[0] == 0xFE && bytes[1] == 0xFF { &bytes[2..] } else { &bytes[..] };
    let units: Vec<u16> = body.chunks(2).filter(|c| c.len() == 2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
    let s: String = String::from_utf16(&units).ok()?;
    if s.trim().is_empty() { None } else { Some(s) }
}

/// ترشيح كتلة نص PDF: إعادة صور الحروف لعناصرها + ترتيب منطقي للعربية + رفض كل ما ليس نصًا حقيقيًا.
/// لا تُعرض bytes ولا Metadata ولا أي PDF internals للمستخدم.
fn finalize_text_block(chars: &[char]) -> Option<String> {
    if chars.iter().all(|c| c.is_whitespace()) { return None; }
    let mut norm: Vec<char> = Vec::with_capacity(chars.len());
    for c in chars { push_base_arabic(*c, &mut norm); }
    let raw: String = norm.into_iter().collect();
    let trimmed = raw.trim();
    if trimmed.is_empty() { return None; }
    if PDF_INTERNAL_MARKERS.iter().any(|m| trimmed.contains(m)) { return None; }
    let text = if trimmed.chars().any(is_rtl_char) { visual_line_to_logical(trimmed) } else { trimmed.to_string() };
    let clean = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if clean.is_empty() || !clean.chars().any(|c| c.is_alphanumeric()) { None } else { Some(clean) }
}

fn push_tj_items(items: Vec<TjItem>, x: f64, y: f64, sep: &mut bool, pending: &mut Vec<(String, f64, f64, bool)>) {
    for item in items {
        match item {
            TjItem::Text(t) => { if !t.is_empty() { pending.push((t, x, y, *sep)); *sep = false; } }
            TjItem::Gap(g) => { if g <= -150.0 { *sep = true; } }
        }
    }
}

/// تجميع كتلة نص واحدة (BT…ET): تُجمَّع القطع المتجاورة بنفس x في عملية رسم واحدة، ثم يُقدَّر
/// تباعد الحروف من هندسة الكتلة نفسها: لا مسافة داخل الكلمة، ومسافة عند حدود الكلمة الحقيقية فقط.
fn assemble_block(items: &[(String, f64, f64, bool)], font_size: f64) -> Option<(String, f64, f64, f64)> {
    let (x, y) = items.first().map(|(_, x, y, _)| (*x, *y))?;
    let size = if font_size > 0.0 { font_size } else { 12.0 };
    // عملية الرسم لكل قطعة بالترتيب: قطعًا متجاورة بنفس x = عملية واحدة.
    let mut op_of: Vec<usize> = Vec::with_capacity(items.len());
    let mut op_x: Vec<f64> = Vec::new();
    let mut op_n: Vec<usize> = Vec::new();
    for (text, px, _, _) in items {
        let n = text.chars().count();
        match op_x.last() {
            Some(last_x) if (*last_x - px).abs() < 0.05 => {
                *op_n.last_mut().unwrap() += n;
                op_of.push(op_x.len() - 1);
            }
            _ => {
                op_x.push(*px);
                op_n.push(n);
                op_of.push(op_x.len() - 1);
            }
        }
    }
    let advance = if op_x.len() >= 2 {
        let before: usize = op_n[..op_n.len() - 1].iter().sum();
        let span = (op_x[op_x.len() - 1] - op_x[0]).abs();
        if before > 0 && span > 0.0 { span / before as f64 } else { 0.5 * size }
    } else {
        0.5 * size
    };

    let mut chars: Vec<char> = Vec::new();
    for (idx, (text, px, _, gap_space)) in items.iter().enumerate() {
        let this_op = op_of[idx];
        if idx > 0 && this_op != op_of[idx - 1] {
            let prev = op_of[idx - 1];
            // نهاية العملية السابقة = بدايتها + حروفها × التباعد المحسوب من الكتلة نفسها.
            let gap = (px - (op_x[prev] + op_n[prev] as f64 * advance)).abs();
            if gap > 0.35 * advance && !chars.is_empty() && chars.last() != Some(&' ') {
                chars.push(' ');
            }
        } else if idx > 0 && *gap_space && !chars.is_empty() && chars.last() != Some(&' ') {
            // فجوة رسم كبيرة داخل العملية نفسها = مسافة كلمة صريحة.
            chars.push(' ');
        }
        chars.extend(text.chars());
    }
    finalize_text_block(&chars).map(|t| (t, x, y, size))
}

/// تقدير عرض قطعة نصية (نصف em لكل حرف) — يُستخدم فقط لقياس الفجوات بين القطع عند القراءة.
fn estimate_width(text: &str, font_size: f64) -> f64 {
    text.chars().count() as f64 * 0.5 * font_size
}

/// دمج قطع السطر بترتيبها البصري، مع مسافة عند وجود فجوة رسم حقيقية فقط:
/// Chromium يجزّئ القطعة داخل الكلمة الواحدة، فإدراج مسافة عمياء كان يفصل الكلمات العربية.
fn join_visual_line(items: &[(f64, String, f64)]) -> String {
    let has_rtl = items.iter().any(|(_, t, _)| t.chars().any(is_rtl_char));
    let mut ordered: Vec<&(f64, String, f64)> = items.iter().collect();
    ordered.sort_by(|a, b| if has_rtl { b.0.partial_cmp(&a.0) } else { a.0.partial_cmp(&b.0) }.unwrap_or(std::cmp::Ordering::Equal));
    let mut out = String::new();
    let mut prev: Option<&(f64, String, f64)> = None;
    for cur in ordered {
        if let Some(p) = prev {
            let gap = if has_rtl { p.0 - cur.0 - estimate_width(&cur.1, cur.2) } else { cur.0 - (p.0 + estimate_width(&p.1, p.2)) };
            if gap > 0.5 * p.2 { out.push(' '); }
        }
        out.push_str(&cur.1);
        prev = Some(cur);
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// استخراج أسطر النص من تدفق محتوى PDF: كتل BT…ET فقط، مع /ActualText ثم ToUnicode.
/// الأسطر تُجمّع حسب موضع الرسم (y) لا حسب ترتيب التدفق، فتُفصل أسطر الجدول وتبقى الكلمات منفصلة.
fn extract_text_blocks(content: &str, cmap: &std::collections::HashMap<u32, Vec<char>>, code_bytes: usize) -> Vec<String> {
    let b = content.as_bytes();
    // كتل النص النهائية: (نص، x، y، حجم الخط) — كل كتلة BT…ET هي قطعة رسم مستقلة في PDF.
    let mut blocks: Vec<(String, f64, f64, f64)> = Vec::new();
    // قطع الكتلة الجارية (نص، x، y، فاصل مطلوب قبله)
    let mut pending: Vec<(String, f64, f64, bool)> = Vec::new();
    let mut nums: Vec<f64> = Vec::new();
    let mut line_x = 0.0f64;
    let mut line_y = 0.0f64;
    let mut leading = 0.0f64;
    let mut font_size = 0.0f64;
    let mut sep = false;
    let mut actual: Option<String> = None;
    let mut expect_actual = false;
    let mut array: Option<Vec<TjItem>> = None;
    let mut closed_array: Option<Vec<TjItem>> = None;
    let mut i = 0usize;

    while i < b.len() {
        match b[i] {
            b'%' => { while i < b.len() && b[i] != b'\n' { i += 1; } }
            b'(' => {
                let mut j = i + 1;
                let mut raw = String::new();
                let mut depth = 1;
                while j < b.len() && depth > 0 {
                    let c = b[j] as char;
                    if c == '\\' && j + 1 < b.len() { raw.push(c); raw.push(b[j + 1] as char); j += 2; continue; }
                    if c == '(' { depth += 1; }
                    if c == ')' { depth -= 1; if depth == 0 { break; } }
                    raw.push(c);
                    j += 1;
                }
                let text = match &actual { Some(a) => a.clone(), None => decode_literal(&raw).unwrap_or_default() };
                if let Some(arr) = array.as_mut() { if !text.is_empty() { arr.push(TjItem::Text(text)); } }
                else if !text.is_empty() { pending.push((text, line_x, line_y, sep)); sep = false; }
                nums.clear();
                i = j + 1;
            }
            b'<' if b.get(i + 1) != Some(&b'<') => {
                let mut j = i + 1;
                let mut hex = String::new();
                while j < b.len() { let c = b[j] as char; if c == '>' { break; } if c.is_ascii_hexdigit() { hex.push(c); } j += 1; }
                if expect_actual {
                    expect_actual = false;
                    actual = utf16be_text(&hex);
                } else {
                    let text = match &actual { Some(a) => a.clone(), None => decode_hex(&hex, cmap, code_bytes).unwrap_or_default() };
                    if !text.is_empty() {
                        if let Some(arr) = array.as_mut() { arr.push(TjItem::Text(text)); }
                        else { pending.push((text, line_x, line_y, sep)); sep = false; }
                    }
                }
                nums.clear();
                i = j + 1;
            }
            b'<' => { if let Some(items) = closed_array.take() { push_tj_items(items, line_x, line_y, &mut sep, &mut pending); } nums.clear(); i += 2; }
            b'[' => { if let Some(items) = closed_array.take() { push_tj_items(items, line_x, line_y, &mut sep, &mut pending); } array = Some(Vec::new()); nums.clear(); i += 1; }
            b']' => { closed_array = array.take(); i += 1; }
            c if c.is_ascii_whitespace() || matches!(c, b'>' | b')' | b'{' | b'}') => { i += 1; }
            _ => {
                let start = i;
                while i < b.len() {
                    let c = b[i];
                    if c.is_ascii_whitespace() || matches!(c, b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%') { break; }
                    i += 1;
                }
                if start == i { i += 1; continue; }
                let token = String::from_utf8_lossy(&b[start..i]).into_owned();
                match token.as_str() {
                    "BT" => { pending.clear(); sep = false; line_x = 0.0; line_y = 0.0; }
                    "ET" => {
                        if let Some(block) = assemble_block(&pending, font_size) { blocks.push(block); }
                        pending.clear(); sep = false;
                    }
                    "Tf" => { if let Some(v) = nums.last() { font_size = *v; } nums.clear(); }
                    "TJ" => { if let Some(items) = closed_array.take() { push_tj_items(items, line_x, line_y, &mut sep, &mut pending); } nums.clear(); }
                    "Tm" => { if nums.len() >= 6 { line_x = nums[4]; line_y = nums[5]; } sep = true; nums.clear(); }
                    "Td" | "TD" => { if nums.len() >= 2 { line_x += nums[0]; line_y += nums[1]; } sep = true; nums.clear(); }
                    "T*" => { line_y -= leading; sep = true; nums.clear(); }
                    "TL" => { if let Some(v) = nums.first() { leading = *v; } nums.clear(); }
                    "ActualText" => { expect_actual = true; nums.clear(); }
                    "EMC" => { actual = None; nums.clear(); }
                    _ => {
                        if let Some(items) = closed_array.take() { push_tj_items(items, line_x, line_y, &mut sep, &mut pending); }
                        if let Ok(v) = token.parse::<f64>() {
                            nums.push(v);
                            if let Some(arr) = array.as_mut() { arr.push(TjItem::Gap(v)); }
                            continue; // معاملات الأوامر تبقى حتى يصل الأمر الذي يستهلكها
                        }
                        nums.clear();
                    }
                }
            }
        }
    }
    if let Some(items) = closed_array.take() { push_tj_items(items, line_x, line_y, &mut sep, &mut pending); }
    if let Some(block) = assemble_block(&pending, font_size) { blocks.push(block); }

    let mut lines: Vec<String> = Vec::new();
    let mut groups: Vec<(f64, Vec<(f64, String, f64)>)> = Vec::new();
    for (text, x, y, size) in blocks {
        match groups.iter_mut().find(|(gy, _)| (y - *gy).abs() <= 2.0) {
            Some((_, items)) => items.push((x, text, size)),
            None => groups.push((y, vec![(x, text, size)])),
        }
    }
    for (_, items) in groups.iter_mut() {
        let line = join_visual_line(items);
        if !line.is_empty() { lines.push(line); }
    }
    lines
}



fn decode_literal(raw: &str) -> Option<String> {
    let mut out = String::new();
    let chars: Vec<char> = raw.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            match chars[i + 1] {
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                'b' | 'f' => {},
                c if c.is_ascii_digit() => {
                    let mut digits = String::new();
                    let mut k = i + 1;
                    while k < chars.len() && digits.len() < 3 && chars[k].is_ascii_digit() { digits.push(chars[k]); k += 1; }
                    if let Ok(v) = u8::from_str_radix(&digits, 8) { out.push(v as char); }
                    i = k - 1;
                }
                c => out.push(c),
            }
            i += 2;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    let t = out.trim().to_string();
    if t.is_empty() { None }
    else if t.chars().all(|c| (' '..='~').contains(&c) || !c.is_ascii()) { Some(t) }
    else { None }
}

fn decode_hex(hex: &str, cmap: &std::collections::HashMap<u32, Vec<char>>, code_bytes: usize) -> Option<String> {
    let bytes: Vec<u8> = (0..hex.len() / 2).filter_map(|k| u8::from_str_radix(&hex[k * 2..k * 2 + 2], 16).ok()).collect();
    if bytes.is_empty() { return None; }
    if cmap.is_empty() {
        if bytes.iter().all(|b| (0x20..=0x7e).contains(b)) {
            let s: String = bytes.iter().map(|b| *b as char).collect();
            let t = s.trim().to_string();
            return if t.is_empty() { None } else { Some(t) };
        }
        return None;
    }
    let width = code_bytes.max(1);
    let mut out = String::new();
    let mut mapped = 0usize;
    let mut total = 0usize;
    for chunk in bytes.chunks(width) {
        if chunk.len() != width { break; }
        let mut code: u32 = 0;
        for c in chunk { code = (code << 8) | *c as u32; }
        total += 1;
        if let Some(chars) = cmap.get(&code) {
            out.extend(chars.iter());
            mapped += 1;
        }
    }
    if total == 0 || (mapped as f64) / (total as f64) < 0.7 { return None; }
    let t = out.trim().to_string();
    if t.is_empty() { None } else { Some(t) }
}

/// إزالة كل الفراغات — للمطابقة فقط (Chromium يفصل بعض الكلمات العربية بفراغ زائد داخل PDF).
fn strip_ws(s: &str) -> String { s.chars().filter(|c| !c.is_whitespace()).collect() }

/// بحث لا يتأثر بالفراغات: يعيد (بداية المطابقة، ما بعد نهايتها) بمواضع النص الأصلي.
fn flat_find(text: &str, needle_flat: &str) -> Option<(usize, usize)> {
    if needle_flat.is_empty() { return None; }
    let mut flat = String::new();
    let mut map: Vec<usize> = Vec::new();
    for (i, c) in text.char_indices() {
        if c.is_whitespace() { continue; }
        flat.push(c);
        map.push(i);
    }
    let hit = flat.find(needle_flat)?;
    let char_start = flat[..hit].chars().count();
    let start = map[char_start];
    let end_char = char_start + needle_flat.chars().count();
    let end = if end_char < map.len() { map[end_char] } else { text.len() };
    Some((start, end))
}

/// قراءة قيمة بعد تسمية عربية معروفة، وتتوقف عند أي تسمية تالية — بلا تخمين.
/// المطابقة تتجاهل الفراغات فقط (لا تتجاهل أي حرف)، والقيمة تُعاد كما استُخرجت من PDF.
fn field_after(text: &str, label: &str, stops: &[&str]) -> Option<String> {
    let (_, label_end) = flat_find(text, &strip_ws(label))?;
    let rest = &text[label_end..];
    let mut bound: Option<usize> = None;
    for stop in stops {
        if *stop == label { continue; }
        let needle = strip_ws(stop);
        if needle.is_empty() { continue; }
        if let Some((p, _)) = flat_find(rest, &needle) { if bound.map(|b| p < b).unwrap_or(true) { bound = Some(p); } }
    }
    let value = match bound { Some(p) => &rest[..p], None => rest };
    let v = value.trim().trim_matches(|c: char| c == ':' || c == '—' || c == '-').trim().to_string();
    let clean: String = v.split_whitespace().collect::<Vec<_>>().join(" ");
    if clean.is_empty() || clean.chars().count() > 120 { return None; }
    if clean.chars().filter(|c| c.is_alphanumeric()).count() < 1 { return None; }
    if stops.iter().any(|s| clean == *s) { return None; }
    Some(clean)
}
