//! قراءة بلاطات الخرائط المحلية فقط — Offline بالكامل.
//! مصادر الجذر بالترتيب: متغير OBR_MAP_ROOT (تطوير)، جذر وقت التشغيل المحدد
//! عند الإقلاع (AppData\maps ثم مجلد resources المضمّن مع التثبيت)، ثم
//! C:\Projects\ROI\Mapbox (بيئة التطوير). لا يُفتح أي مسار خارج القائمة المسموحة.

use serde::Serialize;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

const MAX_RANGE: u64 = 2_000_000;

const DEFAULT_ROOT: &str = r"C:\Projects\ROI\Mapbox";

/// (المعرّف، المسار النسبي داخل جذر ROI للتطوير)
const ASSETS: &[(&str, &str)] = &[
    ("umaps-b", "tiles/basemap/umaps-vector-B.pmtiles"),
    ("saudi-basemap", "tiles/basemap/saudi.pmtiles"),
];

/// أسماء الملفات المسطحة داخل resources/maps المضمّنة مع التثبيت
const FLAT_FILES: &[(&str, &str)] = &[("umaps-b", "umaps-vector-B.pmtiles")];

#[derive(Debug, Clone)]
pub struct MapRootCandidate {
    pub path: PathBuf,
    /// بنية ROI (tiles/basemap/...) أم مسطحة (اسم ملف مباشر)
    pub flat: bool,
}

static RUNTIME_ROOTS: RwLock<Vec<MapRootCandidate>> = RwLock::new(Vec::new());

/// يُستدعى عند إقلاع التطبيق: جذر resources المضمّن + AppData\maps
pub fn set_runtime_roots(roots: Vec<PathBuf>) {
    if let Ok(mut guard) = RUNTIME_ROOTS.write() {
        *guard = roots
            .into_iter()
            .map(|path| MapRootCandidate { path, flat: true })
            .collect();
    }
}

fn candidate_roots() -> Vec<MapRootCandidate> {
    let mut roots: Vec<MapRootCandidate> = Vec::new();
    // 1) تطوير: متغير بيئة صريح ببنية ROI
    if let Ok(env_root) = std::env::var("OBR_MAP_ROOT") {
        if !env_root.trim().is_empty() {
            roots.push(MapRootCandidate { path: PathBuf::from(env_root), flat: false });
        }
    }
    // 2) جذور وقت التشغيل (AppData\maps ثم resources) — بنية مسطحة
    if let Ok(guard) = RUNTIME_ROOTS.read() {
        roots.extend(guard.iter().cloned());
    }
    // 3) جذر بيئة التطوير الافتراضي (بنية ROI)
    roots.push(MapRootCandidate { path: PathBuf::from(DEFAULT_ROOT), flat: false });
    roots
}

#[derive(Serialize)]
pub struct MapAssetStatus {
    pub id: String,
    pub available: bool,
    pub path: String,
    pub bytes: Option<u64>,
}

#[derive(Serialize)]
pub struct MapAssetsReport {
    pub root: String,
    pub assets: Vec<MapAssetStatus>,
}

fn resolve_asset_in_roots(asset_id: &str, roots: &[MapRootCandidate]) -> Result<PathBuf, String> {
    let rel_roi = ASSETS
        .iter()
        .find(|(id, _)| *id == asset_id)
        .map(|(_, rel)| *rel)
        .ok_or_else(|| "مصدر خريطة غير معروف".to_string())?;
    let flat_name = FLAT_FILES
        .iter()
        .find(|(id, _)| *id == asset_id)
        .map(|(_, name)| *name);

    for root in roots {
        let candidate = if root.flat {
            match flat_name {
                Some(name) => root.path.join(name),
                None => continue,
            }
        } else {
            root.path.join(rel_roi)
        };
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err("ملف الخريطة المحلي غير موجود على هذا الجهاز".to_string())
}

pub fn resolve_asset_path(asset_id: &str) -> Result<PathBuf, String> {
    let path = resolve_asset_in_roots(asset_id, &candidate_roots())?;
    // ضمان بقاء المسار داخل أحد الجذور المسموحة (حماية من التلاعب بالمعرف مسبقًا في المطابقة)
    let allowed = candidate_roots()
        .iter()
        .any(|root| path.starts_with(&root.path));
    if !allowed {
        return Err("مسار الخريطة خارج المجلد المسموح".to_string());
    }
    Ok(path)
}

pub fn map_root() -> PathBuf {
    std::env::var("OBR_MAP_ROOT")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROOT))
}

