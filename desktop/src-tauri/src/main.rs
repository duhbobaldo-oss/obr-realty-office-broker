// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod campaigns;
mod db;
mod finance;
mod finance_pdf;
mod finance_service;
mod contracts;
mod capacity;
mod country;
mod document_locale;
mod integration_secrets;
mod official_integrations;
mod services;
mod map_tiles;
mod map_cache;
mod share_pack;
mod tracking_client;
mod whatsapp;
mod license;

#[cfg(test)]
mod tests;

use db::{AppState, SqliteDatabase};
use finance_service::FinanceService;
use services::*;
use std::sync::Mutex;
use tauri::{Manager, State};

// ===== حالة التطبيق =====

struct AppData {
    db: Mutex<AppState>,
    db_path: std::path::PathBuf,
    app_data_dir: std::path::PathBuf,
}

// ===== أدوات مساعدة =====

// تنفيذ عملية على قاعدة البيانات مع الاحتفاظ بالقفل طوال العملية
fn with_db<T>(
    app: &AppData,
    f: impl FnOnce(&dyn db::Database) -> Result<T, String>,
) -> Result<T, String> {
    let state = app.db.lock().map_err(|e| e.to_string())?;
    let guard = state.db.lock().map_err(|e| e.to_string())?;
    f(guard.as_ref())
}

fn get_user_id(app: &AppData, token: &str) -> Result<String, String> {
    // جلسة محلية فقط — لا بوابة ترخيص في هذه النسخة
    with_db(app, |db| validate_session_token(db, token))
}

pub(crate) fn validate_open_url(url: &str) -> Result<(), String> {
    let trimmed = url.trim();
    if trimmed.is_empty() || trimmed.chars().any(|c| c.is_control()) {
        return Err("رابط غير صالح".into());
    }
    if trimmed.starts_with("mailto:") || trimmed.starts_with("https://") {
        return Ok(());
    }
    // خدمة التتبع المحلية قد تعمل على http://127.0.0.1 أثناء التطوير
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("http://127.0.0.1:")
        || lower.starts_with("http://127.0.0.1/")
        || lower.starts_with("http://localhost:")
        || lower.starts_with("http://localhost/")
    {
        return Ok(());
    }
    if trimmed.starts_with("whatsapp://send?") {
        return crate::whatsapp::validate_whatsapp_url(trimmed);
    }
    Err("رابط غير مسموح".into())
}

#[tauri::command]
fn open_external_url(url: String) -> Result<(), String> {
    validate_open_url(&url)?;
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("تعذر فتح البرنامج: {e}"))?;
        return Ok(());
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("فتح الروابط الخارجية غير متاح على هذا النظام".into())
    }
}

#[tauri::command]
fn open_whatsapp_chat(phone: String, text: String) -> Result<(), String> {
    crate::whatsapp::open_whatsapp_chat(phone, text)
}

#[tauri::command]
fn prepare_offer_share_pack(
    app: State<AppData>,
    token: String,
    property_id: String,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    let app_data_dir = app.app_data_dir.clone();
    with_db(&app, |db| {
        let pack = crate::share_pack::prepare_offer_share_pack(db, &user_id, &property_id, &app_data_dir)?;
        serde_json::to_value(pack).map_err(|e| e.to_string())
    })
}

#[tauri::command]
fn copy_share_files_to_clipboard(
    app: State<AppData>,
    token: String,
    paths: Vec<String>,
) -> Result<(), String> {
    let _user_id = get_user_id(&app, &token)?;
    let mut checked = Vec::with_capacity(paths.len());
    for path in &paths {
        let resolved = crate::share_pack::resolve_share_pack_file(&app.app_data_dir, path)?;
        checked.push(resolved.to_string_lossy().into_owned());
    }
    crate::share_pack::copy_files_to_clipboard(&checked)
}

#[tauri::command]
fn open_share_folder(app: State<AppData>, token: String, path: String) -> Result<(), String> {
    let _user_id = get_user_id(&app, &token)?;
    let resolved = crate::share_pack::resolve_share_pack_dir(&app.app_data_dir, &path)?;
    crate::share_pack::open_folder(&resolved.to_string_lossy())
}

/// تصدير تصميم العرض: pdf | png | doc | html إلى مسار يختاره المستخدم.
#[tauri::command]
fn export_design_document(
    app: State<AppData>,
    token: String,
    html: String,
    format: String,
    dest_path: String,
    width_px: Option<u32>,
    height_px: Option<u32>,
) -> Result<serde_json::Value, String> {
    let _user_id = get_user_id(&app, &token)?;
    let path = crate::share_pack::resolve_export_destination(&app.app_data_dir, &dest_path)?;
    let w = width_px.unwrap_or(794);
    let h = height_px.unwrap_or(1123);
    crate::share_pack::export_html_document(&html, format.trim(), &path, w, h)?;
    Ok(serde_json::json!({
        "ok": true,
        "format": format,
        "path": path.to_string_lossy(),
    }))
}

#[tauri::command]
fn map_assets_status() -> Result<crate::map_tiles::MapAssetsReport, String> {
    Ok(crate::map_tiles::assets_report())
}

#[tauri::command]
fn map_read_range(asset_id: String, offset: u64, length: u64) -> Result<Vec<u8>, String> {
    crate::map_tiles::read_range(&asset_id, offset, length)
}

// ===== Commands: كاش الخريطة (Online-first + Offline Cache) =====

/// جلب بلاطة: كاش محلي أولاً → تنزيل وتخزين محلي → خطأ صديق عند القطع
#[tauri::command]
fn map_cache_fetch(url: String) -> Result<Vec<u8>, String> {
    crate::map_cache::fetch_tile(&url)
}

/// حالة الكاش: المسار، عدد البلاطات، الحجم، الحد
#[tauri::command]
fn map_cache_status() -> Result<serde_json::Value, String> {
    let s = crate::map_cache::status()?;
    Ok(serde_json::json!({
        "root": s.root,
        "entries": s.entries,
        "bytes": s.bytes,
        "limit_bytes": s.limit_bytes,
    }))
}

/// تفريغ كاش البلاطات (صيانة)
#[tauri::command]
fn map_cache_clear() -> Result<usize, String> {
    crate::map_cache::clear_all()
}

// ===== Commands: المصادقة =====

#[tauri::command]
fn auth_logout(app: State<AppData>, token: String) -> Result<serde_json::Value, String> {
    with_db(&app, |db| {
        let service = AuthService::new(db);
        service.logout(&token)
    })?;
    Ok(serde_json::json!({ "message": "تم تسجيل الخروج" }))
}

#[tauri::command]
fn auth_me(app: State<AppData>, token: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    let result = with_db(&app, |db| {
        let service = AuthService::new(db);
        service.me(&user_id)
    })?;
    Ok(result)
}

#[tauri::command]
fn auth_ensure_local(app: State<AppData>) -> Result<serde_json::Value, String> {
    // جلسة محلية مباشرة — لا تفعيل ولا حساب OBR في هذه النسخة
    let session = with_db(&app, |db| {
        let service = AuthService::new(db);
        service.ensure_local_session("مستخدم OBR", "local@obr.invalid", None)
    })?;
    let user_id = session["user"]["id"]
        .as_str()
        .ok_or("معرّف المستخدم غير متوفر")?
        .to_string();
    let mut conn = open_migrated_conn(&app.db_path)?;
    db::demo::ensure_official_training_seed(&mut conn, &user_id, &app.app_data_dir)?;
    Ok(session)
}

// ===== Commands: العروض =====

#[tauri::command]
fn properties_list(
    app: State<AppData>,
    token: String,
    page: Option<i64>,
    limit: Option<i64>,
    search: Option<String>,
    status: Option<String>,
    city: Option<String>,
    property_type: Option<String>,
    purpose: Option<String>,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = PropertyService::new(db);
        service.list(
            &user_id,
            page.unwrap_or(1),
            limit.unwrap_or(20),
            &search.unwrap_or_default(),
            &status.unwrap_or_default(),
            &city.unwrap_or_default(),
            &property_type.unwrap_or_default(),
            &purpose.unwrap_or_default(),
        )
    })
}

#[tauri::command]
fn properties_get(app: State<AppData>, token: String, id: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = PropertyService::new(db);
        service.get(&id, &user_id)
    })
}

#[tauri::command]
fn properties_create(app: State<AppData>, token: String, data: serde_json::Value) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = PropertyService::new(db);
        service.create(&user_id, &data)
    })
}

#[tauri::command]
fn properties_update(app: State<AppData>, token: String, id: String, data: serde_json::Value) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = PropertyService::new(db);
        service.update(&id, &user_id, &data)
    })
}

#[tauri::command]
fn properties_delete(app: State<AppData>, token: String, id: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = PropertyService::new(db);
        service.delete(&id, &user_id)
    })
}

#[tauri::command]
fn properties_change_status(app: State<AppData>, token: String, id: String, status: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = PropertyService::new(db);
        service.change_status(&id, &user_id, &status)
    })
}

#[tauri::command]
fn properties_add_update(app: State<AppData>, token: String, id: String, title: String, description: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = PropertyService::new(db);
        service.add_update(&id, &user_id, &title, &description)
    })
}

