use crate::services::property_to_shareable_json;
use crate::db::Database;
use base64::Engine;
use serde_json::Value;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::Duration;

#[derive(Debug, Clone, serde::Serialize)]
pub struct OfferSharePack {
    pub folder: String,
    pub pdf_path: Option<String>,
    pub image_paths: Vec<String>,
    pub files: Vec<String>,
    pub pdf_ready: bool,
    pub image_count: usize,
}

/// جذر حزم المشاركة داخل بيانات التطبيق.
pub fn share_packs_root(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("share-packs")
}

fn canonicalize_path(path: &Path) -> Result<PathBuf, String> {
    path.canonicalize()
        .map_err(|e| format!("تعذر التحقق من المسار: {e}"))
}

fn reject_suspicious_components(path: &Path) -> Result<(), String> {
    for c in path.components() {
        match c {
            Component::ParentDir => {
                return Err("مسار غير مسموح (path traversal)".into());
            }
            Component::Normal(s) => {
                let t = s.to_string_lossy();
                if t.contains('\0') {
                    return Err("مسار غير مسموح".into());
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn is_under_root(path: &Path, root: &Path) -> bool {
    path.starts_with(root)
}

fn existing_canonical_roots(roots: &[PathBuf]) -> Vec<PathBuf> {
    roots
        .iter()
        .filter(|r| r.exists())
        .filter_map(|r| canonicalize_path(r).ok())
        .collect()
}

/// مسار موجود يجب أن يقع تحت أحد الجذور بعد canonicalize (يمنع symlink/junction escape).
pub fn resolve_existing_under_roots(path: &str, roots: &[PathBuf]) -> Result<PathBuf, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("مسار غير صالح".into());
    }
    let input = PathBuf::from(trimmed);
    reject_suspicious_components(&input)?;
    if !input.exists() {
        return Err("المسار غير موجود".into());
    }
    let resolved = canonicalize_path(&input)?;
    let roots = existing_canonical_roots(roots);
    if roots.is_empty() {
        return Err("لا يوجد نطاق مسموح للمسارات".into());
    }
    if !roots.iter().any(|root| is_under_root(&resolved, root)) {
        return Err("المسار خارج النطاق المسموح".into());
    }
    Ok(resolved)
}

/// مسار كتابة قد لا يكون الملف موجودًا بعد؛ يُتحقق من الأب ثم الاسم البسيط.
pub fn resolve_writable_under_roots(path: &str, roots: &[PathBuf]) -> Result<PathBuf, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("مسار غير صالح".into());
    }
    let input = PathBuf::from(trimmed);
    reject_suspicious_components(&input)?;
    let file_name = input
        .file_name()
        .ok_or_else(|| "مسار التصدير غير صالح".to_string())?;
    let parent = input
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| "مسار التصدير غير صالح".to_string())?;

    let roots_c = existing_canonical_roots(roots);
    if roots_c.is_empty() {
        return Err("لا يوجد نطاق مسموح للمسارات".into());
    }

    if parent.exists() {
        let parent_c = canonicalize_path(parent)?;
        if !roots_c.iter().any(|root| is_under_root(&parent_c, root)) {
            return Err("المسار خارج النطاق المسموح".into());
        }
        return Ok(parent_c.join(file_name));
    }

    // الأب غير موجود: اصعد لأقرب سلف موجود ثم أعد بناء الأجزاء بدون `..`
    let mut ancestor = parent.to_path_buf();
    let mut missing: Vec<std::ffi::OsString> = Vec::new();
    while !ancestor.exists() {
        let name = ancestor
            .file_name()
            .ok_or_else(|| "مسار التصدير خارج النطاق المسموح".to_string())?
            .to_os_string();
        missing.push(name);
        match ancestor.parent() {
            Some(p) if !p.as_os_str().is_empty() => ancestor = p.to_path_buf(),
            _ => return Err("مسار التصدير خارج النطاق المسموح".into()),
        }
    }
    reject_suspicious_components(&ancestor)?;
    let ancestor_c = canonicalize_path(&ancestor)?;
    if !roots_c.iter().any(|root| is_under_root(&ancestor_c, root)) {
        return Err("المسار خارج النطاق المسموح".into());
    }
    missing.reverse();
    let mut built = ancestor_c;
    for part in missing {
        reject_suspicious_components(Path::new(&part))?;
        built.push(part);
    }
    built.push(file_name);
    Ok(built)
}

