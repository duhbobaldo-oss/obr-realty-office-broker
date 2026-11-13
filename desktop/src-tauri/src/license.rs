//! هوية التثبيت المحلية (Installation ID) — بلا ترخيص ولا حساب في هذه النسخة.
//! Installation ID يبقى مطلوبًا تقنيًا لتحديد التثبيت عند خدمة تتبع الحملات.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub const PRODUCT_ID: &str = "obr-office-broker-realty";
pub const PRODUCT_NAME: &str = "OBR – مكتب الوسيط العقاري";
pub const APP_IDENTIFIER: &str = "com.mawasib.obr";
const INSTALLATION_FILE_PREFIX: &str = "DPAPI1:";

pub struct LicenseStore {
    root: PathBuf,
}

impl LicenseStore {
    pub fn new(root: impl Into<PathBuf>) -> Self { Self { root: root.into() } }
    fn installation_path(&self) -> PathBuf { self.root.join("installation.id") }

    pub fn installation_id(&self) -> Result<String, String> {
        std::fs::create_dir_all(&self.root).map_err(|e| format!("تعذر إنشاء مجلد البيانات: {e}"))?;
        let path = self.installation_path();
        if path.exists() {
            let stored = std::fs::read_to_string(&path).map_err(|e| format!("تعذر قراءة Installation ID: {e}"))?;
            let stored = stored.trim();
            let value = if let Some(encoded) = stored.strip_prefix(INSTALLATION_FILE_PREFIX) {
                let encrypted = STANDARD.decode(encoded).map_err(|_| "ملف Installation ID المحمي تالف")?;
                String::from_utf8(unprotect_for_this_machine(&encrypted)?).map_err(|_| "ملف Installation ID المحمي تالف")?
            } else {
                // ترحيل آمن للتثبيتات الحالية: نحافظ على نفس UUID.
                Uuid::parse_str(stored).map_err(|_| "ملف Installation ID غير صالح".to_string())?;
                let protected = protect_for_this_machine(stored.as_bytes())?;
                write_atomic(&path, format!("{INSTALLATION_FILE_PREFIX}{}", STANDARD.encode(protected)).as_bytes())?;
                stored.to_string()
            };
            let value = value.trim();
            Uuid::parse_str(value).map_err(|_| "ملف Installation ID غير صالح".to_string())?;
            return Ok(value.to_string());
        }
        let value = Uuid::new_v4().to_string();
        let protected = protect_for_this_machine(value.as_bytes())?;
        write_atomic(&path, format!("{INSTALLATION_FILE_PREFIX}{}", STANDARD.encode(protected)).as_bytes())?;
        Ok(value)
    }
}

/// إزالة حالة هوية التثبيت فقط (أداة صيانة CLI) دون لمس بيانات المستخدم.
pub fn remove_installation_state(root: &Path) -> Result<(), String> {
    let store = LicenseStore::new(root);
    let path = store.installation_path();
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("تعذر حذف Installation ID: {e}"))?;
    }
    // ملفات ربط الهوية القديمة (من نسخ سابقة) تُنظف أيضًا
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("local-identity-") && name.ends_with(".id") {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
    Ok(())
}

#[cfg(windows)]
fn protect_for_this_machine(data: &[u8]) -> Result<Vec<u8>, String> {
    use windows::Win32::Security::Cryptography::{
        CryptProtectData, CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN,
    };
    if data.is_empty() {
        return Err("لا يمكن حماية بيانات فارغة".into());
    }
    let mut input = data.to_vec();
    let mut out = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptProtectData(
            &mut CRYPT_INTEGER_BLOB { cbData: input.len() as u32, pbData: input.as_mut_ptr() },
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut out,
        )
    }
    .map_err(|_| "فشل تشفير DPAPI".to_string())?;
    let protected = unsafe { std::slice::from_raw_parts(out.pbData, out.cbData as usize) }.to_vec();
    unsafe { windows::Win32::Foundation::LocalFree(windows::Win32::Foundation::HLOCAL(out.pbData as *mut _)) };
    Ok(protected)
}

#[cfg(windows)]
fn unprotect_for_this_machine(data: &[u8]) -> Result<Vec<u8>, String> {
    use windows::Win32::Security::Cryptography::{
        CryptUnprotectData, CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN,
    };
    if data.is_empty() {
        return Err("لا يمكن فك حماية بيانات فارغة".into());
    }
    let mut input = data.to_vec();
    let mut out = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptUnprotectData(
            &mut CRYPT_INTEGER_BLOB { cbData: input.len() as u32, pbData: input.as_mut_ptr() },
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut out,
        )
    }
    .map_err(|_| "فشل فك تشفير DPAPI".to_string())?;
    let plain = unsafe { std::slice::from_raw_parts(out.pbData, out.cbData as usize) }.to_vec();
    unsafe { windows::Win32::Foundation::LocalFree(windows::Win32::Foundation::HLOCAL(out.pbData as *mut _)) };
    Ok(plain)
}

#[cfg(not(windows))]
fn protect_for_this_machine(_: &[u8]) -> Result<Vec<u8>, String> { Err("حماية Installation ID مدعومة على Windows فقط".into()) }

#[cfg(not(windows))]
fn unprotect_for_this_machine(_: &[u8]) -> Result<Vec<u8>, String> { Err("حماية Installation ID مدعومة على Windows فقط".into()) }

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let tmp = path.with_extension("tmp");
    {
        let mut f = std::fs::File::create(&tmp).map_err(|e| format!("تعذر إنشاء ملف مؤقت: {e}"))?;
        f.write_all(bytes).map_err(|e| format!("تعذر كتابة الملف: {e}"))?;
        f.sync_all().map_err(|e| format!("تعذر تثبيت الملف: {e}"))?;
    }
    std::fs::rename(&tmp, path).map_err(|e| format!("تعذر تثبيت الملف: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installation_id_is_stable_and_well_formed() {
        let dir = std::env::temp_dir().join(format!("obr_inst_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = LicenseStore::new(&dir);
        let first = store.installation_id().unwrap();
        uuid::Uuid::parse_str(&first).expect("UUID صالح");
        let second = store.installation_id().unwrap();
        assert_eq!(first, second, "نفس التثبيت = نفس Installation ID");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn uninstall_resets_only_installation_state_and_preserves_customer_data() {
        let dir = std::env::temp_dir().join(format!("obr_uninst_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = LicenseStore::new(&dir);
        let id = store.installation_id().unwrap();
        std::fs::write(dir.join("mawasib.db"), b"customer data").unwrap();
        let legacy_binding = dir.join(format!("local-identity-{id}.id"));
        std::fs::write(&legacy_binding, b"owner").unwrap();

        remove_installation_state(&dir).unwrap();
        assert!(!dir.join("installation.id").exists(), "Installation ID أُزيل");
        assert!(!legacy_binding.exists(), "ربط الهوية القديم أُزيل");
        assert!(dir.join("mawasib.db").exists(), "بيانات المستخدم محفوظة");
        std::fs::remove_dir_all(&dir).ok();
    }
}
