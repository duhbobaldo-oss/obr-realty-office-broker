//! كاش بلاطات الخريطة الذي يديره OBR — Online-first + Offline Cache.
//! - المسار: %APPDATA%\com.mawasib.obr\map-cache\ (يليه tiles/ للبلاطات وmanifest.json للفهرس)
//! - التدفق لكل بلاطة: كاش محلي أولاً → غير موجودة والإنترنت متاح: تنزيل ثم خزن ثم عرض
//!   → قطع الاتصال: خطأ صديق (تُعرض بلاطة فارغة محايدة في JS دون كسر الخريطة)
//! - حد الكاش 1.5GB مع تنظيف LRU
//! - الأمان: https فقط + قائمة مضيفين مسموحين صريحة (لا يعمل هذا الأمر كمُلقي عام)
//! - بيانات العقارات والعملاء لا تمر من هنا إطلاقًا: الطلبات تحتوي إحداثيات بلاطة x/y/z فقط

use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::sync::RwLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// المضيفون المسموح تنزيل البلاطات منهم فقط (https صريح)
pub const ALLOWED_HOSTS: &[&str] = &["tiles.openfreemap.org"];

/// حد الكاش: 1.5GB (ضمن نطاق 1–2GB المطلوب)
pub const CACHE_LIMIT_BYTES: u64 = 1_500_000_000;

/// سقف البلاطة الواحدة (البلاطات الفعّالة 5–50KB)
pub const MAX_TILE_BYTES: usize = 2_000_000;

const MANIFEST_NAME: &str = "manifest.json";
const TILES_DIR: &str = "tiles";

#[derive(Debug, Clone)]
struct CacheEntry {
    url: String,
    size: u64,
    /// آخر وصول (ms منذ الإيوبك) — في الذاكرة، ويُثبَّت مع الإضافات
    accessed: u128,
}

#[derive(Debug, Default)]
struct Runtime {
    root: Option<PathBuf>,
    manifest: HashMap<String, CacheEntry>,
}

static RUNTIME: RwLock<Option<Runtime>> = RwLock::new(None);

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// تجزئة FNV-1a 64 بت — تسمية ملفات حتمية بدون اعتماديات إضافية
fn fnv64a(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// التحقق من الرابط: https فقط ومضيف ضمن القائمة المسموحة
pub fn validate_url(url: &str) -> Result<(String, String), String> {
    let text = url.trim();
    let lower = text.to_lowercase();
    if !(lower.starts_with("https://")) {
        return Err("رابط البلاطة يجب أن يكون https".into());
    }
    let rest = &text["https://".len()..];
    let host = rest
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("")
        .trim()
        .to_lowercase();
    if host.is_empty() {
        return Err("رابط البلاطة بلا مضيف".into());
    }
    if !ALLOWED_HOSTS.contains(&host.as_str()) {
        return Err("مضيف البلاطة غير ضمن القائمة المسموحة".into());
    }
    Ok((host, text.to_string()))
}

pub fn set_runtime_root(app_data_dir: &PathBuf) {
    let root = app_data_dir.join("map-cache");
    let _ = fs::create_dir_all(root.join(TILES_DIR));
    let manifest = load_manifest(&root);
    if let Ok(mut guard) = RUNTIME.write() {
        *guard = Some(Runtime {
            root: Some(root),
            manifest,
        });
    }
}

fn manifest_path(root: &PathBuf) -> PathBuf {
    root.join(MANIFEST_NAME)
}

fn load_manifest(root: &PathBuf) -> HashMap<String, CacheEntry> {
    let mut out = HashMap::new();
    let Ok(text) = fs::read_to_string(manifest_path(root)) else {
        return out;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return out;
    };
    if let Some(entries) = value.get("entries").and_then(|e| e.as_object()) {
        for (hash, item) in entries {
            let url = item.get("url").and_then(|u| u.as_str()).unwrap_or("").to_string();
            let size = item.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
            let accessed = item
                .get("accessed")
                .and_then(|a| a.as_u64())
                .unwrap_or(0) as u128;
            if url.is_empty() || size == 0 {
                continue;
            }
            out.insert(
                hash.clone(),
                CacheEntry {
                    url,
                    size,
                    accessed,
                },
            );
        }
    }
    out
}

fn save_manifest(root: &PathBuf, manifest: &HashMap<String, CacheEntry>) {
    let mut entries = serde_json::Map::new();
    for (hash, entry) in manifest {
        entries.insert(
            hash.clone(),
            serde_json::json!({
                "url": entry.url,
                "size": entry.size,
                "accessed": entry.accessed as u64,
            }),
        );
    }
    let value = serde_json::json!({
        "version": 1,
        "limit_bytes": CACHE_LIMIT_BYTES,
        "entries": entries,
    });
    let path = manifest_path(root);
    let tmp = root.join(".manifest.tmp");
    if fs::write(&tmp, serde_json::to_string(&value).unwrap_or_default()).is_ok() {
        let _ = fs::rename(&tmp, &path);
    }
}

fn tile_file(root: &PathBuf, url: &str) -> PathBuf {
    let hash = fnv64a(url);
    root.join(TILES_DIR).join(format!("{hash:016x}.bin"))
}

/// تصنيف أخطاء التنزيل إلى أخطاء صديقة (JS يتحول بها إلى بلاطة فارغة)
fn classify_download_error(err: ureq::Error) -> String {
    match err {
        ureq::Error::Status(code, _) => format!("MAP_HTTP_{code}"),
        ureq::Error::Transport(t) => format!("MAP_OFFLINE: {t}"),
    }
}

fn download(url: &str) -> Result<Vec<u8>, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(8))
        .timeout(Duration::from_secs(25))
        .build();
    let resp = agent
        .get(url)
        .set("User-Agent", "RealtyOffice/0.1 (+desktop)")
        .call()
        .map_err(classify_download_error)?;
    let len = resp
        .header("Content-Length")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    if len > MAX_TILE_BYTES {
        return Err("MAP_TOO_LARGE: بلاطة أكبر من المسموح".into());
    }
    let mut buf = Vec::new();
    resp.into_reader()
        .take((MAX_TILE_BYTES + 1) as u64)
        .read_to_end(&mut buf)
        .map_err(|e| format!("MAP_READ: {e}"))?;
    if buf.len() > MAX_TILE_BYTES {
        return Err("MAP_TOO_LARGE: بلاطة أكبر من المسموح".into());
    }
    Ok(buf)
}