#[tauri::command]
fn properties_add_image(
    app: State<AppData>,
    token: String,
    id: String,
    image_url: Option<String>,
    is_main: Option<bool>,
    data: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    let url = pick_image_url(image_url, data.as_ref()).ok_or_else(|| "ملف الصورة مطلوب".to_string())?;
    // الصور تُخزَّن كملفات فعلية داخل files\properties: data URL يُفك ويُكتب ملفًا
    // وتُحفظ في القاعدة relative_path فقط. القيم غير data (مسارات موجودة) تمر كما هي.
    let url = if url.starts_with("data:") {
        let store = db::files::FileStore::new(&app.app_data_dir)?;
        let (relative, _name) = db::images_migration::store_data_url_as_file(&store, &url)?;
        relative
    } else {
        url
    };
    let make_main = is_main
        .or_else(|| {
            data.as_ref().and_then(|d| {
                d.get("isMain")
                    .or_else(|| d.get("is_main"))
                    .and_then(|v| v.as_bool())
            })
        })
        .unwrap_or(false);
    with_db(&app, |db| {
        let service = PropertyService::new(db);
        service.add_image(&id, &user_id, &url, make_main)
    })
}

/// قراءة صورة عقار مخزنة كملف وإعادتها كـ data URL للعرض في الواجهة.
#[tauri::command]
fn property_image_read(app: State<AppData>, token: String, path: String) -> Result<String, String> {
    let _ = get_user_id(&app, &token)?;
    if !path.starts_with("properties/") || path.contains("..") || path.contains('\\') {
        return Err("مسار صورة غير مسموح".to_string());
    }
    let store = db::files::FileStore::new(&app.app_data_dir)?;
    let abs = store.absolute_from_relative(&path);
    let bytes = std::fs::read(&abs).map_err(|_| "الصورة غير موجودة".to_string())?;
    let mime = match abs.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => "image/jpeg",
    };
    use base64::Engine;
    Ok(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

fn pick_image_url(direct: Option<String>, data: Option<&serde_json::Value>) -> Option<String> {
    let from_direct = direct.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    if from_direct.is_some() {
        return from_direct;
    }
    let d = data?;
    d.get("imageUrl")
        .or_else(|| d.get("image_url"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[tauri::command]
fn properties_images(app: State<AppData>, token: String, id: String) -> Result<Vec<serde_json::Value>, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        if db.get_property(&id, &user_id)?.is_none() {
            return Err("العرض غير موجود".to_string());
        }
        let imgs = db.list_property_images(&id)?;
        Ok(imgs
            .iter()
            .map(|i| serde_json::json!({ "id": i.id, "image_url": i.image_url }))
            .collect())
    })
}

#[tauri::command]
fn properties_delete_image(app: State<AppData>, token: String, image_id: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = PropertyService::new(db);
        service.delete_image(&image_id, &user_id)
    })
}

#[tauri::command]
fn properties_duplicate(app: State<AppData>, token: String, id: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = PropertyService::new(db);
        service.duplicate(&id, &user_id)
    })
}

// ===== Commands: العملاء =====

#[tauri::command]
fn clients_list(
    app: State<AppData>,
    token: String,
    page: Option<i64>,
    limit: Option<i64>,
    search: Option<String>,
    role: Option<String>,
    relationship: Option<String>,
    city: Option<String>,
    status: Option<String>,
    needs_followup: Option<bool>,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = ClientService::new(db);
        service.list(
            &user_id,
            page.unwrap_or(1),
            limit.unwrap_or(20),
            &search.unwrap_or_default(),
            // relationship اسم قديم للمعامل نفسه؛ يُقبل للتوافق.
            &role.clone().or(relationship.clone()).unwrap_or_default(),
            &city.unwrap_or_default(),
            &status.unwrap_or_default(),
            needs_followup.unwrap_or(false),
        )
    })
}

#[tauri::command]
fn clients_options(app: State<AppData>, token: String, search: Option<String>) -> Result<Vec<serde_json::Value>, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = ClientService::new(db);
        service.options(&user_id, &search.clone().unwrap_or_default())
    })
}

#[tauri::command]
fn clients_get(app: State<AppData>, token: String, id: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = ClientService::new(db);
        service.get(&id, &user_id)
    })
}

#[tauri::command]
fn clients_create(app: State<AppData>, token: String, data: serde_json::Value) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = ClientService::new(db);
        service.create(&user_id, &data)
    })
}

#[tauri::command]
fn clients_update(app: State<AppData>, token: String, id: String, data: serde_json::Value) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = ClientService::new(db);
        service.update(&id, &user_id, &data)
    })
}

#[tauri::command]
fn clients_delete(app: State<AppData>, token: String, id: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = ClientService::new(db);
        service.delete(&id, &user_id)
    })
}

#[tauri::command]
fn clients_add_communication(
    app: State<AppData>,
    token: String,
    id: String,
    comm_type: Option<String>,
    r#type: Option<String>,
    subject: Option<String>,
    notes: Option<String>,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    let comm_type = comm_type.or(r#type).unwrap_or_else(|| "note".to_string());
    let subject = subject.unwrap_or_default();
    let notes = notes.unwrap_or_default();
    with_db(&app, |db| {
        let service = ClientService::new(db);
        service.add_communication(&id, &user_id, &comm_type, &subject, &notes)
    })
}

#[tauri::command]
fn clients_link_property(app: State<AppData>, token: String, id: String, property_id: String, link_type: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = ClientService::new(db);
        service.link_property(&id, &user_id, &property_id, &link_type)
    })
}

#[tauri::command]
fn clients_unlink_property(app: State<AppData>, token: String, id: String, property_id: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = ClientService::new(db);
        service.unlink_property(&id, &user_id, &property_id)
    })
}

// ===== Commands: المؤشر =====

#[tauri::command]
async fn market_list(
    app: State<'_, AppData>,
    token: String,
    page: Option<i64>,
    limit: Option<i64>,
    city: Option<String>,
    district: Option<String>,
    property_type: Option<String>,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = MarketService::new(db);
        service.list(
            &user_id,
            page.unwrap_or(1),
            limit.unwrap_or(20),
            &city.unwrap_or_default(),
            &district.unwrap_or_default(),
            &property_type.unwrap_or_default(),
        )
    })
}

#[tauri::command]
async fn market_stats(app: State<'_, AppData>, token: String, city: Option<String>, district: Option<String>, property_type: Option<String>) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = MarketService::new(db);
        service.stats(&user_id, &city.unwrap_or_default(), &district.unwrap_or_default(), &property_type.unwrap_or_default())
    })
}

#[tauri::command]
async fn market_trends(app: State<'_, AppData>, token: String, city: Option<String>, district: Option<String>, property_type: Option<String>) -> Result<Vec<serde_json::Value>, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = MarketService::new(db);
        service.trends(&user_id, &city.unwrap_or_default(), &district.unwrap_or_default(), &property_type.unwrap_or_default())
    })
}

#[tauri::command]
fn market_create(app: State<AppData>, token: String, data: serde_json::Value) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = MarketService::new(db);
        service.create(&user_id, &data)
    })
}

#[tauri::command]
fn market_update(app: State<AppData>, token: String, id: String, data: serde_json::Value) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = MarketService::new(db);
        service.update(&id, &user_id, &data)
    })
}

#[tauri::command]
fn market_bulk_create(app: State<AppData>, token: String, items: Vec<serde_json::Value>) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = MarketService::new(db);
        service.bulk_create(&user_id, &items)
    })
}

#[tauri::command]
fn market_delete(app: State<AppData>, token: String, id: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = MarketService::new(db);
        service.delete(&id, &user_id)
    })
}

#[tauri::command]
fn market_delete_many(app: State<AppData>, token: String, ids: Vec<String>) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = MarketService::new(db);
        service.delete_many(&ids, &user_id)
    })
}

#[tauri::command]
async fn market_delete_filtered(app: State<'_, AppData>, token: String, city: Option<String>, district: Option<String>, property_type: Option<String>) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let deleted = MarketService::new(db).delete_filtered(
            &user_id,
            &city.unwrap_or_default(),
            &district.unwrap_or_default(),
            &property_type.unwrap_or_default(),
        )?;
        Ok(serde_json::json!({ "deleted": deleted }))
    })
}

#[tauri::command]
fn market_purge_audit(app: State<AppData>, token: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = MarketService::new(db);
        service.purge_audit_leftovers(&user_id)
    })
}

#[tauri::command]
fn market_disclaimer() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "disclaimer": "تنبيه: البيانات والمؤشرات المعروضة في هذا القسم ناتجة عن البيانات التي يقوم المستخدم بإدخالها أو استيرادها بنفسه، وتمثل قاعدة بيانات ومرجعًا شخصيًا للمستخدم فقط، ولا تمثل قراءة أو تحليلًا للسوق العقاري العام، ولا تعد تقييمًا عقاريًا أو استشارة استثمارية أو مؤشرًا رسميًا للأسعار."
    }))
}

#[tauri::command]
async fn market_filter_cities(app: State<'_, AppData>, token: String) -> Result<Vec<String>, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        MarketService::new(db).filter_cities(&user_id)
    })
}

#[tauri::command]
async fn market_filter_districts(app: State<'_, AppData>, token: String, city: String) -> Result<Vec<String>, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        MarketService::new(db).filter_districts(&user_id, &city)
    })
}

