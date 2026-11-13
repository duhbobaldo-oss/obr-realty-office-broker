//! نسخ احتياطي محلي: حزمة مجلد (DB + files + manifest) مع رقم وتاريخ، واستعادة حقيقية.
//! الإنشاء: كتابة في staging بنفس مجلد backups ثم التحقق ثم rename ذرّي للاسم النهائي.

use crate::db::files::FileStore;
use crate::db::migrations;
use rusqlite::Connection;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct BackupPackage {
    pub path: PathBuf,
    pub schema_version: i32,
    pub sequence: u32,
    pub stamp: String,
    pub folder_name: String,
}

const PARTIAL_PREFIX: &str = ".partial-";

fn is_partial_backup_name(name: &str) -> bool {
    name.starts_with(PARTIAL_PREFIX)
}

#[cfg(test)]
thread_local! {
    /// حقن فشل للاختبار فقط: `"db"` | `"files"` | `"manifest"` | `"validate"`
    static TEST_FAIL_STAGE: std::cell::Cell<Option<&'static str>> =
        std::cell::Cell::new(None);
}

#[cfg(test)]
pub fn test_set_backup_fail_stage(stage: Option<&'static str>) {
    TEST_FAIL_STAGE.with(|c| c.set(stage));
}

#[cfg(test)]
fn test_maybe_fail(stage: &'static str) -> Result<(), String> {
    TEST_FAIL_STAGE.with(|c| {
        if c.get() == Some(stage) {
            Err(format!("test injected fail at {stage}"))
        } else {
            Ok(())
        }
    })
}

#[cfg(not(test))]
fn test_maybe_fail(_stage: &'static str) -> Result<(), String> {
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    if !src.exists() {
        fs::create_dir_all(dst).map_err(|e| e.to_string())?;
        return Ok(());
    }
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let ty = entry.file_type().map_err(|e| e.to_string())?;
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &to)?;
        } else if ty.is_file() {
            fs::copy(entry.path(), &to).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// يحذف فقط مجلد staging لهذه العملية — لا يمس partial قديمًا عشوائيًا.
fn cleanup_staging_dir(staging: &Path) {
    if staging.exists() {
        let _ = fs::remove_dir_all(staging);
    }
}

/// أعلى رقم تسلسلي موجود في مجلد النسخ + 1 (يتجاهل `.partial-*`)
pub fn next_backup_sequence(app_data_dir: &Path) -> u32 {
    let dir = app_data_dir.join("backups");
    let mut max = 0u32;
    let Ok(rd) = fs::read_dir(&dir) else {
        return 1;
    };
    for entry in rd.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if is_partial_backup_name(&name) {
            continue;
        }
        // obr-backup-001-20260315-143022
        let rest = match name.strip_prefix("obr-backup-") {
            Some(r) => r,
            None => continue,
        };
        if let Some(num_str) = rest.split('-').next() {
            if let Ok(n) = num_str.parse::<u32>() {
                max = max.max(n);
            }
        }
    }
    max + 1
}

fn sanitize_backup_folder_name(folder_name: &str) -> Result<String, String> {
    let safe_name = folder_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>();
    if safe_name.is_empty() || is_partial_backup_name(&safe_name) {
        return Err("اسم حزمة النسخ الاحتياطي غير صالح".into());
    }
    Ok(safe_name)
}

/// يضمن اسمًا نهائيًا فريدًا داخل backups — بدون overwrite.
fn allocate_unique_final_name(backups_dir: &Path, preferred: &str) -> Result<String, String> {
    let candidate = backups_dir.join(preferred);
    if !candidate.exists() {
        return Ok(preferred.to_string());
    }
    // تعارض نادر (نفس الثانية): لا نستبدل — نضيف لاحقة فريدة
    for _ in 0..8 {
        let suffix = &uuid::Uuid::new_v4().simple().to_string()[..8];
        let alt = format!("{preferred}-{suffix}");
        if !backups_dir.join(&alt).exists() {
            return Ok(alt);
        }
    }
    Err(format!(
        "تعذر تخصيص اسم فريد للنسخة دون استبدال: {}",
        preferred
    ))
}