/// إزالة الأقدم استعمالًا حتى يعود الإجمالي تحت الحد (LRU)
fn evict_lru(root: &PathBuf, manifest: &mut HashMap<String, CacheEntry>, reserve: u64) {
    let total: u64 = manifest.values().map(|e| e.size).sum();
    if total + reserve <= CACHE_LIMIT_BYTES {
        return;
    }
    let mut by_age: Vec<(String, u128)> = manifest
        .iter()
        .map(|(hash, e)| (hash.clone(), e.accessed))
        .collect();
    by_age.sort_by_key(|(_, accessed)| *accessed);
    let mut current = total;
    for (hash, _) in by_age {
        if current + reserve <= CACHE_LIMIT_BYTES * 95 / 100 {
            break;
        }
        let size = manifest.get(&hash).map(|e| e.size).unwrap_or(0);
        let url = manifest.get(&hash).map(|e| e.url.clone()).unwrap_or_default();
        if !url.is_empty() {
            let _ = fs::remove_file(tile_file(root, &url));
        }
        if manifest.remove(&hash).is_some() {
            current = current.saturating_sub(size);
        }
    }
}

/// جلب بلاطة: كاش محلي أولاً → تنزيل وخزن → خطأ صديق عند القطع
pub fn fetch_tile(url: &str) -> Result<Vec<u8>, String> {
    let (_host, clean) = validate_url(url)?;
    let snapshot_root = {
        let guard = RUNTIME.read().map_err(|_| "قفل الكاش مشغول")?;
        let runtime = guard.as_ref().ok_or("كاش الخريطة غير مهيأ")?;
        runtime.root.clone().ok_or("كاش الخريطة غير مهيأ")?
    };
    let path = tile_file(&snapshot_root, &clean);
    // 1) الكاش المحلي أولاً — عرض فوري
    if let Ok(bytes) = fs::read(&path) {
        let ms = now_ms();
        if let Ok(mut guard) = RUNTIME.write() {
            if let Some(runtime) = guard.as_mut() {
                if let Some(entry) = runtime.manifest.get_mut(&fnv64a_hex(&clean)) {
                    entry.accessed = ms;
                }
            }
        }
        return Ok(bytes);
    }
    // 2) غير موجودة: تنزيل من المصدر
    let bytes = download(&clean)?;
    let size = bytes.len() as u64;
    // ضمان وجود المجلد دائمًا (قد يُحذف يدويًا أو بواسطة صيانة خارجية)
    let _ = fs::create_dir_all(snapshot_root.join(TILES_DIR));
    fs::write(&path, &bytes).map_err(|e| format!("MAP_WRITE: {e}"))?;
    // 3) تحديث الفهرس + LRU
    if let Ok(mut guard) = RUNTIME.write() {
        if let Some(runtime) = guard.as_mut() {
            runtime.manifest.insert(
                fnv64a_hex(&clean),
                CacheEntry {
                    url: clean.clone(),
                    size,
                    accessed: now_ms(),
                },
            );
            evict_lru(&runtime.root.clone().unwrap_or_default(), &mut runtime.manifest, 0);
            save_manifest(&runtime.root.clone().unwrap_or_default(), &runtime.manifest);
        }
    }
    Ok(bytes)
}

fn fnv64a_hex(url: &str) -> String {
    format!("{:016x}", fnv64a(url))
}

#[derive(Serialize)]
pub struct CacheStatus {
    pub root: String,
    pub entries: usize,
    pub bytes: u64,
    pub limit_bytes: u64,
}