#[tauri::command]
async fn market_geo_summary(app: State<'_, AppData>, token: String, city: Option<String>, district: Option<String>, mode: Option<String>, limit: Option<i64>) -> Result<Vec<serde_json::Value>, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        MarketService::new(db).geo_summary(
            &user_id,
            &city.unwrap_or_default(),
            &district.unwrap_or_default(),
            &mode.unwrap_or_else(|| "city".to_string()),
            limit.unwrap_or(500),
        )
    })
}

// ===== Commands: الإعدادات =====

#[tauri::command]
async fn official_integrations_request(app: State<'_, AppData>, token: String, data: serde_json::Value) -> Result<serde_json::Value, String> {
    let user = get_user_id(&app, &token)?;
    let path = app.db_path.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let conn = open_migrated_conn(&path)?;
        let id = data["id"].as_str().unwrap_or("");
        match data["action"].as_str().unwrap_or("") {
            "list" => official_integrations::list(&conn, &user),
            "save" => official_integrations::save(&conn, &user, &data),
            "delete" => official_integrations::delete(&conn, &user, id),
            "test" => official_integrations::test_connection(&conn, &user, id),
            "resolve" => {
                let (country, license_type): (String, String) = if let Some(country) = data["country"].as_str().filter(|c| !c.is_empty()) {
                    (country.into(), data["licenseType"].as_str().unwrap_or("").into())
                } else {
                    conn.query_row("SELECT country, COALESCE(fal_license_type,'') FROM settings WHERE user_id=?1", [&user], |r| Ok((r.get(0)?, r.get(1)?))).map_err(|_| "integration.storage")?
                };
                official_integrations::resolve(&conn, &user, &country, &license_type)
            },
            _ => Err("integration.invalid".into()),
        }
    }).await.map_err(|_| "integration.failed".to_string())?
}

#[tauri::command]
fn settings_get(app: State<AppData>, token: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = SettingsService::new(db);
        service.get(&user_id)
    })
}

#[tauri::command]
fn settings_update(app: State<AppData>, token: String, data: serde_json::Value) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = SettingsService::new(db);
        service.update(&user_id, &data)
    })
}

// ===== Commands: لوحة المعلومات =====

#[tauri::command]
async fn dashboard_stats(app: State<'_, AppData>, token: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    let mut stats = with_db(&app, |db| {
        let service = DashboardService::new(db);
        service.stats(&user_id)
    })?;
    let conn = open_migrated_conn(&app.db_path)?;
    stats["contracts"] = crate::contracts::summary(&conn, &user_id)?;
    Ok(stats)
}

#[tauri::command]
async fn dashboard_activity(app: State<'_, AppData>, token: String) -> Result<Vec<serde_json::Value>, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = DashboardService::new(db);
        service.activity(&user_id)
    })
}

#[tauri::command]
async fn dashboard_aggregates(
    app: State<'_, AppData>,
    token: String,
    country: Option<String>,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    let conn = open_migrated_conn(&app.db_path)?;
    let mut result = capacity::aggregates(&conn, &user_id, country.as_deref())?;
    if let Some(obj) = result.as_object_mut() {
        obj.insert("campaign_top_details".into(), serde_json::Value::Array(
            capacity::campaign_top_details(&conn, &user_id)?
                .into_iter()
                .collect(),
        ));
    }
    Ok(result)
}

// ===== Commands: البروشرات =====

#[tauri::command]
fn brochures_templates() -> Result<Vec<serde_json::Value>, String> {
    Ok(BrochureService::templates())
}

#[tauri::command]
async fn brochures_list(
    app: State<'_, AppData>,
    token: String,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = BrochureService::new(db);
        let lim = limit.unwrap_or(0);
        let off = offset.unwrap_or(0);
        let items = service.list(&user_id, lim, off)?;
        if lim > 0 {
            // الترقيم الخلفي: إجمالي حقيقي مع كائن {items, total}
            let total = db.count_brochures(&user_id)?;
            Ok(serde_json::json!({ "items": items, "total": total }))
        } else {
            // السلوك القديم: مصفوفة كاملة (مكتبة العروض/تصميم العروض)
            Ok(serde_json::Value::Array(items))
        }
    })
}

#[tauri::command]
fn brochures_get(app: State<AppData>, token: String, id: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = BrochureService::new(db);
        service.get(&id, &user_id)
    })
}

#[tauri::command]
fn brochures_create(
    app: State<AppData>,
    token: String,
    property_id: Option<String>,
    template_id: Option<String>,
    title: Option<String>,
    content: Option<serde_json::Value>,
    data: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    // دعم body المسطح عبر api.ts (data) وكذلك الوسائط المباشرة
    let merged = data.unwrap_or_else(|| serde_json::json!({}));
    let property_id = property_id
        .or_else(|| merged.get("property_id").and_then(|v| v.as_str()).map(|s| s.to_string()));
    let template_id = template_id
        .or_else(|| merged.get("template_id").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .unwrap_or_else(|| "royal-sheet".to_string());
    let title = title.or_else(|| merged.get("title").and_then(|v| v.as_str()).map(|s| s.to_string()));
    let content = content.or_else(|| merged.get("content").cloned());

    with_db(&app, |db| {
        let service = BrochureService::new(db);
        service.create(
            &user_id,
            property_id.as_deref(),
            &template_id,
            title.as_deref(),
            content.as_ref(),
        )
    })
}

#[tauri::command]
fn brochures_update(
    app: State<AppData>,
    token: String,
    id: String,
    property_id: Option<String>,
    template_id: Option<String>,
    title: Option<String>,
    content: Option<serde_json::Value>,
    data: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    let merged = data.unwrap_or_else(|| serde_json::json!({}));
    let property_id = property_id
        .or_else(|| merged.get("property_id").and_then(|v| v.as_str()).map(|s| s.to_string()));
    let template_id = template_id
        .or_else(|| merged.get("template_id").and_then(|v| v.as_str()).map(|s| s.to_string()));
    let title = title.or_else(|| merged.get("title").and_then(|v| v.as_str()).map(|s| s.to_string()));
    let content = content.or_else(|| merged.get("content").cloned());

    with_db(&app, |db| {
        let service = BrochureService::new(db);
        service.update(
            &id,
            &user_id,
            template_id.as_deref(),
            title.as_deref(),
            content.as_ref(),
            property_id.as_deref(),
        )
    })
}

#[tauri::command]
fn brochures_delete(app: State<AppData>, token: String, id: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = BrochureService::new(db);
        service.delete(&id, &user_id)
    })
}

// ===== Commands: الحملات التسويقية =====

#[tauri::command]
fn campaigns_list(
    app: State<AppData>,
    token: String,
    page: Option<i64>,
    limit: Option<i64>,
    platform: Option<String>,
    property_id: Option<String>,
    period_from: Option<String>,
    period_to: Option<String>,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = CampaignService::new(db);
        service.list(
            &user_id,
            page.unwrap_or(1),
            limit.unwrap_or(20),
            &platform.unwrap_or_default(),
            &property_id.unwrap_or_default(),
            &period_from.unwrap_or_default(),
            &period_to.unwrap_or_default(),
        )
    })
}

#[tauri::command]
fn campaigns_stats(
    app: State<AppData>,
    token: String,
    platform: Option<String>,
    property_id: Option<String>,
    period_from: Option<String>,
    period_to: Option<String>,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = CampaignService::new(db);
        service.stats(
            &user_id,
            &platform.unwrap_or_default(),
            &property_id.unwrap_or_default(),
            &period_from.unwrap_or_default(),
            &period_to.unwrap_or_default(),
        )
    })
}

#[tauri::command]
fn campaigns_get(app: State<AppData>, token: String, id: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = CampaignService::new(db);
        service.get(&id, &user_id)
    })
}

#[tauri::command]
fn campaigns_create(
    app: State<AppData>,
    token: String,
    data: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = CampaignService::new(db);
        service.create(&user_id, &data)
    })
}

#[tauri::command]
fn campaigns_update(
    app: State<AppData>,
    token: String,
    id: String,
    data: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    let payload = data.unwrap_or_else(|| serde_json::json!({}));
    // DB write قصيرة → تحرير القفل → HTTP اختياري → DB write قصيرة
    let (mut result, remote_destination) = with_db(&app, |db| {
        CampaignService::new(db).update(&id, &user_id, &payload)
    })?;
    if let Some(dest) = remote_destination {
        if let Ok(base) = crate::tracking_client::resolve_endpoint_base() {
            let installation = tracking_installation_id(&app);
            let existing = result["tracking_code"].as_str().unwrap_or("").to_string();
            // رابط قائم → تحديث الوجهة عبر PATCH؛ لا رابط → إنشاء أول مرة
            let outcome = if existing.is_empty() {
                crate::tracking_client::register_link(&base, &installation, &dest)
            } else {
                crate::tracking_client::update_destination(&base, &installation, &existing, &dest)
            };
            if let Ok(registered) = outcome {
                result = with_db(&app, |db| {
                    CampaignService::new(db).apply_remote_tracking_registration(
                        &id,
                        &user_id,
                        &registered,
                    )
                })?;
            }
        }
    }
    Ok(result)
}

