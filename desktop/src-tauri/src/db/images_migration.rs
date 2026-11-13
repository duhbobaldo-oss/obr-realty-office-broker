//! ترحيل صور العقارات من Base64/data URL داخل SQLite إلى ملفات فعلية
//! داخل `files\properties\` مع الاحتفاظ بـ relative_path فقط في قاعدة البيانات.
//!
//! قواعد الأمان:
//! - لا يُحدَّث صف في قاعدة البيانات إلا بعد كتابة الملف والتحقق من وجوده وحجمه.
//! - عند فشل أي صف: يُترك كما هو (data URL الأصلي لا يُحذف) ويُحسب ضمن errors.
//! - الترحيل idempotent: الصفوف التي ليست data: تُتجاهل.

use crate::db::files::FileStore;
use rusqlite::Connection;
use std::collections::HashMap;

#[derive(Debug, Default, serde::Serialize)]
pub struct ImagesMigrationStats {
    pub images_migrated: u64,
    pub main_images_migrated: u64,
    pub errors: u64,
    pub bytes_written: u64,
}

/// فك data URL إلى (mime, bytes). يعيد Err للقيم غير الصالحة.
pub fn decode_data_url(data_url: &str) -> Result<(String, Vec<u8>), String> {
    let rest = data_url
        .strip_prefix("data:")
        .ok_or_else(|| "ليست data URL".to_string())?;
    let (meta, payload) = rest
        .split_once(',')
        .ok_or_else(|| "data URL بلا حمولة".to_string())?;
    let mime = meta
        .strip_suffix(";base64")
        .unwrap_or(meta)
        .trim()
        .to_string();
    let mime = if mime.is_empty() {
        "image/jpeg".to_string()
    } else {
        mime
    };
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(payload.trim())
        .map_err(|e| format!("base64 غير صالح: {e}"))?;
    if bytes.is_empty() {
        return Err("حمولة الصورة فارغة".to_string());
    }
    Ok((mime, bytes))
}

fn mime_to_ext(mime: &str) -> &'static str {
    match mime {
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => "jpg",
    }
}

/// تخزين data URL كملف فعلي داخل files\properties وإعادة (المسار النسبي، اسم التخزين).
/// يُستخدم عند رفع صور جديدة وعند الترحيل.
pub fn store_data_url_as_file(
    store: &FileStore,
    data_url: &str,
) -> Result<(String, String), String> {
    let (mime, bytes) = decode_data_url(data_url)?;
    let ext = mime_to_ext(&mime);
    store.store_bytes(&format!("img.{ext}"), &bytes, "properties")
}

/// ترحيل صورة واحدة: فك → تخزين → التحقق من الملف. يعيد المسار النسبي.
fn migrate_one(data_url: &str, store: &FileStore) -> Result<(String, u64), String> {
    let (relative, _name) = store_data_url_as_file(store, data_url)?;
    let abs = store.absolute_from_relative(&relative);
    let meta = std::fs::metadata(&abs).map_err(|e| format!("الملف المخزن غير موجود: {e}"))?;
    if meta.len() == 0 {
        return Err("الملف المخزن فارغ".to_string());
    }
    Ok((relative, meta.len()))
}

/// تنفيذ الترحيل على قاعدة بيانات مفتوحة. يُستدعى عند الإقلاع بعد إنشاء FileStore.
pub fn migrate_property_images_to_files(
    conn: &Connection,
    store: &FileStore,
) -> Result<ImagesMigrationStats, String> {
    let mut stats = ImagesMigrationStats::default();
    let mut url_map: HashMap<String, String> = HashMap::new();

    // 1) صفوف property_images التي ما تزال data URLs
    let mut stmt = conn
        .prepare("SELECT id, image_url FROM property_images WHERE image_url LIKE 'data:%'")
        .map_err(|e| e.to_string())?;
    let rows: Vec<(String, String)> = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    drop(stmt);

    for (id, data_url) in rows {
        match migrate_one(&data_url, store) {
            Ok((relative, bytes)) => {
                let updated = conn
                    .execute(
                        "UPDATE property_images SET image_url = ?1 WHERE id = ?2 AND image_url = ?3",
                        rusqlite::params![relative, id, data_url],
                    )
                    .map_err(|e| e.to_string())?;
                if updated == 1 {
                    stats.images_migrated += 1;
                    stats.bytes_written += bytes;
                    url_map.insert(data_url, relative);
                } else {
                    stats.errors += 1;
                }
            }
            Err(_) => {
                // الصف يبقى data URL — لا يُحذف الأصل
                stats.errors += 1;
            }
        }
    }

    // 2) main_image التي ما تزال data URL
    let mut stmt = conn
        .prepare("SELECT id, main_image FROM properties WHERE main_image LIKE 'data:%'")
        .map_err(|e| e.to_string())?;
    let rows: Vec<(String, String)> = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    drop(stmt);

    for (id, data_url) in rows {
        // نفس الصورة غالبًا مُرّحت أعلى: استخدم نفس المسار لتجنب تكرار الملف
        let outcome = if let Some(relative) = url_map.get(&data_url) {
            Ok((relative.clone(), 0))
        } else {
            migrate_one(&data_url, store)
        };
        match outcome {
            Ok((relative, bytes)) => {
                let updated = conn
                    .execute(
                        "UPDATE properties SET main_image = ?1 WHERE id = ?2 AND main_image = ?3",
                        rusqlite::params![relative, id, data_url],
                    )
                    .map_err(|e| e.to_string())?;
                if updated == 1 {
                    stats.main_images_migrated += 1;
                    stats.bytes_written += bytes;
                } else {
                    stats.errors += 1;
                }
            }
            Err(_) => {
                stats.errors += 1;
            }
        }
    }

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_validates() {
        let ok = decode_data_url("data:image/png;base64,aGVsbG8=").unwrap();
        assert_eq!(ok.0, "image/png");
        assert_eq!(ok.1, b"hello".to_vec());
        assert!(decode_data_url("http://x").is_err());
        assert!(decode_data_url("data:image/png;base64,###").is_err());
        assert!(decode_data_url("data:image/png;base64,").is_err());
    }
}