pub fn assets_report() -> MapAssetsReport {
    let roots = candidate_roots();
    let assets = ASSETS
        .iter()
        .map(|(id, _)| match resolve_asset_in_roots(id, &roots) {
            Ok(path) => {
                let meta = std::fs::metadata(&path).ok();
                MapAssetStatus {
                    id: (*id).to_string(),
                    available: meta.as_ref().map(|m| m.is_file()).unwrap_or(false),
                    path: path.display().to_string(),
                    bytes: meta.map(|m| m.len()),
                }
            }
            Err(_) => MapAssetStatus {
                id: (*id).to_string(),
                available: false,
                path: String::new(),
                bytes: None,
            },
        })
        .collect();
    MapAssetsReport {
        root: roots
            .first()
            .map(|r| r.path.display().to_string())
            .unwrap_or_default(),
        assets,
    }
}

pub fn read_range(asset_id: &str, offset: u64, length: u64) -> Result<Vec<u8>, String> {
    if length == 0 {
        return Ok(Vec::new());
    }
    if length > MAX_RANGE {
        return Err("طلب بلاطة أكبر من المسموح".to_string());
    }
    let path = resolve_asset_path(asset_id)?;
    read_file_range(&path, offset, length)
}

fn read_file_range(path: &Path, offset: u64, length: u64) -> Result<Vec<u8>, String> {
    let mut file = File::open(path).map_err(|e| format!("تعذر فتح ملف الخريطة: {e}"))?;
    let size = file
        .metadata()
        .map_err(|e| format!("تعذر قراءة حجم ملف الخريطة: {e}"))?
        .len();
    if offset > size {
        return Err("إزاحة البلاطة خارج الملف".to_string());
    }
    let remaining = size - offset;
    let take = remaining.min(length) as usize;
    file.seek(SeekFrom::Start(offset))
        .map_err(|e| format!("تعذر قراءة ملف الخريطة: {e}"))?;
    let mut buf = vec![0u8; take];
    file.read_exact(&mut buf)
        .map_err(|e| format!("تعذر قراءة بلاطة الخريطة: {e}"))?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_asset_resolves() {
        let path = resolve_asset_path("umaps-b").expect("umaps-b");
        assert!(path.ends_with("umaps-vector-B.pmtiles"));
    }

    #[test]
    fn unknown_asset_rejected() {
        assert!(resolve_asset_path("../secrets").is_err());
        assert!(resolve_asset_path("umaps-b/../../x").is_err());
    }

    #[test]
    fn range_too_large_rejected() {
        let err = read_range("umaps-b", 0, MAX_RANGE + 1).unwrap_err();
        assert!(err.contains("أكبر"));
    }

    #[test]
    fn flat_resource_layout_resolves() {
        let tmp = std::env::temp_dir().join(format!("obr_map_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("umaps-vector-B.pmtiles"), b"PM").unwrap();
        let roots = vec![MapRootCandidate { path: tmp.clone(), flat: true }];
        let path = resolve_asset_in_roots("umaps-b", &roots).expect("flat resolve");
        assert!(path.starts_with(&tmp));
        assert!(path.ends_with("umaps-vector-B.pmtiles"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn missing_everywhere_reports_friendly_error() {
        let tmp = std::env::temp_dir().join(format!("obr_map_empty_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();
        let roots = vec![MapRootCandidate { path: tmp.clone(), flat: true }];
        let err = resolve_asset_in_roots("umaps-b", &roots).unwrap_err();
        assert!(err.contains("غير موجود"));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