#[tauri::command]
fn campaigns_delete(app: State<AppData>, token: String, id: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    let code = with_db(&app, |db| {
        CampaignService::new(db).delete_load_tracking_code(&id, &user_id)
    })?;
    if let Some(code) = code {
        if let Ok(base) = crate::tracking_client::resolve_endpoint_base() {
            let installation = tracking_installation_id(&app);
            let _ = crate::tracking_client::set_link_enabled(&base, &installation, &code, false);
        }
    }
    with_db(&app, |db| CampaignService::new(db).delete_persist(&id, &user_id))
}

/// هوية التثبيت المرسلة لخدمة التتبع — تحديد لحدود الاستخدام، ليست مصادقة.
fn tracking_installation_id(app: &AppData) -> String {
    license::LicenseStore::new(&app.app_data_dir)
        .installation_id()
        .unwrap_or_default()
}

#[tauri::command]
fn campaigns_tracking_enable(
    app: State<AppData>,
    token: String,
    id: String,
    data: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let _ = data; // لا يُقبل عنوان خدمة من الواجهة
    let user_id = get_user_id(&app, &token)?;
    let original = with_db(&app, |db| {
        CampaignService::new(db).tracking_enable_load_destination(&id, &user_id)
    })?;
    let base = crate::tracking_client::resolve_endpoint_base()?;
    let installation = tracking_installation_id(&app);
    let registered = crate::tracking_client::register_link(&base, &installation, &original)?;
    let _ = crate::tracking_client::set_link_enabled(&base, &installation, &registered.tracking_code, true);
    with_db(&app, |db| {
        CampaignService::new(db).tracking_enable_persist(&id, &user_id, &original, &registered)
    })?;
    // مزامنة الإحصاءات خارج القفل أيضًا — عند الفشل نُرجع الحملة كما حُفظت
    match campaigns_tracking_sync_inner(&app, &user_id, &id) {
        Ok(v) => Ok(v),
        Err(_) => with_db(&app, |db| CampaignService::new(db).get(&id, &user_id)),
    }
}

#[tauri::command]
fn campaigns_tracking_status(app: State<AppData>, token: String) -> Result<serde_json::Value, String> {
    let _ = get_user_id(&app, &token)?;
    Ok(serde_json::json!({
        "available": crate::tracking_client::is_tracking_configured()
    }))
}

#[tauri::command]
fn campaigns_tracking_disable(
    app: State<AppData>,
    token: String,
    id: String,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    let code = with_db(&app, |db| {
        CampaignService::new(db).tracking_disable_load_code(&id, &user_id)
    })?;
    if let Some(code) = code {
        if let Ok(base) = crate::tracking_client::resolve_endpoint_base() {
            let installation = tracking_installation_id(&app);
            let _ = crate::tracking_client::set_link_enabled(&base, &installation, &code, false);
        }
    }
    with_db(&app, |db| CampaignService::new(db).tracking_disable_persist(&id, &user_id))
}

/// مزامنة إحصاءات التتبع: قراءة DB → HTTP → كتابة DB (بدون احتجاز القفل أثناء الشبكة).
fn campaigns_tracking_sync_inner(
    app: &AppData,
    user_id: &str,
    id: &str,
) -> Result<serde_json::Value, String> {
    let snapshot = with_db(app, |db| {
        CampaignService::new(db).tracking_sync_load_snapshot(id, user_id)
    })?;
    let base = crate::tracking_client::resolve_endpoint_base()?;
    let installation = tracking_installation_id(app);
    let stats = crate::tracking_client::fetch_stats(&base, &installation, &snapshot.tracking_code)?;
    with_db(app, |db| {
        CampaignService::new(db).tracking_sync_apply_stats(&snapshot, &stats)
    })
}

#[tauri::command]
fn campaigns_tracking_sync(
    app: State<AppData>,
    token: String,
    id: String,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    campaigns_tracking_sync_inner(&app, &user_id, &id)
}

// ===== Commands: المواعيد =====

#[tauri::command]
fn appointments_list(
    app: State<AppData>,
    token: String,
    from: Option<String>,
    to: Option<String>,
) -> Result<Vec<serde_json::Value>, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = AppointmentService::new(db);
        let items = service.list(&user_id, &from.unwrap_or_default(), &to.unwrap_or_default())?;
        items
            .into_iter()
            .map(|item| serde_json::to_value(item).map_err(|e| e.to_string()))
            .collect()
    })
}

#[tauri::command]
fn appointments_get(app: State<AppData>, token: String, id: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = AppointmentService::new(db);
        service.get(&id, &user_id)
    })
}

#[tauri::command]
fn appointments_create(
    app: State<AppData>,
    token: String,
    data: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = AppointmentService::new(db);
        service.create(&user_id, &data)
    })
}

#[tauri::command]
fn appointments_update(
    app: State<AppData>,
    token: String,
    id: String,
    data: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    let payload = data.unwrap_or_else(|| serde_json::json!({}));
    with_db(&app, |db| {
        let service = AppointmentService::new(db);
        service.update(&id, &user_id, &payload)
    })
}

#[tauri::command]
fn appointments_delete(app: State<AppData>, token: String, id: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = AppointmentService::new(db);
        service.delete(&id, &user_id)
    })
}

// ===== Commands: المالية والعمولات =====

#[tauri::command]
fn finance_summary(
    app: State<AppData>,
    token: String,
    period_from: Option<String>,
    period_to: Option<String>,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        FinanceService::new(db).summary(
            &user_id,
            &period_from.unwrap_or_default(),
            &period_to.unwrap_or_default(),
        )
    })
}

#[tauri::command]
fn finance_ledger(
    app: State<AppData>,
    token: String,
    period_from: Option<String>,
    period_to: Option<String>,
    kind: Option<String>,
    status: Option<String>,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        FinanceService::new(db).ledger(
            &user_id,
            &period_from.unwrap_or_default(),
            &period_to.unwrap_or_default(),
            &kind.unwrap_or_default(),
            &status.unwrap_or_default(),
        )
    })
}

#[tauri::command]
fn finance_receivables(
    app: State<AppData>,
    token: String,
    period_from: Option<String>,
    period_to: Option<String>,
    status: Option<String>,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        FinanceService::new(db).receivables(
            &user_id,
            &period_from.unwrap_or_default(),
            &period_to.unwrap_or_default(),
            &status.unwrap_or_default(),
        )
    })
}

#[tauri::command]
async fn finance_operations_list(
    app: State<'_, AppData>,
    token: String,
    period_from: Option<String>,
    period_to: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        FinanceService::new(db).list_operations(
            &user_id,
            &period_from.unwrap_or_default(),
            &period_to.unwrap_or_default(),
            limit.unwrap_or(0),
            offset.unwrap_or(0),
        )
    })
}

#[tauri::command]
fn finance_operations_get(app: State<AppData>, token: String, id: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| FinanceService::new(db).get_operation(&id, &user_id))
}

#[tauri::command]
fn finance_operations_create(
    app: State<AppData>,
    token: String,
    data: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| FinanceService::new(db).create_operation(&user_id, &data))
}

#[tauri::command]
fn finance_operations_update(
    app: State<AppData>,
    token: String,
    id: String,
    data: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    let payload = data.unwrap_or_else(|| serde_json::json!({}));
    with_db(&app, |db| FinanceService::new(db).update_operation(&id, &user_id, &payload))
}

#[tauri::command]
fn finance_operations_delete(app: State<AppData>, token: String, id: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| FinanceService::new(db).delete_operation(&id, &user_id))
}

#[tauri::command]
async fn finance_invoices_list(
    app: State<'_, AppData>,
    token: String,
    period_from: Option<String>,
    period_to: Option<String>,
    status: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        FinanceService::new(db).list_invoices(
            &user_id,
            &period_from.unwrap_or_default(),
            &period_to.unwrap_or_default(),
            &status.unwrap_or_default(),
            limit.unwrap_or(0),
            offset.unwrap_or(0),
        )
    })
}

#[tauri::command]
fn finance_invoices_get(app: State<AppData>, token: String, id: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| FinanceService::new(db).get_invoice(&id, &user_id))
}

#[tauri::command]
fn finance_invoices_create(
    app: State<AppData>,
    token: String,
    data: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| FinanceService::new(db).create_invoice(&user_id, &data))
}

#[tauri::command]
fn finance_invoices_update(
    app: State<AppData>,
    token: String,
    id: String,
    data: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    let payload = data.unwrap_or_else(|| serde_json::json!({}));
    with_db(&app, |db| FinanceService::new(db).update_invoice(&id, &user_id, &payload))
}

#[tauri::command]
fn finance_invoices_delete(app: State<AppData>, token: String, id: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| FinanceService::new(db).delete_invoice(&id, &user_id))
}

#[tauri::command]
async fn finance_commissions_list(
    app: State<'_, AppData>,
    token: String,
    period_from: Option<String>,
    period_to: Option<String>,
    status: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        FinanceService::new(db).list_commissions(
            &user_id,
            &period_from.unwrap_or_default(),
            &period_to.unwrap_or_default(),
            &status.unwrap_or_default(),
            limit.unwrap_or(0),
            offset.unwrap_or(0),
        )
    })
}