pub fn ensure_share_packs_root(app_data_dir: &Path) -> Result<PathBuf, String> {
    let root = share_packs_root(app_data_dir);
    std::fs::create_dir_all(&root).map_err(|e| format!("تعذر تجهيز مجلد المشاركة: {e}"))?;
    canonicalize_path(&root)
}

pub fn resolve_share_pack_file(app_data_dir: &Path, path: &str) -> Result<PathBuf, String> {
    let root = ensure_share_packs_root(app_data_dir)?;
    let resolved = resolve_existing_under_roots(path, std::slice::from_ref(&root))?;
    if !resolved.is_file() {
        return Err("أحد ملفات المشاركة غير موجود".into());
    }
    Ok(resolved)
}

pub fn resolve_share_pack_dir(app_data_dir: &Path, path: &str) -> Result<PathBuf, String> {
    let root = ensure_share_packs_root(app_data_dir)?;
    let resolved = resolve_existing_under_roots(path, std::slice::from_ref(&root))?;
    if !resolved.is_dir() {
        return Err("مجلد المشاركة غير موجود".into());
    }
    Ok(resolved)
}

/// جذور التصدير المسموحة: بيانات التطبيق + Documents/Downloads/Desktop للمستخدم.
pub fn export_allowed_roots(app_data_dir: &Path) -> Vec<PathBuf> {
    let mut roots = vec![app_data_dir.to_path_buf()];
    if let Ok(profile) = std::env::var("USERPROFILE") {
        let home = PathBuf::from(profile);
        for name in ["Documents", "Downloads", "Desktop"] {
            roots.push(home.join(name));
        }
        let onedrive_docs = home.join("OneDrive").join("Documents");
        if onedrive_docs.is_dir() {
            roots.push(onedrive_docs);
        }
    }
    roots
}

pub fn resolve_export_destination(app_data_dir: &Path, dest_path: &str) -> Result<PathBuf, String> {
    resolve_writable_under_roots(dest_path, &export_allowed_roots(app_data_dir))
}

pub fn prepare_offer_share_pack(
    db: &dyn Database,
    user_id: &str,
    property_id: &str,
    app_data_dir: &Path,
) -> Result<OfferSharePack, String> {
    let property = db
        .get_property(property_id, user_id)?
        .ok_or_else(|| "العرض غير موجود".to_string())?;
    if property.user_id != user_id {
        return Err("العرض غير موجود".into());
    }
    let shareable = property_to_shareable_json(&property);
    let images = db.list_property_images(property_id)?;
    // حقل فارغ = إخفاء؛ لا نص افتراضي في مستندات العميل.
    let office = db
        .get_settings(user_id)?
        .and_then(|s| s.office_name)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let folder = app_data_dir
        .join("share-packs")
        .join(format!("{}-{}", safe_stem(&json_str(&shareable, "name")), uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&folder).map_err(|e| format!("تعذر إنشاء مجلد المشاركة: {e}"))?;

    let mut image_paths = Vec::new();
    for (i, img) in images.iter().enumerate().take(4) {
        if let Ok((ext, bytes)) = decode_data_url(&img.image_url) {
            let path = folder.join(format!("صورة-{}.{}", i + 1, ext));
            std::fs::write(&path, bytes).map_err(|e| format!("تعذر حفظ الصورة: {e}"))?;
            image_paths.push(path);
        }
    }

    let html = build_offer_html_localized(&shareable, office.as_deref(), &image_paths, &db.get_settings(user_id)?);
    let html_path = folder.join("offer.html");
    std::fs::write(&html_path, html).map_err(|e| format!("تعذر كتابة ملف العرض: {e}"))?;

    let pdf_path = folder.join("offer.pdf");
    let pdf_ready = print_html_to_pdf(&html_path, &pdf_path).is_ok() && pdf_path.is_file();

    let mut files = Vec::new();
    if pdf_ready {
        files.push(pdf_path.clone());
    }
    files.extend(image_paths.iter().cloned());

    Ok(OfferSharePack {
        folder: folder.to_string_lossy().into_owned(),
        pdf_path: if pdf_ready {
            Some(pdf_path.to_string_lossy().into_owned())
        } else {
            None
        },
        image_count: image_paths.len(),
        image_paths: image_paths.iter().map(|p| p.to_string_lossy().into_owned()).collect(),
        files: files.iter().map(|p| p.to_string_lossy().into_owned()).collect(),
        pdf_ready,
    })
}

pub fn copy_files_to_clipboard(paths: &[String]) -> Result<(), String> {
    if paths.is_empty() {
        return Err("لا توجد ملفات لنسخها".into());
    }
    for path in paths {
        if !Path::new(path).is_file() {
            return Err("أحد ملفات المشاركة غير موجود".into());
        }
    }
    let script = std::env::temp_dir().join(format!("obr-clip-{}.ps1", std::process::id()));
    let mut body = String::from("Add-Type -AssemblyName System.Windows.Forms\r\n$c = New-Object System.Collections.Specialized.StringCollection\r\n");
    for path in paths {
        body.push_str(&format!("[void]$c.Add('{}')\r\n", path.replace('\'', "''")));
    }
    body.push_str("[System.Windows.Forms.Clipboard]::SetFileDropList($c)\r\n");
    std::fs::write(&script, body).map_err(|e| format!("تعذر تجهيز الحافظة: {e}"))?;
    let status = Command::new("powershell")
        .args([
            "-STA",
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &script.to_string_lossy(),
        ])
        .status()
        .map_err(|_| "تعذر نسخ الملفات إلى الحافظة".to_string())?;
    let _ = std::fs::remove_file(&script);
    if !status.success() {
        return Err("تعذر نسخ الملفات إلى الحافظة".into());
    }
    Ok(())
}

pub fn open_folder(path: &str) -> Result<(), String> {
    let folder = Path::new(path);
    if !folder.is_dir() {
        return Err("مجلد المشاركة غير موجود".into());
    }
    Command::new("explorer")
        .arg(folder)
        .spawn()
        .map_err(|_| "تعذر فتح مجلد الملفات".to_string())?;
    Ok(())
}

pub fn decode_data_url(raw: &str) -> Result<(String, Vec<u8>), String> {
    let raw = raw.trim();
    let (meta, data) = raw
        .strip_prefix("data:")
        .and_then(|rest| rest.split_once(','))
        .ok_or_else(|| "صورة العرض ليست ملفًا صالحًا".to_string())?;
    if !meta.contains("base64") {
        return Err("صيغة الصورة غير مدعومة".into());
    }
    let ext = if meta.contains("png") {
        "png"
    } else if meta.contains("webp") {
        "webp"
    } else {
        "jpg"
    };
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data.trim())
        .map_err(|_| "تعذر قراءة بيانات الصورة".to_string())?;
    if bytes.is_empty() {
        return Err("الصورة فارغة".into());
    }
    Ok((ext.to_string(), bytes))
}