/// حالة الكاش للعرض/التحقق
pub fn status() -> Result<CacheStatus, String> {
    let guard = RUNTIME.read().map_err(|_| "قفل الكاش مشغول")?;
    let runtime = guard.as_ref().ok_or("كاش الخريطة غير مهيأ")?;
    let root = runtime.root.clone().ok_or("كاش الخريطة غير مهيأ")?;
    let bytes = runtime.manifest.values().map(|e| e.size).sum();
    Ok(CacheStatus {
        root: root.display().to_string(),
        entries: runtime.manifest.len(),
        bytes,
        limit_bytes: CACHE_LIMIT_BYTES,
    })
}

/// تفريغ الكاش بالكامل (صيانة يدوية)
pub fn clear_all() -> Result<usize, String> {
    let mut guard = RUNTIME.write().map_err(|_| "قفل الكاش مشغول")?;
    let runtime = guard.as_mut().ok_or("كاش الخريطة غير مهيأ")?;
    let root = runtime.root.clone().ok_or("كاش الخريطة غير مهيأ")?;
    let deleted = runtime.manifest.len();
    for entry in runtime.manifest.values() {
        let _ = fs::remove_file(tile_file(&root, &entry.url));
    }
    runtime.manifest.clear();
    save_manifest(&root, &runtime.manifest);
    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// الاختبارات تتشارك حالة RUNTIME العالمية — تُنفَّذ بالتتابع عبر قفل واحد
    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn temp_root() -> PathBuf {
        let tmp = std::env::temp_dir().join(format!("obr_map_cache_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(tmp.join(TILES_DIR)).unwrap();
        tmp
    }

    #[test]
    fn https_and_host_allowlist_enforced() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        assert!(validate_url("https://tiles.openfreemap.org/planet/1/2/3.pbf").is_ok());
        assert!(validate_url("http://tiles.openfreemap.org/planet/1/2/3.pbf").is_err());
        assert!(validate_url("https://evil.example.com/planet/1/2/3.pbf").is_err());
        assert!(validate_url("https://tiles.openfreemap.org.evil.com/x.pbf").is_err());
    }

    #[test]
    fn cache_hit_returns_from_disk_without_network() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let parent = temp_root();
        let root = parent.join("map-cache");
        fs::create_dir_all(root.join(TILES_DIR)).unwrap();
        set_runtime_root(&parent);
        let url = "https://tiles.openfreemap.org/planet/5/11/12.pbf";
        let path = tile_file(&root, url);
        fs::write(&path, b"TILE_BYTES").unwrap();
        let bytes = fetch_tile(url).expect("hit");
        assert_eq!(bytes, b"TILE_BYTES");
        let _ = fs::remove_dir_all(&parent);
    }

    #[test]
    fn miss_without_remote_is_friendly_error() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let root = temp_root();
        set_runtime_root(&root);
        // مسار غير موجود على المضيف المسموح — يجب أن يُصنَّف خطأ HTTP صديقًا (MAP_HTTP_*)
        // أو خطأ قطع صديق (MAP_OFFLINE) — كلاهما صديق للبلاطة الفارغة في JS
        let url = "https://tiles.openfreemap.org/nonexistent-path/1/2/3.pbf";
        let err = fetch_tile(url).unwrap_err();
        assert!(err.starts_with("MAP_"), "friendly error: {err}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn lru_eviction_respects_limit() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let root = temp_root();
        // حد مصغّر عبر إدخال يدوي: نستخدم نفس البنية بنسبة
        set_runtime_root(&root);
        let url = "https://tiles.openfreemap.org/planet/3/4/5.pbf";
        // نعبئ الفهرس يدويًا بإدخالين ثم نضيف فوق الحد الافتراضي عبر نسخة مصغّرة من المنطق
        let ms = now_ms();
        if let Ok(mut guard) = RUNTIME.write() {
            if let Some(runtime) = guard.as_mut() {
                runtime.manifest.insert(
                    fnv64a_hex("https://tiles.openfreemap.org/old/1.pbf"),
                    CacheEntry {
                        url: "https://tiles.openfreemap.org/old/1.pbf".into(),
                        size: CACHE_LIMIT_BYTES - 100,
                        accessed: ms,
                    },
                );
                runtime.manifest.insert(
                    fnv64a_hex("https://tiles.openfreemap.org/new/2.pbf"),
                    CacheEntry {
                        url: "https://tiles.openfreemap.org/new/2.pbf".into(),
                        size: 1000,
                        accessed: ms + 5,
                    },
                );
                evict_lru(&root, &mut runtime.manifest, 1000);
                assert!(
                    !runtime.manifest.contains_key(&fnv64a_hex(
                        "https://tiles.openfreemap.org/old/1.pbf"
                    )),
                    "الأقدم استعمالًا أزيل"
                );
            }
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn status_reports_root_and_entries() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let root = temp_root();
        set_runtime_root(&root);
        let s = status().expect("status");
        assert!(s.root.ends_with("map-cache"));
        assert_eq!(s.limit_bytes, CACHE_LIMIT_BYTES);
        let _ = fs::remove_dir_all(&root);
    }
}