#[tauri::command]
fn finance_commissions_get(app: State<AppData>, token: String, id: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| FinanceService::new(db).get_commission(&id, &user_id))
}

#[tauri::command]
fn finance_commissions_create(
    app: State<AppData>,
    token: String,
    data: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| FinanceService::new(db).create_commission(&user_id, &data))
}

#[tauri::command]
fn finance_commissions_update(
    app: State<AppData>,
    token: String,
    id: String,
    data: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    let payload = data.unwrap_or_else(|| serde_json::json!({}));
    with_db(&app, |db| FinanceService::new(db).update_commission(&id, &user_id, &payload))
}

#[tauri::command]
fn finance_commissions_delete(app: State<AppData>, token: String, id: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| FinanceService::new(db).delete_commission(&id, &user_id))
}

#[tauri::command]
async fn finance_expenses_list(
    app: State<'_, AppData>,
    token: String,
    period_from: Option<String>,
    period_to: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        FinanceService::new(db).list_expenses(
            &user_id,
            &period_from.unwrap_or_default(),
            &period_to.unwrap_or_default(),
            limit.unwrap_or(0),
            offset.unwrap_or(0),
        )
    })
}

#[tauri::command]
fn finance_expenses_get(app: State<AppData>, token: String, id: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| FinanceService::new(db).get_expense(&id, &user_id))
}

#[tauri::command]
fn finance_expenses_create(
    app: State<AppData>,
    token: String,
    data: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| FinanceService::new(db).create_expense(&user_id, &data))
}

#[tauri::command]
fn finance_expenses_update(
    app: State<AppData>,
    token: String,
    id: String,
    data: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    let payload = data.unwrap_or_else(|| serde_json::json!({}));
    with_db(&app, |db| FinanceService::new(db).update_expense(&id, &user_id, &payload))
}

#[tauri::command]
fn finance_expenses_delete(app: State<AppData>, token: String, id: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| FinanceService::new(db).delete_expense(&id, &user_id))
}

#[tauri::command]
async fn finance_payments_list(
    app: State<'_, AppData>,
    token: String,
    period_from: Option<String>,
    period_to: Option<String>,
    target_type: Option<String>,
    target_id: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        FinanceService::new(db).list_payments(
            &user_id,
            &period_from.unwrap_or_default(),
            &period_to.unwrap_or_default(),
            &target_type.unwrap_or_default(),
            &target_id.unwrap_or_default(),
            limit.unwrap_or(0),
            offset.unwrap_or(0),
        )
    })
}

#[tauri::command]
fn finance_payments_get(app: State<AppData>, token: String, id: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| FinanceService::new(db).get_payment(&id, &user_id))
}

#[tauri::command]
fn finance_payments_create(
    app: State<AppData>,
    token: String,
    data: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| FinanceService::new(db).create_payment(&user_id, &data))
}

#[tauri::command]
fn finance_payments_update(
    app: State<AppData>,
    token: String,
    id: String,
    data: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    let payload = data.unwrap_or_else(|| serde_json::json!({}));
    with_db(&app, |db| FinanceService::new(db).update_payment(&id, &user_id, &payload))
}

#[tauri::command]
fn finance_payments_delete(app: State<AppData>, token: String, id: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| FinanceService::new(db).delete_payment(&id, &user_id))
}

#[tauri::command]
fn finance_invoice_pdf(
    app: State<AppData>,
    token: String,
    id: String,
    open: Option<bool>,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    let (invoice, payments, settings) = with_db(&app, |db| {
        FinanceService::new(db).invoice_pdf_payload(&id, &user_id)
    })?;
    let html = crate::finance_pdf::invoice_html(&invoice, &payments, &settings);
    let stem = invoice
        .get("number")
        .and_then(|v| v.as_str())
        .unwrap_or("invoice");
    let written = crate::finance_pdf::generate_pdf(
        &app.app_data_dir,
        "invoice",
        stem,
        &html,
    )?;
    let opened = open.unwrap_or(true);
    if opened {
        crate::finance_pdf::open_pdf_file(std::path::Path::new(&written.pdf_path))?;
    }
    Ok(serde_json::json!({
        "pdf_path": written.pdf_path,
        "opened": opened
    }))
}

#[tauri::command]
fn finance_record_pdf(
    app: State<AppData>,
    token: String,
    record_kind: String,
    id: String,
    open: Option<bool>,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    let (record, settings) = with_db(&app, |db| {
        FinanceService::new(db).record_pdf_payload(&record_kind, &id, &user_id)
    })?;
    let document_kind = if record_kind.starts_with("receivable_") { "receivable" } else { record_kind.as_str() };
    let html = crate::finance_pdf::record_html(document_kind, &record, &settings)?;
    let written = crate::finance_pdf::generate_pdf(
        &app.app_data_dir,
        document_kind,
        &id,
        &html,
    )?;
    let opened = open.unwrap_or(true);
    if opened {
        crate::finance_pdf::open_pdf_file(std::path::Path::new(&written.pdf_path))?;
    }
    Ok(serde_json::json!({ "pdf_path": written.pdf_path, "opened": opened }))
}

#[tauri::command]
fn finance_report_pdf(
    app: State<AppData>,
    token: String,
    period_from: Option<String>,
    period_to: Option<String>,
    open: Option<bool>,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    let from = period_from.unwrap_or_default();
    let to = period_to.unwrap_or_default();
    let (summary, settings) = with_db(&app, |db| {
        FinanceService::new(db).report_pdf_payload(&user_id, &from, &to)
    })?;
    let html = crate::finance_pdf::report_html(&summary, &settings, &from, &to);
    let written = crate::finance_pdf::generate_pdf(
        &app.app_data_dir,
        "report",
        "financial",
        &html,
    )?;
    let opened = open.unwrap_or(true);
    if opened {
        crate::finance_pdf::open_pdf_file(std::path::Path::new(&written.pdf_path))?;
    }
    Ok(serde_json::json!({
        "pdf_path": written.pdf_path,
        "opened": opened,
        "summary": summary
    }))
}

#[tauri::command]
fn finance_open_pdf(app: State<AppData>, path: String) -> Result<serde_json::Value, String> {
    if !crate::finance_pdf::pdf_parent_ok(&app.app_data_dir, &path) {
        return Err("مسار ملف PDF غير مسموح.".into());
    }
    crate::finance_pdf::open_pdf_file(std::path::Path::new(&path))?;
    Ok(serde_json::json!({ "opened": true, "pdf_path": path }))
}

// ===== Commands: التصدير =====

#[tauri::command]
fn export_all(app: State<AppData>, token: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = ExportService::new(db);
        service.all(&user_id)
    })
}

#[tauri::command]
fn export_properties(app: State<AppData>, token: String) -> Result<Vec<db::Property>, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = ExportService::new(db);
        service.properties(&user_id)
    })
}

#[tauri::command]
fn export_clients(app: State<AppData>, token: String) -> Result<Vec<db::Client>, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = ExportService::new(db);
        service.clients(&user_id)
    })
}

#[tauri::command]
fn export_market(app: State<AppData>, token: String) -> Result<Vec<db::MarketReferenceData>, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = ExportService::new(db);
        service.market(&user_id)
    })
}

// ===== Commands: نظام / نسخ احتياطي =====

#[tauri::command]
fn system_info(app: State<AppData>, token: String) -> Result<serde_json::Value, String> {
    let _user_id = get_user_id(&app, &token)?;
    let conn = db::open_configured(&app.db_path)?;
    let schema_version = db::migrations::schema_version(&conn)?;
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    let backups = db::backup::list_backup_packages(&app.app_data_dir).unwrap_or_default();
    Ok(serde_json::json!({
        "local_first": true,
        "db_path": app.db_path.to_string_lossy(),
        "app_data_dir": app.app_data_dir.to_string_lossy(),
        "files_dir": app.app_data_dir.join("files").to_string_lossy(),
        "backups_dir": app.app_data_dir.join("backups").to_string_lossy(),
        "schema_version": schema_version,
        "integrity_ok": integrity == "ok",
        "restore_ui_available": true,
        "backups_count": backups.len()
    }))
}

#[tauri::command]
fn backup_create(app: State<AppData>, token: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    let conn = db::open_configured(&app.db_path)?;
    let store = db::files::FileStore::new(&app.app_data_dir)?;
    let pkg = db::backup::create_backup_package(
        &app.app_data_dir,
        &app.db_path,
        &conn,
        &store,
        Some(&user_id),
    )?;
    Ok(serde_json::json!({
        "path": pkg.path.to_string_lossy(),
        "folder_name": pkg.folder_name,
        "sequence": pkg.sequence,
        "stamp": pkg.stamp,
        "schema_version": pkg.schema_version,
        "restore_supported": true,
        "message": format!("تم إنشاء النسخة رقم {} بتاريخ {}", pkg.sequence, pkg.stamp)
    }))
}

#[tauri::command]
fn backup_list(app: State<AppData>, token: String) -> Result<serde_json::Value, String> {
    let _user_id = get_user_id(&app, &token)?;
    let items = db::backup::list_backup_packages(&app.app_data_dir)?;
    Ok(serde_json::json!({ "items": items }))
}