/// شروط جوهرية قبل إعلان النسخة نهائية (أقرب ما يمكن لـ restorable + سلامة).
pub fn validate_backup_package_complete(package_path: &Path) -> Result<(), String> {
    if !package_path.is_dir() {
        return Err("مسار حزمة النسخ غير صالح".into());
    }
    let db = package_path.join("mawasib.db");
    if !db.is_file() {
        return Err("الحزمة لا تحتوي على mawasib.db".into());
    }
    let db_size = fs::metadata(&db).map(|m| m.len()).unwrap_or(0);
    if db_size == 0 {
        return Err("نسخة قاعدة البيانات في الحزمة فارغة".into());
    }
    db_integrity_ok(&db)?;

    let files_dir = package_path.join("files");
    if !files_dir.is_dir() {
        return Err("الحزمة لا تحتوي على مجلد files/".into());
    }

    let manifest_path = package_path.join("manifest.json");
    if !manifest_path.is_file() {
        return Err("الحزمة لا تحتوي على manifest.json".into());
    }
    let raw = fs::read_to_string(&manifest_path).map_err(|e| format!("تعذر قراءة manifest: {e}"))?;
    let v: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| format!("manifest.json غير صالح: {e}"))?;
    let format = v.get("format").and_then(|x| x.as_str()).unwrap_or("");
    if format != "obr-backup-v1" {
        return Err(format!("صيغة حزمة غير مدعومة: {format}"));
    }
    if v.get("schema_version").and_then(|x| x.as_i64()).is_none() {
        return Err("manifest بلا schema_version".into());
    }
    Ok(())
}

/// نسخة أمان إلزامية قبل الاستعادة. أي فشل هنا يجب أن يمنع الاستعادة تمامًا.
pub fn create_pre_restore_safety_backup(
    app_data_dir: &Path,
    db_path: &Path,
    conn: &Connection,
    file_store: &FileStore,
    user_id: Option<&str>,
) -> Result<BackupPackage, String> {
    create_backup_package(app_data_dir, db_path, conn, file_store, user_id).map_err(|e| {
        format!("تعذر إنشاء نسخة أمان قبل الاستعادة — أُلغيت الاستعادة ولم يُمس أي شيء: {e}")
    })
}

pub fn create_backup_package(
    app_data_dir: &Path,
    db_path: &Path,
    conn: &Connection,
    file_store: &FileStore,
    user_id: Option<&str>,
) -> Result<BackupPackage, String> {
    let seq = next_backup_sequence(app_data_dir);
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let folder_name = format!("obr-backup-{:03}-{}", seq, stamp);
    create_backup_package_named(
        app_data_dir,
        db_path,
        conn,
        file_store,
        user_id,
        &folder_name,
        &format!("manual backup #{seq} @ {stamp}"),
        seq,
        &stamp,
    )
}