pub fn build_offer_html(property: &Value, office: Option<&str>, image_paths: &[PathBuf]) -> String {
    build_offer_html_localized(property, office, image_paths, &None)
}

pub fn build_offer_html_localized(property: &Value, office: Option<&str>, image_paths: &[PathBuf], settings: &Option<crate::db::Settings>) -> String {
    use crate::document_locale::{text as tr, language};
    let name = esc(&json_str(property, "name"));
    // خانة فارغة = سطر اسم المكتب فارغ (يُخفى بـ CSS) — لا نص افتراضي.
    let office = office
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(esc)
        .unwrap_or_default();
    let city = esc(&json_str(property, "city"));
    let district = esc(&json_str(property, "district"));
    let area = json_str(property, "area");
    let raw_price = json_str(property, "price");
    let price = if raw_price.is_empty() { raw_price } else { format!("{} {}", raw_price, json_str(property, "currency")) };
    let ptype = esc(&json_str(property, "property_type"));
    let purpose_raw = json_str(property, "purpose");
    let purpose = esc(match purpose_raw.as_str() { "sale" => tr("design.sale", settings), "rent" => tr("design.rent", settings), "investment" => tr("design.investment", settings), _ => &purpose_raw });
    let desc = esc(&json_str(property, "description"));
    let fal = esc(&json_str(property, "fal_ad_license"));
    let fields = [
        (tr("design.ui.129", settings), city), (tr("design.ui.130", settings), district), (tr("design.ui.132", settings), purpose),
        (tr("doc.kind", settings), ptype), (tr("design.ui.128", settings), esc(&area)), (tr("design.ui.126", settings), esc(&price)),
        (tr("design.ui.138", settings), fal),
    ].into_iter().filter(|(_, value)| !value.trim().is_empty()).map(|(label, value)| {
        format!("<div><div class=\"label\">{}</div><div class=\"value\">{}</div></div>", esc(label), value)
    }).collect::<String>();
    let mut imgs = String::new();
    for path in image_paths {
        let src = file_url(path);
        imgs.push_str(&format!(
            r#"<img src="{src}" alt="" style="width:100%;max-height:280px;object-fit:contain;background:#1a2746;border-radius:10px;margin:0 0 12px" />"#
        ));
    }
    format!(
        r#"<!DOCTYPE html>
<html lang="{lang}" dir="{dir}">
<head>
<meta charset="utf-8" />
<title>{name}</title>
<style>
  body {{ font-family: "Segoe UI", Tahoma, Arial, sans-serif; margin: 28px; color: #1a2746; }}
  h1 {{ margin: 0 0 6px; font-size: 26px; }}
  .office {{ color: #d97706; font-weight: 700; margin-bottom: 18px; }}
  .office:empty {{ display: none; }}
  .grid {{ display: grid; grid-template-columns: 1fr 1fr; gap: 8px 20px; margin: 16px 0 22px; }}
  .label {{ color: #6f7380; font-size: 12px; }}
  .value {{ font-weight: 700; font-size: 15px; }}
  p {{ line-height: 1.7; }}
</style>
</head>
<body>
  <div class="office">{office}</div>
  <h1>{name}</h1>
  {fields_html}
  {imgs}
  {desc_html}
</body>
</html>"#,
        lang = language(settings),
        dir = if language(settings) == "en" { "ltr" } else { "rtl" },
        fields_html = if fields.is_empty() { String::new() } else { format!("<div class=\"grid\">{fields}</div>") },
        desc_html = if desc.is_empty() { String::new() } else { format!("<p>{desc}</p>") },
    )
}

pub fn html_contains_internal_leak(html: &str) -> bool {
    ["owner_phone", "owner_name", "internal_price", "internal_notes", "commission_agreement", "owner_client_id"]
        .iter()
        .any(|key| html.contains(key))
}

pub fn print_html_to_pdf(html_path: &Path, pdf_path: &Path) -> Result<(), String> {
    let browser = find_browser().ok_or_else(|| "تعذر إنشاء PDF — ثبّت Microsoft Edge أو Chrome".to_string())?;
    let html_url = file_url(html_path);
    // ملف تعريف مؤقت ومعزول لكل عملية طباعة: يمنع تصادم نسخ المتصفح المتوازية على
    // profile المستخدم (وبدونه يفشل التصدير عند وجود Edge/Chrome مفتوح أو تصدير متزامن).
    let profile_dir = std::env::temp_dir().join(format!("obr-pdf-profile-{}", uuid::Uuid::new_v4()));
    let status = Command::new(&browser)
        .args([
            "--headless=new",
            "--disable-gpu",
            "--no-first-run",
            "--no-pdf-header-footer",
            &format!("--user-data-dir={}", profile_dir.display()),
            &format!("--print-to-pdf={}", pdf_path.display()),
            &html_url,
        ])
        .status()
        .map_err(|_| "تعذر إنشاء ملف PDF".to_string())?;
    // العملية المطْلَقة (Chromium/Edge) قد تخرج فورًا بنجاح بينما الكتابة الفعلية تُنفَّذ
    // في عملية أخرى وتستغرق ثوانٍ — قياس فعلي (5 عمليات متوازية): الملف ظهر بعد ≤5.5s من
    // خروج العملية. لذلك ننتظر بحد أقصى 20s مع فحص كل 250ms بدل نافذة 3s التي كانت تسبب
    // فشلًا متقطعًا. (المسار السريع لا يتغير: يخرج فور ظهور الملف.)
    let mut written = false;
    for _ in 0..80 {
        if pdf_path.is_file() && std::fs::metadata(pdf_path).map(|m| m.len() > 800).unwrap_or(false) {
            written = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    // تنظيف أفضل-جهد لمجلد التعريف المؤقت (قد يُبقي المتصفح مقابض لحظات بعد خروجه).
    for _ in 0..5 {
        if std::fs::remove_dir_all(&profile_dir).is_ok() {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    if !status.success() || !written {
        return Err("تعذر إنشاء ملف PDF".into());
    }
    Ok(())
}

/// لقطة شاشة لصفحة HTML عبر المتصفح المحلي — لتصدير صورة العرض.
pub fn print_html_to_png(
    html_path: &Path,
    png_path: &Path,
    width_px: u32,
    height_px: u32,
) -> Result<(), String> {
    let browser = find_browser().ok_or_else(|| "تعذر إنشاء الصورة — ثبّت Microsoft Edge أو Chrome".to_string())?;
    let html_url = file_url(html_path);
    let w = width_px.clamp(320, 4096);
    let h = height_px.clamp(320, 8192);
    let status = Command::new(&browser)
        .args([
            "--headless=new",
            "--disable-gpu",
            "--no-first-run",
            "--hide-scrollbars",
            "--force-device-scale-factor=1",
            "--default-background-color=FFFFFFFF",
            &format!("--window-size={w},{h}"),
            &format!("--screenshot={}", png_path.display()),
            &html_url,
        ])
        .status()
        .map_err(|_| "تعذر إنشاء ملف الصورة".to_string())?;
    if !status.success() {
        return Err("تعذر إنشاء ملف الصورة".into());
    }
    for _ in 0..20 {
        if png_path.is_file() && std::fs::metadata(png_path).map(|m| m.len() > 400).unwrap_or(false) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    Err("تعذر إنشاء ملف الصورة".into())
}

/// يكتب HTML مؤقتًا ثم يصدّر PDF أو PNG إلى المسار الذي اختاره المستخدم.
pub fn export_html_document(
    html: &str,
    format: &str,
    dest_path: &Path,
    width_px: u32,
    height_px: u32,
) -> Result<(), String> {
    if html.trim().is_empty() {
        return Err("لا يوجد محتوى للتصدير".into());
    }
    if dest_path.as_os_str().is_empty() {
        return Err("مسار التصدير غير صالح".into());
    }
    let expected_ext = match format {
        "pdf" => "pdf",
        "png" => "png",
        "doc" => "doc",
        "html" => "html",
        other => return Err(format!("صيغة تصدير غير مدعومة: {other}")),
    };
    let actual_ext = dest_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if actual_ext != expected_ext {
        return Err(format!(
            "امتداد الملف يجب أن يكون .{expected_ext} لصيغة {format}"
        ));
    }
    if dest_path.exists() {
        return Err("الملف موجود مسبقًا — اختر اسمًا آخر دون استبدال صامت".into());
    }
    if let Some(parent) = dest_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| format!("تعذر تجهيز مجلد التصدير: {e}"))?;
        }
    }

    match format {
        "doc" | "html" => {
            std::fs::write(dest_path, html).map_err(|e| format!("تعذر حفظ الملف: {e}"))?;
            Ok(())
        }
        "pdf" => {
            let tmp = std::env::temp_dir().join(format!("obr-export-{}.html", uuid::Uuid::new_v4()));
            std::fs::write(&tmp, html).map_err(|e| format!("تعذر تجهيز ملف مؤقت: {e}"))?;
            let result = print_html_to_pdf(&tmp, dest_path);
            let _ = std::fs::remove_file(&tmp);
            result
        }
        "png" => {
            let tmp = std::env::temp_dir().join(format!("obr-export-{}.html", uuid::Uuid::new_v4()));
            std::fs::write(&tmp, html).map_err(|e| format!("تعذر تجهيز ملف مؤقت: {e}"))?;
            let result = print_html_to_png(&tmp, dest_path, width_px, height_px);
            let _ = std::fs::remove_file(&tmp);
            result
        }
        other => Err(format!("صيغة تصدير غير مدعومة: {other}")),
    }
}

fn find_browser() -> Option<PathBuf> {
    let local = std::env::var("PROGRAMFILES").unwrap_or_default();
    let local_x86 = std::env::var("PROGRAMFILES(X86)").unwrap_or_default();
    let candidates = [
        format!("{local}\\Microsoft\\Edge\\Application\\msedge.exe"),
        format!("{local_x86}\\Microsoft\\Edge\\Application\\msedge.exe"),
        format!("{local}\\Google\\Chrome\\Application\\chrome.exe"),
        format!("{local_x86}\\Google\\Chrome\\Application\\chrome.exe"),
    ];
    candidates.into_iter().map(PathBuf::from).find(|p| p.is_file())
}

fn json_str(value: &Value, key: &str) -> String {
    match value.get(key) {
        Some(Value::Null) | None => String::new(),
        Some(Value::String(s)) => {
            if s.trim().is_empty() {
                String::new()
            } else {
                s.clone()
            }
        }
        Some(other) => other.to_string().trim_matches('"').to_string(),
    }
}

fn purpose_ar(value: &str) -> String {
    match value {
        "sale" => "بيع".into(),
        "rent" => "إيجار".into(),
        "investment" => "استثمار".into(),
        other => other.to_string(),
    }
}

fn esc(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn safe_stem(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .take(24)
        .collect();
    if cleaned.trim_matches('-').is_empty() {
        "offer".into()
    } else {
        cleaned
    }
}

fn file_url(path: &Path) -> String {
    let raw = path.to_string_lossy().replace('\\', "/");
    let prefixed = if raw.starts_with('/') {
        raw
    } else {
        format!("/{raw}")
    };
    format!("file://{}", prefixed.replace(' ', "%20"))
}
