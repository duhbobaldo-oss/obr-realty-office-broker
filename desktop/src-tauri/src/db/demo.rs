//! Official training data (`obr-training-v1`) — seed, auto-install, delete, reseed.

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use super::files::FileStore;
use super::training_seed;

pub const DEMO_BATCH_ID: &str = "obr-training-v1";
pub const LEGACY_DEMO_BATCH_ID: &str = "obr-demo-v1";
pub const DEMO_BATCH_LABEL: &str = "بيانات تدريب OBR الرسمية";
pub const TRAINING_SEED_DELETED_KEY: &str = "training_seed_deleted";

const OPS_TABLES: &[&str] = &[
    "finance_payments",
    "finance_commissions",
    "finance_invoices",
    "finance_expenses",
    "finance_operations",
    "communications",
    "client_property_links",
    "client_roles",
    "property_images",
    "property_updates",
    "brochures",
    "marketing_campaigns",
    "office_appointments",
    "market_reference_data",
    "clients",
    "properties",
    "file_assets",
];

pub fn get_app_meta(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT value FROM app_meta WHERE key = ?1",
        params![key],
        |r| r.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())
}

pub fn set_app_meta(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO app_meta(key, value, updated_at) VALUES (?1, ?2, datetime('now'))
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')",
        params![key, value],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn training_seed_deleted(conn: &Connection) -> Result<bool, String> {
    Ok(get_app_meta(conn, TRAINING_SEED_DELETED_KEY)?
        .map(|v| v == "true")
        .unwrap_or(false))
}

fn count_all(conn: &Connection, table: &str) -> Result<i64, String> {
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .map_err(|e| format!("count {table}: {e}"))
}

fn count_batch(conn: &Connection, table: &str, batch_id: &str) -> Result<i64, String> {
    let has: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info(?1) WHERE name='seed_batch_id'",
            params![table],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if has == 0 {
        return Ok(0);
    }
    conn.query_row(
        &format!("SELECT COUNT(*) FROM {table} WHERE seed_batch_id = ?1"),
        params![batch_id],
        |r| r.get(0),
    )
    .map_err(|e| format!("count_batch {table}: {e}"))
}

fn counts_map(conn: &Connection, tables: &[&str]) -> Result<BTreeMap<String, i64>, String> {
    let mut m = BTreeMap::new();
    for t in tables {
        m.insert((*t).to_string(), count_all(conn, t)?);
    }
    Ok(m)
}

pub(crate) fn batch_counts_map(conn: &Connection, batch_id: &str) -> Result<BTreeMap<String, i64>, String> {
    let mut m = BTreeMap::new();
    for t in OPS_TABLES {
        m.insert((*t).to_string(), count_batch(conn, t, batch_id)?);
    }
    let batch_row: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM seed_batches WHERE id = ?1",
            params![batch_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    m.insert("seed_batches".into(), batch_row);
    Ok(m)
}

fn remove_training_files(app_data_dir: &Path) -> Result<(), String> {
    // المسار المطابق لما يكتبه FileStore::store_bytes (sanitized: '/' → '_')
    let dir = app_data_dir.join("files").join("training_obr-training-v1");
    if dir.is_dir() {
        fs::remove_dir_all(&dir).map_err(|e| format!("فشل حذف ملفات التدريب: {e}"))?;
    }
    // توافق قديم: الاسم غير المسطّح إن وُجد
    let legacy = app_data_dir.join("files").join("training").join("obr-training-v1");
    if legacy.is_dir() {
        fs::remove_dir_all(&legacy).map_err(|e| format!("فشل حذف ملفات التدريب: {e}"))?;
    }
    Ok(())
}

/// حذف الملفات الفعلية المسجلة في file_assets للدفعة (مثل صور العروض تحت properties/)
/// — قبل حذف صفوف file_assets، وبعد commit الناجح.
fn remove_batch_files(
    app_data_dir: &Path,
    relative_paths: &[String],
) -> Result<(), String> {
    let root = app_data_dir.join("files");
    for rel in relative_paths {
        if rel.is_empty() || rel.contains("..") || rel.contains('\\') {
            continue;
        }
        let abs = root.join(rel);
        // حماية: يجب أن يبقى داخل مجلد files
        if abs.starts_with(&root) && abs.is_file() {
            fs::remove_file(&abs).map_err(|e| format!("فشل حذف ملف التدريب {rel}: {e}"))?;
        }
    }
    Ok(())
}

fn migrate_legacy_batch_id(conn: &Connection) -> Result<(), String> {
    let legacy: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM seed_batches WHERE id = ?1",
            params![LEGACY_DEMO_BATCH_ID],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if legacy == 0 {
        return Ok(());
    }
    let tables = [
        "properties",
        "property_images",
        "property_updates",
        "clients",
        "communications",
        "client_property_links",
        "market_reference_data",
        "brochures",
        "client_roles",
        "marketing_campaigns",
        "office_appointments",
        "finance_operations",
        "finance_invoices",
        "finance_commissions",
        "finance_expenses",
        "finance_payments",
        "audit_logs",
        "file_assets",
    ];
    for t in tables {
        let has: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info(?1) WHERE name='seed_batch_id'",
                params![t],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if has > 0 {
            conn.execute(
                &format!(
                    "UPDATE {t} SET seed_batch_id = ?1, is_training = 1 WHERE seed_batch_id = ?2"
                ),
                params![DEMO_BATCH_ID, LEGACY_DEMO_BATCH_ID],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    conn.execute(
        "UPDATE seed_batches SET id = ?1, label = ?2 WHERE id = ?3",
        params![DEMO_BATCH_ID, DEMO_BATCH_LABEL, LEGACY_DEMO_BATCH_ID],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// يمسح كل البيانات التشغيلية (وليس users/licenses/sessions/settings/migrations/app_meta/backup_history).
pub fn purge_operational_data(conn: &mut Connection) -> Result<BTreeMap<String, i64>, String> {
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| e.to_string())?;

    let before = counts_map(conn, OPS_TABLES)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let deletes = [
        "DELETE FROM finance_payments",
        "DELETE FROM finance_commissions",
        "DELETE FROM finance_invoices",
        "DELETE FROM finance_expenses",
        "DELETE FROM finance_operations",
        "DELETE FROM communications",
        "DELETE FROM client_property_links",
        "DELETE FROM client_roles",
        "DELETE FROM property_images",
        "DELETE FROM property_updates",
        "DELETE FROM brochures",
        "DELETE FROM marketing_campaigns",
        "DELETE FROM office_appointments",
        "DELETE FROM market_reference_data",
        "DELETE FROM clients",
        "DELETE FROM properties",
        "DELETE FROM file_assets WHERE seed_batch_id IS NOT NULL",
        "DELETE FROM seed_batches",
    ];
    for sql in deletes {
        tx.execute(sql, [])
            .map_err(|e| format!("فشل التنظيف ({sql}): {e}"))?;
    }

    tx.commit().map_err(|e| e.to_string())?;

    let after = counts_map(conn, OPS_TABLES)?;
    let mut removed = BTreeMap::new();
    for t in OPS_TABLES {
        let b = *before.get(*t).unwrap_or(&0);
        let a = *after.get(*t).unwrap_or(&0);
        removed.insert((*t).to_string(), b - a);
    }
    Ok(removed)
}

pub fn delete_demo_batch(
    conn: &mut Connection,
    batch_id: &str,
    app_data_dir: &Path,
) -> Result<BTreeMap<String, i64>, String> {
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| e.to_string())?;

    let before = batch_counts_map(conn, batch_id)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    // جمع المسارات النسبية لملفات الدفعة قبل حذف الصفوف (لحذف الملفات الفعلية بعد commit)
    let mut batch_file_paths: Vec<String> = Vec::new();
    {
        let mut stmt = tx
            .prepare(
                "SELECT relative_path FROM file_assets WHERE seed_batch_id = ?1 AND relative_path != ''",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![batch_id], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        for row in rows {
            batch_file_paths.push(row.map_err(|e| e.to_string())?);
        }
    }

    let stmts = [
        "DELETE FROM finance_payments WHERE seed_batch_id = ?1",
        "DELETE FROM finance_commissions WHERE seed_batch_id = ?1",
        "DELETE FROM finance_invoices WHERE seed_batch_id = ?1",
        "DELETE FROM finance_expenses WHERE seed_batch_id = ?1",
        "DELETE FROM finance_operations WHERE seed_batch_id = ?1",
        "DELETE FROM communications WHERE seed_batch_id = ?1",
        "DELETE FROM client_property_links WHERE seed_batch_id = ?1",
        "DELETE FROM client_roles WHERE seed_batch_id = ?1",
        "DELETE FROM property_images WHERE seed_batch_id = ?1",
        "DELETE FROM property_updates WHERE seed_batch_id = ?1",
        "DELETE FROM brochures WHERE seed_batch_id = ?1",
        "DELETE FROM marketing_campaigns WHERE seed_batch_id = ?1",
        "DELETE FROM office_appointments WHERE seed_batch_id = ?1",
        "DELETE FROM market_reference_data WHERE seed_batch_id = ?1",
        "DELETE FROM clients WHERE seed_batch_id = ?1",
        "DELETE FROM properties WHERE seed_batch_id = ?1",
        "DELETE FROM file_assets WHERE seed_batch_id = ?1",
        "DELETE FROM seed_batches WHERE id = ?1",
    ];
    for sql in stmts {
        tx.execute(sql, params![batch_id])
            .map_err(|e| format!("فشل حذف التدريب ({sql}): {e}"))?;
    }

    tx.commit().map_err(|e| e.to_string())?;

    if batch_id == DEMO_BATCH_ID {
        remove_training_files(app_data_dir)?;
        remove_batch_files(app_data_dir, &batch_file_paths)?;
        set_app_meta(conn, TRAINING_SEED_DELETED_KEY, "true")?;
    }
    Ok(before)
}

pub fn demo_status(conn: &Connection, batch_id: &str) -> Result<Value, String> {
    let _ = migrate_legacy_batch_id(conn);
    let batch_exists: bool = conn
        .query_row(
            "SELECT 1 FROM seed_batches WHERE id = ?1",
            params![batch_id],
            |_| Ok(true),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or(false);

    let demo = batch_counts_map(conn, batch_id)?;
    let mut total_demo = 0i64;
    for (k, v) in &demo {
        if k != "seed_batches" {
            total_demo += v;
        }
    }

    let non_demo_clients: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM clients WHERE seed_batch_id IS NULL OR seed_batch_id = ''",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let non_demo_properties: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM properties WHERE seed_batch_id IS NULL OR seed_batch_id = ''",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let deleted_flag = training_seed_deleted(conn)?;

    Ok(json!({
        "batch_id": batch_id,
        "batch_exists": batch_exists,
        "demo_counts": demo,
        "demo_total_records": total_demo,
        "non_demo_clients": non_demo_clients,
        "non_demo_properties": non_demo_properties,
        "training_seed_deleted": deleted_flag,
        "identification": "seed_batch_id=obr-training-v1, is_training=1",
        "auto_seed_on_install": true
    }))
}

fn map_to_json(m: &BTreeMap<String, i64>) -> Value {
    let mut obj = Map::new();
    for (k, v) in m {
        obj.insert(k.clone(), json!(*v));
    }
    Value::Object(obj)
}

pub fn seed_demo(
    conn: &mut Connection,
    user_id: &str,
    app_data_dir: &Path,
) -> Result<BTreeMap<String, i64>, String> {
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| e.to_string())?;
    migrate_legacy_batch_id(conn)?;

    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM seed_batches WHERE id = ?1",
            params![DEMO_BATCH_ID],
            |_| Ok(true),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or(false);
    if exists {
        return Err(format!(
            "الدفعة {DEMO_BATCH_ID} موجودة. احذف البيانات التدريبية أولاً ثم أعد التحميل."
        ));
    }

    let _store = FileStore::new(app_data_dir)?;
    let counts = training_seed::insert_training_batch(conn, user_id, app_data_dir)?;
    set_app_meta(conn, TRAINING_SEED_DELETED_KEY, "false")?;
    Ok(counts)
}

/// قاعدة بيانات Fresh Install حقيقية = صفر سجلات تشغيلية في كل الجداول.
/// تُستخدم لمنع زرع بيانات التدريب تلقائيًا داخل قاعدة مستخدم قديمة أو مُرقّاة.
fn is_fresh_install(conn: &Connection) -> Result<bool, String> {
    for t in OPS_TABLES {
        if count_all(conn, t)? > 0 {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Auto-seed on first run unless user deleted training data.
/// يُزرع التدريب تلقائيًا فقط في قاعدة Fresh Install حقيقية؛
/// قاعدة مستخدم قديمة أو مُرقّاة لا تُزرع فيها بيانات تدريب تلقائيًا أبدًا.
pub fn ensure_official_training_seed(
    conn: &mut Connection,
    user_id: &str,
    app_data_dir: &Path,
) -> Result<bool, String> {
    migrate_legacy_batch_id(conn)?;
    if training_seed_deleted(conn)? {
        return Ok(false);
    }
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM seed_batches WHERE id = ?1",
            params![DEMO_BATCH_ID],
            |_| Ok(true),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or(false);
    if exists {
        return Ok(false);
    }
    if !is_fresh_install(conn)? {
        return Ok(false);
    }
    seed_demo(conn, user_id, app_data_dir)?;
    Ok(true)
}

pub fn reseed_demo(
    conn: &mut Connection,
    user_id: &str,
    app_data_dir: &Path,
) -> Result<Value, String> {
    let deleted = if conn
        .query_row(
            "SELECT 1 FROM seed_batches WHERE id = ?1",
            params![DEMO_BATCH_ID],
            |_| Ok(true),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or(false)
    {
        delete_demo_batch(conn, DEMO_BATCH_ID, app_data_dir)?
    } else {
        BTreeMap::new()
    };
    set_app_meta(conn, TRAINING_SEED_DELETED_KEY, "false")?;
    let seeded = seed_demo(conn, user_id, app_data_dir)?;
    Ok(json!({
        "batch_id": DEMO_BATCH_ID,
        "deleted_before_seed": map_to_json(&deleted),
        "seeded": map_to_json(&seeded)
    }))
}

pub fn prepare_demo_environment(
    conn: &mut Connection,
    user_id: &str,
    app_data_dir: &Path,
) -> Result<(BTreeMap<String, i64>, BTreeMap<String, i64>), String> {
    let removed = purge_operational_data(conn)?;
    set_app_meta(conn, TRAINING_SEED_DELETED_KEY, "false")?;
    let seeded = seed_demo(conn, user_id, app_data_dir)?;
    Ok((removed, seeded))
}

pub fn counts_to_json(m: &BTreeMap<String, i64>) -> Value {
    map_to_json(m)
}