pub fn create_backup_package_named(
    app_data_dir: &Path,
    _db_path: &Path,
    conn: &Connection,
    file_store: &FileStore,
    user_id: Option<&str>,
    folder_name: &str,
    notes: &str,
    sequence: u32,
    stamp: &str,
) -> Result<BackupPackage, String> {
    let schema_version = migrations::schema_version(conn)?;
    let preferred = sanitize_backup_folder_name(folder_name)?;

    let backups_dir = app_data_dir.join("backups");
    fs::create_dir_all(&backups_dir).map_err(|e| e.to_string())?;

    let final_name = allocate_unique_final_name(&backups_dir, &preferred)?;
    let final_root = backups_dir.join(&final_name);
    // دفاع إضافي: لا overwrite أبدًا
    if final_root.exists() {
        return Err(format!(
            "حزمة النسخ موجودة مسبقاً — رُفض الاستبدال: {}",
            final_root.display()
        ));
    }

    let staging_name = format!("{PARTIAL_PREFIX}{}", uuid::Uuid::new_v4());
    let staging_root = backups_dir.join(&staging_name);
    if staging_root.exists() {
        return Err("تعارض مجلد staging للنسخ الاحتياطي".into());
    }

    let write_result = (|| -> Result<(), String> {
        fs::create_dir_all(&staging_root).map_err(|e| e.to_string())?;

        let db_dest = staging_root.join("mawasib.db");
        copy_database_consistent(conn, &db_dest)?;
        test_maybe_fail("db")?;

        let files_dest = staging_root.join("files");
        copy_dir_recursive(file_store.root(), &files_dest)?;
        test_maybe_fail("files")?;

        let db_size = fs::metadata(&db_dest).map(|m| m.len()).unwrap_or(0);
        if db_size == 0 {
            return Err("نسخة قاعدة البيانات في الحزمة فارغة أو غير موجودة".into());
        }

        let manifest = json!({
            "format": "obr-backup-v1",
            "created_at": chrono::Utc::now().to_rfc3339(),
            "local_stamp": stamp,
            "sequence": sequence,
            "schema_version": schema_version,
            "app_id": "com.mawasib.obr",
            "folder_name": final_name,
            "includes": ["mawasib.db", "files/", "manifest.json"],
            "db_bytes": db_size,
            "restore_supported": true,
            "notes": notes
        });
        fs::write(
            staging_root.join("manifest.json"),
            serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?,
        )
        .map_err(|e| format!("فشل كتابة manifest: {e}"))?;
        test_maybe_fail("manifest")?;

        validate_backup_package_complete(&staging_root)?;
        test_maybe_fail("validate")?;
        Ok(())
    })();

    if let Err(e) = write_result {
        cleanup_staging_dir(&staging_root);
        return Err(e);
    }

    // الاسم النهائي يظهر فقط بعد اكتمال المكونات والتحقق
    if final_root.exists() {
        cleanup_staging_dir(&staging_root);
        return Err(format!(
            "حزمة النسخ ظهرت أثناء التحضير — رُفض الاستبدال: {}",
            final_root.display()
        ));
    }
    if let Err(e) = fs::rename(&staging_root, &final_root) {
        cleanup_staging_dir(&staging_root);
        return Err(format!("فشل تثبيت النسخة الاحتياطية نهائيًا: {e}"));
    }

    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO backup_history(id, user_id, package_path, schema_version, created_at, notes) VALUES (?1, ?2, ?3, ?4, datetime('now'), ?5)",
        rusqlite::params![
            id,
            user_id,
            final_root.to_string_lossy().to_string(),
            schema_version,
            notes
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(BackupPackage {
        path: final_root,
        schema_version,
        sequence,
        stamp: stamp.to_string(),
        folder_name: final_name,
    })
}

/// قائمة النسخ الاحتياطية المحلية (الأحدث أولاً) — تتجاهل `.partial-*`
pub fn list_backup_packages(app_data_dir: &Path) -> Result<Vec<serde_json::Value>, String> {
    let dir = app_data_dir.join("backups");
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut items = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            continue;
        }
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if is_partial_backup_name(&name) {
            continue;
        }
        let manifest_path = path.join("manifest.json");
        let mut sequence = 0u32;
        let mut stamp = String::new();
        let mut created_at = String::new();
        let mut schema_version = 0i32;
        if manifest_path.exists() {
            if let Ok(raw) = fs::read_to_string(&manifest_path) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                    sequence = v.get("sequence").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
                    stamp = v
                        .get("local_stamp")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string();
                    created_at = v
                        .get("created_at")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string();
                    schema_version = v
                        .get("schema_version")
                        .and_then(|x| x.as_i64())
                        .unwrap_or(0) as i32;
                }
            }
        }
        if stamp.is_empty() {
            // محاولة من اسم المجلد
            if let Some(rest) = name.strip_prefix("obr-backup-") {
                let parts: Vec<&str> = rest.splitn(2, '-').collect();
                if parts.len() == 2 {
                    sequence = parts[0].parse().unwrap_or(0);
                    stamp = parts[1].to_string();
                }
            }
        }
        let has_db = path.join("mawasib.db").exists();
        let db_bytes = fs::metadata(path.join("mawasib.db"))
            .map(|m| m.len())
            .unwrap_or(0);
        items.push(json!({
            "folder_name": name,
            "path": path.to_string_lossy(),
            "sequence": sequence,
            "stamp": stamp,
            "created_at": created_at,
            "schema_version": schema_version,
            "db_bytes": db_bytes,
            "restorable": has_db
        }));
    }
    items.sort_by(|a, b| {
        let sa = a.get("sequence").and_then(|x| x.as_u64()).unwrap_or(0);
        let sb = b.get("sequence").and_then(|x| x.as_u64()).unwrap_or(0);
        sb.cmp(&sa)
    });
    Ok(items)
}