#[tauri::command]
fn backup_restore(app: State<AppData>, token: String, package_path: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    let path = std::path::PathBuf::from(package_path.trim());
    if !path.is_dir() {
        return Err("مسار النسخة غير موجود".into());
    }
    let backups_root = app.app_data_dir.join("backups");
    let canon_pkg = path.canonicalize().map_err(|e| e.to_string())?;
    let canon_root = backups_root
        .canonicalize()
        .map_err(|_| "مجلد النسخ الاحتياطي غير موجود بعد".to_string())?;
    if !canon_pkg.starts_with(&canon_root) {
        return Err("يُسمح فقط باستعادة النسخ من مجلد النسخ الاحتياطي للتطبيق".into());
    }

    // نسخة أمان قبل الاستعادة — الفشل هنا يمنع الاستعادة تمامًا
    {
        let conn = db::open_configured(&app.db_path)?;
        let store = db::files::FileStore::new(&app.app_data_dir)?;
        let _safety = db::backup::create_pre_restore_safety_backup(
            &app.app_data_dir,
            &app.db_path,
            &conn,
            &store,
            Some(&user_id),
        )?;
    }

    let hold = app.app_data_dir.join(".obr-restore-hold.db");
    {
        let outer = app.db.lock().map_err(|e| e.to_string())?;
        let mut inner = outer.db.lock().map_err(|e| e.to_string())?;
        // إطلاق قفل ملف القاعدة على Windows عبر تحويل الاتصال لمسار مؤقت
        *inner = Box::new(db::SqliteDatabase::new(&hold).map_err(|e| e.to_string())?);
    }

    let restore_result = db::backup::restore_backup_package(&app.app_data_dir, &app.db_path, &path);

    // إعادة ربط الاتصال الحي دائمًا (القاعدة الأصلية عند فشل الاستعادة / الجديدة عند النجاح)
    let reconnect_result = (|| -> Result<(), String> {
        let outer = app.db.lock().map_err(|e| e.to_string())?;
        let mut inner = outer.db.lock().map_err(|e| e.to_string())?;
        *inner = Box::new(db::SqliteDatabase::new(&app.db_path).map_err(|e| e.to_string())?);
        Ok(())
    })();

    let _ = std::fs::remove_file(&hold);
    let _ = std::fs::remove_file(format!("{}-wal", hold.display()));
    let _ = std::fs::remove_file(format!("{}-shm", hold.display()));

    reconnect_result?;
    let restored = restore_result?;

    Ok(serde_json::json!({
        "ok": true,
        "message": "تمت استعادة النسخة الاحتياطية بنجاح.",
        "restored": restored
    }))
}

#[tauri::command]
fn backup_delete(app: State<AppData>, token: String, package_path: String) -> Result<serde_json::Value, String> {
    let _user_id = get_user_id(&app, &token)?;
    let path = std::path::PathBuf::from(package_path.trim());
    if !path.is_dir() {
        return Err("مسار النسخة غير موجود".into());
    }
    // يُسمح بالحذف فقط داخل مجلد النسخ الاحتياطي للتطبيق
    let backups_root = app.app_data_dir.join("backups");
    let canon_pkg = path.canonicalize().map_err(|e| e.to_string())?;
    let canon_root = backups_root
        .canonicalize()
        .map_err(|_| "مجلد النسخ الاحتياطي غير موجود بعد".to_string())?;
    if !canon_pkg.starts_with(&canon_root) || canon_pkg == canon_root {
        return Err("يُسمح فقط بحذف النسخ من مجلد النسخ الاحتياطي للتطبيق".into());
    }
    std::fs::remove_dir_all(&canon_pkg).map_err(|e| format!("تعذر حذف النسخة: {e}"))?;
    Ok(serde_json::json!({ "ok": true, "deleted": path.to_string_lossy() }))
}

#[tauri::command]
fn import_data_json(
    app: State<AppData>,
    token: String,
    payload: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    with_db(&app, |db| {
        let service = ExportService::new(db);
        service.import_all(&user_id, &payload)
    })
}

fn open_migrated_conn(db_path: &std::path::Path) -> Result<rusqlite::Connection, String> {
    let mut conn = db::open_configured(db_path)?;
    db::migrations::run_migrations(&mut conn)?;
    Ok(conn)
}

// ===== Commands: بيانات تجريبية =====

#[tauri::command]
fn demo_status(app: State<AppData>, token: String) -> Result<serde_json::Value, String> {
    let _user_id = get_user_id(&app, &token)?;
    let conn = open_migrated_conn(&app.db_path)?;
    db::demo::demo_status(&conn, db::demo::DEMO_BATCH_ID)
}

#[tauri::command]
fn demo_delete(app: State<AppData>, token: String) -> Result<serde_json::Value, String> {
    let _user_id = get_user_id(&app, &token)?;
    let mut conn = open_migrated_conn(&app.db_path)?;
    let before = db::demo::demo_status(&conn, db::demo::DEMO_BATCH_ID)?;
    let deleted = db::demo::delete_demo_batch(&mut conn, db::demo::DEMO_BATCH_ID, &app.app_data_dir)?;
    let after = db::demo::demo_status(&conn, db::demo::DEMO_BATCH_ID)?;
    Ok(serde_json::json!({
        "batch_id": db::demo::DEMO_BATCH_ID,
        "deleted": db::demo::counts_to_json(&deleted),
        "before": before,
        "after": after,
        "message": "تم حذف البيانات التدريبية فقط (obr-training-v1)."
    }))
}

#[tauri::command]
fn demo_reseed(app: State<AppData>, token: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    let mut conn = open_migrated_conn(&app.db_path)?;
    let result = db::demo::reseed_demo(&mut conn, &user_id, &app.app_data_dir)?;
    let status = db::demo::demo_status(&conn, db::demo::DEMO_BATCH_ID)?;
    Ok(serde_json::json!({
        "result": result,
        "status": status,
        "message": "أُعيد تحميل البيانات التدريبية. لم تُمس سجلات المستخدم غير التدريبية."
    }))
}

/// نسخة احتياطي مسماة + مسح تشغيلي + Seed. لا يُستدعى تلقائياً عند التثبيت.
#[tauri::command]
fn demo_prepare(app: State<AppData>, token: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    let mut conn = open_migrated_conn(&app.db_path)?;
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let folder = format!("pre-demo-reset-{stamp}");
    let store = db::files::FileStore::new(&app.app_data_dir)?;
    let pkg = db::backup::create_backup_package_named(
        &app.app_data_dir,
        &app.db_path,
        &conn,
        &store,
        Some(&user_id),
        &folder,
        "pre-demo-reset: backup before operational purge + demo seed",
        0,
        &stamp,
    )?;

    let db_ok = pkg.path.join("mawasib.db").is_file();
    let files_ok = pkg.path.join("files").exists();
    let manifest_ok = pkg.path.join("manifest.json").is_file();
    if !db_ok || !manifest_ok {
        return Err("فشل التحقق من حزمة النسخ الاحتياطي قبل المسح".into());
    }

    let (removed, seeded) = db::demo::prepare_demo_environment(&mut conn, &user_id, &app.app_data_dir)?;
    let status = db::demo::demo_status(&conn, db::demo::DEMO_BATCH_ID)?;

    Ok(serde_json::json!({
        "backup": {
            "path": pkg.path.to_string_lossy(),
            "schema_version": pkg.schema_version,
            "db_present": db_ok,
            "files_dir_present": files_ok,
            "manifest_present": manifest_ok
        },
        "removed": db::demo::counts_to_json(&removed),
        "seeded": db::demo::counts_to_json(&seeded),
        "status": status,
        "message": "اكتمل: Backup → تنظيف تشغيلي → Demo Seed. لن يُزرع تلقائياً على أجهزة العملاء."
    }))
}

// ===== الإعداد والتشغيل =====

