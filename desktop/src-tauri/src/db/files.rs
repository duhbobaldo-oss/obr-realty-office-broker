//! تخزين ملفات محلي قابل للنقل مع manifest في قاعدة البيانات.

use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct FileStore {
    root: PathBuf,
}

impl FileStore {
    pub fn new(app_data_dir: &Path) -> Result<Self, String> {
        let root = app_data_dir.join("files");
        fs::create_dir_all(&root).map_err(|e| format!("فشل إنشاء مجلد الملفات: {e}"))?;
        fs::create_dir_all(root.join("attachments"))
            .map_err(|e| format!("فشل إنشاء attachments: {e}"))?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn absolute_from_relative(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    pub fn exists(&self, relative: &str) -> bool {
        self.absolute_from_relative(relative).is_file()
    }

    /// يحفظ بايتات ملف تحت اسم فريد ويعيد المسار النسبي + اسم التخزين.
    pub fn store_bytes(
        &self,
        original_name: &str,
        bytes: &[u8],
        subdir: &str,
    ) -> Result<(String, String), String> {
        let safe_subdir = subdir
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect::<String>();
        let dir = self.root.join(&safe_subdir);
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

        let ext = Path::new(original_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bin");
        let stored_name = format!("{}.{}", Uuid::new_v4(), ext);
        let relative = format!("{safe_subdir}/{stored_name}");
        let abs = self.root.join(&relative);
        fs::write(&abs, bytes).map_err(|e| format!("فشل كتابة الملف: {e}"))?;
        Ok((relative.replace('\\', "/"), stored_name))
    }

    pub fn mark_missing_flags(conn: &Connection) -> Result<u64, String> {
        // يُستدعى من طبقة أعلى مع معرفة root — هنا واجهة مساعدة للـ SQL فقط
        let _ = conn;
        Ok(0)
    }
}

pub fn refresh_missing_flags(conn: &Connection, store: &FileStore) -> Result<u64, String> {
    let mut stmt = conn
        .prepare("SELECT id, relative_path FROM file_assets")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?;

    let mut updated = 0u64;
    for row in rows {
        let (id, rel) = row.map_err(|e| e.to_string())?;
        let missing = if store.exists(&rel) { 0 } else { 1 };
        let n = conn
            .execute(
                "UPDATE file_assets SET missing = ?1 WHERE id = ?2",
                rusqlite::params![missing, id],
            )
            .map_err(|e| e.to_string())?;
        updated += n as u64;
    }
    Ok(updated)
}