/// حجم وعدد ملفات شجرة مجلد (للتحقق بعد النسخ)
fn dir_stats(dir: &Path) -> (u64, u64) {
    let mut count = 0u64;
    let mut bytes = 0u64;
    if let Ok(rd) = fs::read_dir(dir) {
        for entry in rd.flatten() {
            if let Ok(ty) = entry.file_type() {
                if ty.is_dir() {
                    let (c, b) = dir_stats(&entry.path());
                    count += c;
                    bytes += b;
                } else if ty.is_file() {
                    count += 1;
                    bytes += fs::metadata(entry.path()).map(|m| m.len()).unwrap_or(0);
                }
            }
        }
    }
    (count, bytes)
}

fn db_integrity_ok(path: &Path) -> Result<(), String> {
    let probe = crate::db::open_configured(path)
        .map_err(|e| format!("تعذر فتح قاعدة البيانات: {e}"))?;
    let ok: String = probe
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|e| format!("قاعدة البيانات تالفة أو غير قابلة للقراءة: {e}"))?;
    if ok != "ok" {
        return Err(format!("قاعدة البيانات تالفة (integrity={ok})"));
    }
    Ok(())
}

/// لقطة متسقة عبر SQLite Online Backup API (آمنة مع DELETE أو WAL دون تغيير صيغة الحزمة).
fn copy_database_consistent(src: &Connection, dest: &Path) -> Result<(), String> {
    if dest.exists() {
        fs::remove_file(dest).map_err(|e| format!("تعذر تجهيز ملف النسخة: {e}"))?;
    }
    // إن وُجد WAL، ادفع ما أمكن؛ Backup API يلتقط الحالة المتسقة بغض النظر.
    let _ = src.execute_batch("PRAGMA wal_checkpoint(PASSIVE);");

    let mut dst = Connection::open(dest).map_err(|e| format!("تعذر إنشاء ملف النسخة: {e}"))?;
    {
        let backup = rusqlite::backup::Backup::new(src, &mut dst)
            .map_err(|e| format!("تعذر بدء النسخ المتسق: {e}"))?;
        backup
            .run_to_completion(64, std::time::Duration::from_millis(10), None)
            .map_err(|e| format!("فشل النسخ المتسق لقاعدة البيانات: {e}"))?;
    }
    Ok(())
}

