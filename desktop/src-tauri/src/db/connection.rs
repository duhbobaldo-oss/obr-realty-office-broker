//! إعدادات اتصال SQLite الموحّدة لاتصالات الإنتاج.
//!
//! السياسة (عمدًا):
//! - `foreign_keys=ON` — مطلوب لصحة العلاقات.
//! - `busy_timeout=5000ms` — يمنع فشلًا فوريًا بـ database is locked عند اتصال جانبي
//!   (نسخ احتياطي / demo / system_info) بينما الاتصال الأساسي مفتوح.
//! - لا نفرض `journal_mode=WAL` ولا نغيّر `synchronous` — نُبقي افتراضات SQLite
//!   (`DELETE` / `FULL`) ما لم يثبت لاحقًا حاجة صريحة.

use rusqlite::Connection;
use std::time::Duration;

/// مهلة انتظار القفل لاتصالات الإنتاج المهمة.
pub const BUSY_TIMEOUT_MS: u32 = 5_000;

/// يطبّق PRAGMAs الإنتاج الدنيا على اتصال مفتوح.
pub fn configure_connection(conn: &Connection) -> Result<(), String> {
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| format!("تعذر تفعيل foreign_keys: {e}"))?;
    conn.busy_timeout(Duration::from_millis(BUSY_TIMEOUT_MS as u64))
        .map_err(|e| format!("تعذر ضبط busy_timeout: {e}"))?;
    Ok(())
}

/// يفتح ملف SQLite ثم يطبّق إعدادات الإنتاج.
pub fn open_configured(path: &std::path::Path) -> Result<Connection, String> {
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    configure_connection(&conn)?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn configure_sets_foreign_keys_and_busy_timeout() {
        let path = temp_dir().join(format!("obr_conn_cfg_{}.db", uuid::Uuid::new_v4()));
        let conn = open_configured(&path).expect("open");
        let fk: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
            .unwrap();
        assert_eq!(fk, 1);
        let busy: i64 = conn
            .query_row("PRAGMA busy_timeout", [], |r| r.get(0))
            .unwrap();
        assert_eq!(busy, BUSY_TIMEOUT_MS as i64);
        // لا نفرض WAL
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mode.to_ascii_lowercase(), "delete");
        let _ = std::fs::remove_file(&path);
    }
}