fn main() {
    if std::env::args().any(|a| a == "--remove-license-installation-state") {
        let root = std::env::var_os("OBR_APP_DATA_DIR")
            .map(std::path::PathBuf::from)
            .map(Ok)
            .unwrap_or_else(dirs_next_app_data);
        let result = root.and_then(|root| license::remove_installation_state(&root));
        std::process::exit(if result.is_ok() { 0 } else { 1 });
    }
    // أداة تحقق للمطور: تشغيل Migrations وقراءة فعلية على قاعدة بيانات محددة.
    // تُستخدم لاختبار الترقية على قاعدة إنتاج قائمة وليس على قاعدة فارغة فقط.
    if let Some(arg) = std::env::args().find(|a| a.starts_with("--verify-db=")) {
        let path = arg.trim_start_matches("--verify-db=").to_string();
        match run_verify_db_cli(&path) {
            Ok(msg) => {
                println!("{msg}");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("VERIFY_DB_FAILED: {e}");
                std::process::exit(1);
            }
        }
    }

    // أداة تطوير لمرة واحدة: لا تُستدعى من مثبت العميل
    if std::env::args().any(|a| a == "--demo-prepare") {
        match run_demo_prepare_cli() {
            Ok(msg) => {
                println!("{msg}");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("DEMO_PREPARE_FAILED: {e}");
                std::process::exit(1);
            }
        }
    }
    if std::env::args().any(|a| a == "--demo-status") {
        match run_demo_status_cli() {
            Ok(msg) => {
                println!("{msg}");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("DEMO_STATUS_FAILED: {e}");
                std::process::exit(1);
            }
        }
    }
    if std::env::args().any(|a| a == "--demo-delete") {
        match run_demo_delete_cli() {
            Ok(msg) => {
                println!("{msg}");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("DEMO_DELETE_FAILED: {e}");
                std::process::exit(1);
            }
        }
    }
    if std::env::args().any(|a| a == "--demo-reseed") {
        match run_demo_reseed_cli() {
            Ok(msg) => {
                println!("{msg}");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("DEMO_RESEED_FAILED: {e}");
                std::process::exit(1);
            }
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // تحديد مسار قاعدة البيانات في مجلد بيانات التطبيق
            let app_data_dir = std::env::var_os("OBR_APP_DATA_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| app.path().app_data_dir().expect("فشل تحديد مجلد بيانات التطبيق"));
            std::fs::create_dir_all(&app_data_dir).expect("فشل إنشاء مجلد بيانات التطبيق");
            let db_path = app_data_dir.join("mawasib.db");

            let sqlite_db = SqliteDatabase::new(&db_path).expect("فشل فتح قاعدة البيانات");
            // جذور الخرائط المحلية: AppData\maps ثم مجلد resources المضمّن مع التثبيت
            {
                let mut map_roots = vec![app_data_dir.join("maps")];
                if let Ok(res_dir) = app.path().resource_dir() {
                    map_roots.push(res_dir.join("resources").join("maps"));
                    map_roots.push(res_dir.join("maps"));
                }
                map_tiles::set_runtime_roots(map_roots);
            }
            // كاش بلاطات الخريطة الذي يديره OBR: AppData\map-cache (حد 1.5GB + LRU)
            map_cache::set_runtime_root(&app_data_dir);
            // جذور صور العروض التسويقية للتدريب: نفس الجذور المستخدمة في مسار CLI
            {
                db::training_seed::set_demo_image_sources(demo_image_source_dirs(&app_data_dir));
            }
            // تأسيس مجلد الملفات المحلي
            let _ = db::files::FileStore::new(&app_data_dir);
            // ترحيل صور العقارات من Base64 داخل القاعدة إلى ملفات فعلية
            // (idempotent وآمن: عند فشل أي صف يبقى data URL الأصلي كما هو)
            match db::files::FileStore::new(&app_data_dir) {
                Ok(img_store) => match open_migrated_conn(&db_path) {
                    Ok(conn) => match db::images_migration::migrate_property_images_to_files(&conn, &img_store) {
                        Ok(stats) => {
                            if stats.images_migrated + stats.main_images_migrated > 0 {
                                eprintln!(
                                    "images migration: {} صورة + {} صورة رئيسية ({:.1} KB مكتوبة، {} أخطاء)",
                                    stats.images_migrated,
                                    stats.main_images_migrated,
                                    stats.bytes_written as f64 / 1024.0,
                                    stats.errors
                                );
                                // استعادة مساحة الصفحات المحررة بعد ترحيل الصور (مرة واحدة)
                                if let Err(e) = conn.execute_batch("VACUUM;") {
                                    eprintln!("images migration vacuum skipped: {e}");
                                }
                            }
                        }
                        Err(e) => eprintln!("images migration skipped: {e}"),
                    },
                    Err(e) => eprintln!("images migration skipped (conn): {e}"),
                },
                Err(e) => eprintln!("images migration skipped (store): {e}"),
            }
            // مفتاح JWT لكل تثبيت: يُولَّد مرة واحدة ويُحفظ في AppData ولا يتغير عند إعادة التشغيل
            let auth_secret = services::load_or_create_auth_secret(&app_data_dir)
                .expect("فشل تهيئة مفتاح التوثيق");
            services::init_auth_secret(auth_secret).expect("فشل تثبيت مفتاح التوثيق");
            app.manage(AppData {
                db: Mutex::new(AppState::new(Box::new(sqlite_db))),
                db_path,
                app_data_dir,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // المصادقة المحلية (بدون حساب OBR)
            auth_logout,
            auth_me,
            auth_ensure_local,
            // العروض
            properties_list,
            properties_get,
            properties_create,
            properties_update,
            properties_delete,
            properties_change_status,
            properties_add_update,
            properties_add_image,
            property_image_read,
            properties_images,
            properties_delete_image,
            properties_duplicate,
            // العملاء
            clients_list,
            clients_options,
            clients_get,
            clients_create,
            clients_update,
            clients_delete,
            clients_add_communication,
            clients_link_property,
            clients_unlink_property,
            // المؤشر
            market_list,
            market_stats,
            market_trends,
            market_create,
            market_update,
            market_bulk_create,
            market_delete,
            market_delete_many,
            market_delete_filtered,
            market_purge_audit,
            market_disclaimer,
            market_filter_cities,
            market_filter_districts,
            market_geo_summary,
            // الإعدادات
            settings_get,
            official_integrations_request,
            settings_update,
            // لوحة المعلومات
            dashboard_stats,
            dashboard_activity,
            dashboard_aggregates,
            // البروشرات
            brochures_templates,
            brochures_list,
            brochures_get,
            brochures_create,
            brochures_update,
            brochures_delete,
            // الحملات التسويقية
            campaigns_list,
            campaigns_stats,
            campaigns_get,
            campaigns_create,
            campaigns_update,
            campaigns_delete,
            campaigns_tracking_enable,
            campaigns_tracking_disable,
            campaigns_tracking_sync,
            campaigns_tracking_status,
            appointments_list,
            appointments_get,
            appointments_create,
            appointments_update,
            appointments_delete,
            finance_summary,
            finance_ledger,
            finance_receivables,
            finance_operations_list,
            finance_operations_get,
            finance_operations_create,
            finance_operations_update,
            finance_operations_delete,
            finance_invoices_list,
            finance_invoices_get,
            finance_invoices_create,
            finance_invoices_update,
            finance_invoices_delete,
            finance_commissions_list,
            finance_commissions_get,
            finance_commissions_create,
            finance_commissions_update,
            finance_commissions_delete,
            finance_expenses_list,
            finance_expenses_get,
            finance_expenses_create,
            finance_expenses_update,
            finance_expenses_delete,
            finance_payments_list,
            finance_payments_get,
            finance_payments_create,
            finance_payments_update,
            finance_payments_delete,
            finance_invoice_pdf,
            finance_record_pdf,
            finance_report_pdf,
            finance_open_pdf,
            contracts_list,
            contracts_summary,
            contracts_preview_pdf,
            contracts_portal_assist,
            contracts_save,
            contracts_delete,
            contracts_pdf_data,
            contracts_open_pdf,
            contracts_print_pdf,
            // التصدير
            export_all,
            export_properties,
            export_clients,
            export_market,
            system_info,
            backup_create,
            backup_list,
            backup_restore,
            backup_delete,
            import_data_json,
            demo_status,
            demo_delete,
            demo_reseed,
            demo_prepare,
            open_external_url,
            open_whatsapp_chat,
            prepare_offer_share_pack,
            copy_share_files_to_clipboard,
            open_share_folder,
            export_design_document,
            map_assets_status,
            map_read_range,
            map_cache_fetch,
            map_cache_status,
            map_cache_clear,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn run_demo_prepare_cli() -> Result<String, String> {
    let app_data_dir = dirs_next_app_data()?;
    crate::db::training_seed::set_demo_image_sources(demo_image_source_dirs(&app_data_dir));
    std::fs::create_dir_all(&app_data_dir).map_err(|e| e.to_string())?;
    let db_path = app_data_dir.join("mawasib.db");
    let mut conn = open_migrated_conn(&db_path)?;

    let user_id: String = conn
        .query_row(
            "SELECT id FROM users ORDER BY created_at ASC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .map_err(|_| "لا يوجد مستخدم محلي — شغّل التطبيق مرة واحدة أولاً".to_string())?;

    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let folder = format!("pre-demo-reset-{stamp}");
    let store = db::files::FileStore::new(&app_data_dir)?;
    let pkg = db::backup::create_backup_package_named(
        &app_data_dir,
        &db_path,
        &conn,
        &store,
        Some(&user_id),
        &folder,
        "pre-demo-reset CLI: backup before operational purge + demo seed",
        0,
        &stamp,
    )?;

    let db_ok = pkg.path.join("mawasib.db").is_file();
    let files_ok = pkg.path.join("files").exists();
    let manifest_ok = pkg.path.join("manifest.json").is_file();
    if !db_ok || !manifest_ok {
        return Err("فشل التحقق من حزمة النسخ الاحتياطي".into());
    }

    let (removed, seeded) = db::demo::prepare_demo_environment(&mut conn, &user_id, &app_data_dir)?;
    let status = db::demo::demo_status(&conn, db::demo::DEMO_BATCH_ID)?;

    let out = serde_json::json!({
        "backup": {
            "path": pkg.path.to_string_lossy(),
            "schema_version": pkg.schema_version,
            "db_present": db_ok,
            "files_dir_present": files_ok,
            "manifest_present": manifest_ok
        },
        "removed": db::demo::counts_to_json(&removed),
        "seeded": db::demo::counts_to_json(&seeded),
        "status": status
    });
    let report_path = app_data_dir.join("demo-prepare-last.json");
    std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&out).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(format!(
        "OK backup={} report={}",
        pkg.path.display(),
        report_path.display()
    ))
}

/// يشغّل Migrations على قاعدة محددة ثم يقرأ فعلياً من الجداول المتأثرة.
/// الهدف إثبات أن الترقية والقراءة تعملان على بيانات قائمة، لا على قاعدة جديدة فقط.
fn run_verify_db_cli(path: &str) -> Result<String, String> {
    let db_path = std::path::PathBuf::from(path);
    if !db_path.is_file() {
        return Err(format!("القاعدة غير موجودة: {path}"));
    }
    use db::Database as _;
    let db = db::SqliteDatabase::new(&db_path)?;
    let version = db.schema_version()?;
    let integrity = db.integrity_ok()?;

    let user = db.get_first_user()?.ok_or("لا يوجد مستخدم في هذه القاعدة")?;
    // قراءة الإعدادات هي ما كان يفشل سابقاً بسبب ترتيب الأعمدة (fal_license_type NULL).
    let settings = db.get_settings(&user.id)?;
    let (clients, clients_total) = db.list_clients(&user.id, "", "", "", "", false, 5, 0)?;
    let (properties, props_total) = db.list_properties(&user.id, "", "", "", "", "", 5, 0)?;
    let brochures = db.list_brochures(&user.id, 0, 0)?;
    let mut role_samples = Vec::new();
    for c in &clients {
        role_samples.push(serde_json::json!({ "client": c.name, "roles": db.list_client_roles(&c.id)? }));
    }

    let out = serde_json::json!({
        "db_path": db_path.display().to_string(),
        "schema_version": version,
        "integrity_ok": integrity,
        "user_id": user.id,
        "settings_read_ok": true,
        "settings_fal_license_number": settings.as_ref().and_then(|s| s.fal_license_number.clone()),
        "settings_fal_license_type": settings.as_ref().and_then(|s| s.fal_license_type.clone()),
        "settings_updated_at": settings.as_ref().map(|s| s.updated_at.clone()),
        "clients_total": clients_total,
        "properties_total": props_total,
        "brochures_read_ok": true,
        "brochures_count": brochures.len(),
        "role_samples": role_samples,
    });
    println!("{}", serde_json::to_string_pretty(&out).map_err(|e| e.to_string())?);
    Ok("VERIFY_DB_OK".to_string())
}

fn dirs_next_app_data() -> Result<std::path::PathBuf, String> {
    // الأولوية لمجلد البيانات المحمّل (OBR_APP_DATA_DIR) ثم fallback %APPDATA%\com.mawasib.obr
    if let Some(portable) = std::env::var_os("OBR_APP_DATA_DIR") {
        if !portable.is_empty() {
            return Ok(std::path::PathBuf::from(portable));
        }
    }
    // نفس مجلد Tauri identifier com.mawasib.obr
    let base = std::env::var("APPDATA").map_err(|_| "APPDATA غير معرّف".to_string())?;
    Ok(std::path::PathBuf::from(base).join("com.mawasib.obr"))
}

fn write_cli_report(name: &str, value: serde_json::Value) -> Result<String, String> {
    let app_data_dir = dirs_next_app_data()?;
    let report_path = app_data_dir.join(name);
    std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(format!("OK report={}", report_path.display()))
}

fn run_demo_status_cli() -> Result<String, String> {
    let app_data_dir = dirs_next_app_data()?;
    let db_path = app_data_dir.join("mawasib.db");
    let conn = open_migrated_conn(&db_path)?;
    let status = db::demo::demo_status(&conn, db::demo::DEMO_BATCH_ID)?;
    write_cli_report("demo-status-last.json", status.clone())?;
    Ok(status.to_string())
}

fn run_demo_delete_cli() -> Result<String, String> {
    let app_data_dir = dirs_next_app_data()?;
    let db_path = app_data_dir.join("mawasib.db");
    let mut conn = open_migrated_conn(&db_path)?;
    let before = db::demo::demo_status(&conn, db::demo::DEMO_BATCH_ID)?;
    let deleted = db::demo::delete_demo_batch(&mut conn, db::demo::DEMO_BATCH_ID, &app_data_dir)?;
    let after = db::demo::demo_status(&conn, db::demo::DEMO_BATCH_ID)?;
    let out = serde_json::json!({
        "before": before,
        "deleted": db::demo::counts_to_json(&deleted),
        "after": after
    });
    write_cli_report("demo-delete-last.json", out.clone())?;
    Ok(out.to_string())
}

/// جذور صور العروض التسويقية للتدريب — تُستخدم في مسار الواجهة ومسار CLI معًا
/// (resources\demo بجوار الـEXE في التثبيت، AppData\demo، ومجلد المصدر في التطوير)
fn demo_image_source_dirs(app_data_dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut dirs = vec![app_data_dir.join("demo")];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            dirs.push(exe_dir.join("resources").join("demo"));
            dirs.push(exe_dir.join("demo"));
        }
    }
    dirs.push(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("demo"),
    );
    dirs
}

fn run_demo_reseed_cli() -> Result<String, String> {
    let app_data_dir = dirs_next_app_data()?;
    crate::db::training_seed::set_demo_image_sources(demo_image_source_dirs(&app_data_dir));
    let db_path = app_data_dir.join("mawasib.db");
    let mut conn = open_migrated_conn(&db_path)?;
    let user_id: String = conn
        .query_row(
            "SELECT id FROM users ORDER BY created_at ASC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .map_err(|_| "لا يوجد مستخدم محلي".to_string())?;
    let result = db::demo::reseed_demo(&mut conn, &user_id, &app_data_dir)?;
    let status = db::demo::demo_status(&conn, db::demo::DEMO_BATCH_ID)?;
    let out = serde_json::json!({ "result": result, "status": status });
    write_cli_report("demo-reseed-last.json", out.clone())?;
    Ok(out.to_string())
}

#[tauri::command]
fn contracts_list(app: State<AppData>, token: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    let conn = open_migrated_conn(&app.db_path)?;
    crate::contracts::list(&conn, &user_id)
}

#[tauri::command]
fn contracts_summary(app: State<AppData>, token: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    let conn = open_migrated_conn(&app.db_path)?;
    crate::contracts::summary(&conn, &user_id)
}

#[tauri::command]
fn contracts_preview_pdf(app: State<AppData>, token: String, data_url: String) -> Result<serde_json::Value, String> {
    let _user_id = get_user_id(&app, &token)?;
    crate::contracts::preview_pdf(&data_url)
}

#[tauri::command]
fn contracts_portal_assist(app: State<AppData>, token: String, data: serde_json::Value) -> Result<serde_json::Value, String> {
    let _user_id = get_user_id(&app, &token)?;
    crate::contracts::prepare_portal_assist(&data)
}

#[tauri::command]
fn contracts_save(app: State<AppData>, token: String, data: serde_json::Value) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    let conn = open_migrated_conn(&app.db_path)?;
    crate::contracts::save(&conn, &app.app_data_dir, &user_id, &data)
}

#[tauri::command]
fn contracts_pdf_data(app: State<AppData>, token: String, id: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    let conn = open_migrated_conn(&app.db_path)?;
    crate::contracts::pdf_data(&conn, &app.app_data_dir, &user_id, &id)
}

#[tauri::command]
fn contracts_open_pdf(app: State<AppData>, token: String, id: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    let conn = open_migrated_conn(&app.db_path)?;
    crate::contracts::open_pdf(&conn, &app.app_data_dir, &user_id, &id)
}

#[tauri::command]
fn contracts_print_pdf(app: State<AppData>, token: String, data: serde_json::Value, open: Option<bool>) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    let settings = with_db(&app, |db| db.get_settings(&user_id))?;
    let mut data = data;
    let context = if let Some(id) = data.get("id").and_then(|v|v.as_str()).filter(|s|!s.is_empty()) {
        let conn = open_migrated_conn(&app.db_path)?;
        let records = crate::contracts::list(&conn, &user_id)?;
        let saved = records["items"].as_array().unwrap().iter().find(|r| r["id"] == id).ok_or("العقد غير موجود")?;
        crate::country::record_context(saved)?
    } else { let conn = open_migrated_conn(&app.db_path)?; crate::country::new_context(&conn, "contracts", &user_id, &data, &settings)? };
    data.as_object_mut().ok_or("بيانات عقد غير صالحة")?.extend(serde_json::to_value(&context).map_err(|e|e.to_string())?.as_object().unwrap().clone());
    let settings = context.print_settings(&settings);
    let html = crate::contracts::print_html(&data, &settings);
    let stem = data.get("contract_number").and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty()).unwrap_or("contract");
    let written = crate::finance_pdf::generate_pdf(&app.app_data_dir, "contract", stem, &html)?;
    let opened = open.unwrap_or(true);
    if opened {
        crate::finance_pdf::open_pdf_file(std::path::Path::new(&written.pdf_path))?;
    }
    Ok(serde_json::json!({ "pdf_path": written.pdf_path, "opened": opened }))
}


#[tauri::command]
fn contracts_delete(app: State<AppData>, token: String, id: String) -> Result<serde_json::Value, String> {
    let user_id = get_user_id(&app, &token)?;
    let conn = open_migrated_conn(&app.db_path)?;
    crate::contracts::delete(&conn, &app.app_data_dir, &user_id, &id)
}