/// يستبدل قاعدة البيانات وملفات التطبيق بمحتوى حزمة النسخ بشكل شبه-آتومي:
/// 1) فك النسخة إلى ملف مؤقت + فحص سلامة النسخة الأصلية والمؤقتة قبل لمس أي شيء حي.
/// 2) الملفات: تُنقل القديمة جانبًا (rename) ثم تُنسخ الجديدة وتُطابق إحصائيًا.
/// 3) القاعدة: تُنقل الحية جانبًا (rename بدل الحذف) ثم تُستبدل ويُفحص التكامل نهائيًا.
/// عند أي فشل: Rollback كامل — القاعدة والملفات القديمة تعود كما كانت وقابلة للاسترجاع.
/// يجب إغلاق اتصال SQLite الحي قبل الاستدعاء.
pub fn restore_backup_package(
    app_data_dir: &Path,
    db_path: &Path,
    package_path: &Path,
) -> Result<serde_json::Value, String> {
    if !package_path.is_dir() {
        return Err("مسار النسخة الاحتياطية غير صالح".into());
    }
    let src_db = package_path.join("mawasib.db");
    if !src_db.exists() {
        return Err("الحزمة لا تحتوي على mawasib.db".into());
    }
    let src_size = fs::metadata(&src_db).map(|m| m.len()).unwrap_or(0);
    if src_size == 0 {
        return Err("ملف قاعدة البيانات في الحزمة فارغ".into());
    }

    // 1) فحص سلامة النسخة الأصلية داخل الحزمة
    db_integrity_ok(&src_db)?;

    // 2) فك إلى مسار مؤقت وفحص النسخة المؤقتة نفسها
    let tmp_db = app_data_dir.join("mawasib.db.restore-tmp");
    let _ = fs::remove_file(&tmp_db);
    fs::copy(&src_db, &tmp_db).map_err(|e| format!("فشل تجهيز الاستعادة: {e}"))?;
    db_integrity_ok(&tmp_db)?;

    // لا تُستعاد جلسات قديمة: قد تكون أُبطلت بعد إنشاء النسخة، وإعادتها تُحيي توكن Logout.
    // التعديل على staging فقط قبل تبديل القاعدة الحية، مع تثبيته داخل ملف DB لا WAL منفصل.
    let revoke_staged_sessions = (|| -> Result<(), String> {
        let conn = Connection::open(&tmp_db).map_err(|e| e.to_string())?;
        conn.pragma_update(None, "journal_mode", "DELETE").map_err(|e| e.to_string())?;
        let has_sessions: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='sessions'",
            [], |row| row.get(0),
        ).map_err(|e| e.to_string())?;
        if has_sessions > 0 {
            conn.execute("DELETE FROM sessions", []).map_err(|e| e.to_string())?;
        }
        Ok(())
    })();
    if let Err(error) = revoke_staged_sessions {
        let _ = fs::remove_file(&tmp_db);
        return Err(format!("تعذر إبطال جلسات النسخة المستعادة: {error}"));
    }
    db_integrity_ok(&tmp_db)?;

    let files_src = package_path.join("files");
    let files_dst = app_data_dir.join("files");
    let restore_files = files_src.exists();
    let src_files_stats = if restore_files {
        let s = dir_stats(&files_src);
        if s.0 == 0 && s.1 == 0 {
            (0u64, 0u64) // مجلد files فارغ في الحزمة مسموح
        } else {
            s
        }
    } else {
        (0u64, 0u64)
    };

    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let pre_db = app_data_dir.join(format!("mawasib.db.pre-restore-{stamp}"));
    let pre_files = app_data_dir.join(format!("files.pre-restore-{stamp}"));
    let wal = PathBuf::from(format!("{}-wal", db_path.display()));
    let shm = PathBuf::from(format!("{}-shm", db_path.display()));
    let mut files_moved_aside = false;

    // دالة الرجوع للخلف: تعيد القاعدة والملفات القديمة عند أي فشل.
    // db_moved=true فقط عندما تكون القاعدة الحية قد نُقلت جانبًا فعليًا،
    // وإلا يُمنع لمس ملف القاعدة الحية إطلاقًا.
    let rollback = |files_moved: bool, db_moved: bool| -> Result<(), String> {
        // إرجاع القاعدة القديمة (فقط إذا كانت قد نُقلت جانبًا)
        if db_moved {
            let _ = fs::remove_file(db_path);
            let _ = fs::remove_file(&wal);
            let _ = fs::remove_file(&shm);
            if pre_db.exists() {
                fs::rename(&pre_db, db_path)
                    .map_err(|e| format!("فشل Rollback القاعدة: {e}"))?;
            }
        }
        // إرجاع ملفات القديمة
        if files_moved {
            if files_dst.exists() {
                let _ = fs::remove_dir_all(&files_dst);
            }
            if pre_files.exists() {
                fs::rename(&pre_files, &files_dst)
                    .map_err(|e| format!("فشل Rollback الملفات: {e}"))?;
            }
        }
        Ok(())
    };

    // 3) الملفات أولًا (الخطأ الأرجح): نقل القديمة جانبًا ثم النسخ والمطابقة
    if restore_files {
        if files_dst.exists() {
            fs::rename(&files_dst, &pre_files).map_err(|e| format!("فشل نقل الملفات القديمة: {e}"))?;
            files_moved_aside = true;
        }
        if let Err(e) = copy_dir_recursive(&files_src, &files_dst) {
            rollback(files_moved_aside, false)?;
            let _ = fs::remove_file(&tmp_db);
            return Err(format!("فشل نسخ الملفات — لم يتغير شيء: {e}"));
        }
        let dst_stats = dir_stats(&files_dst);
        if dst_stats.0 != src_files_stats.0 || dst_stats.1 != src_files_stats.1 {
            rollback(files_moved_aside, false)?;
            let _ = fs::remove_file(&tmp_db);
            return Err(format!(
                "عدم تطابق الملفات بعد النسخ (src={src_files_stats:?}, dst={dst_stats:?}) — لم يتغير شيء"
            ));
        }
    }

    // 4) القاعدة: نقل الحية جانبًا (rename بدل الحذف) ثم الاستبدال
    if db_path.exists() {
        if let Err(e) = fs::rename(db_path, &pre_db) {
            rollback(files_moved_aside, false)?;
            let _ = fs::remove_file(&tmp_db);
            return Err(format!("فشل تحزين القاعدة الحية: {e}"));
        }
    }
    let _ = fs::remove_file(&wal);
    let _ = fs::remove_file(&shm);
    if let Err(e) = fs::rename(&tmp_db, db_path).or_else(|_| {
        fs::copy(&tmp_db, db_path).map(|_| ()).map_err(|e| e.to_string())
    }) {
        rollback(files_moved_aside, true)?;
        let _ = fs::remove_file(&tmp_db);
        return Err(format!("فشل استبدال القاعدة: {e}"));
    }
    let _ = fs::remove_file(&tmp_db);

    // 5) تحقق نهائي من القاعدة الحية الجديدة
    if let Err(e) = db_integrity_ok(db_path) {
        rollback(files_moved_aside, true)?;
        return Err(format!("فشل التحقق النهائي — تمت إعادة القاعدة القديمة: {e}"));
    }

    // 6) تنظيف ملفات الرجوع بعد نجاح كامل (نسخة الأمان تبقى في backups/)
    let _ = fs::remove_file(&pre_db);
    let _ = fs::remove_file(&wal);
    let _ = fs::remove_file(&shm);
    if files_moved_aside {
        let _ = fs::remove_dir_all(&pre_files);
    }

    let schema_version = {
        let conn = crate::db::open_configured(db_path)?;
        migrations::schema_version(&conn)?
    };

    Ok(json!({
        "restored": true,
        "package_path": package_path.to_string_lossy(),
        "db_path": db_path.to_string_lossy(),
        "schema_version": schema_version,
        "files_restored": restore_files,
        "files_count": src_files_stats.0,
        "atomic": "pre-restore rename + integrity verify + rollback"
    }))
}
