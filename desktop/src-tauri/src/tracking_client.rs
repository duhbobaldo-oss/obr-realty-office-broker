//! عميل HTTP لخدمة تتبع روابط الإعلان الخارجية (OBR Track).
//! العنوان يُحدَّد من بيئة التشغيل/البناء أو الافتراضي العام — ليس سرًا.
//!
//! قرار المنتج (Desktop = untrusted public client):
//! لا يحمل التطبيق أي Secret خاص بالخادم. التطبيق يحدد نفسه بـ
//! `installation_id` + `product_id` (تحديد وليس مصادقة)، والخادم يطبّق
//! التحقق من المدخلات، وحدود المعدل، والحصص لكل تثبيت.
//! الأسرار (إن وجدت في الخدمة) تبقى على الخادم حصرًا.
//!
//! مسارات الخدمة (v2 — بلا مفتاح):
//! - GET   /health
//! - POST  /v2/links                       Body: { destination_url, installation_id, product_id }
//! - PATCH /v2/links/{code}                Body: { original_ad_url?, enabled? }
//! - GET   /v2/links/{code}/stats
//! - GET   /t/{code}                       إعادة توجيه عامة (بلا هوية)

use serde::Deserialize;
use serde_json::json;

pub const MSG_UNAVAILABLE: &str = "خدمة تتبع الروابط غير متاحة حاليًا.";
pub const MSG_SYNC_FAIL: &str = "تعذر تحديث إحصاءات التتبع";
pub const MSG_CREATE_FAIL: &str = "تعذر إنشاء رابط التتبع. حاول مرة أخرى لاحقًا.";
pub const MSG_UPDATE_FAIL: &str = "تعذر تحديث رابط التتبع.";

/// عنوان الإنتاج العام (ليس سرًا).
pub const DEFAULT_ENDPOINT: &str = "https://obr-track.obr432.workers.dev";

pub const PRODUCT_ID: &str = "obr-office-broker-realty";

#[derive(Debug, Clone, Deserialize)]
pub struct TrackingStats {
    #[serde(default)]
    pub tracked_clicks: i64,
    #[serde(default)]
    pub unique_clicks: i64,
    #[serde(default)]
    pub last_click_at: Option<String>,
    #[serde(default = "default_enabled_true")]
    pub enabled: bool,
}

fn default_enabled_true() -> bool {
    true
}

/// نتيجة إنشاء رابط تتبع.
#[derive(Debug, Clone)]
pub struct RegisteredLink {
    pub tracking_code: String,
    pub tracking_url: String,
}

#[derive(Debug, Deserialize)]
struct CreateLinkResponse {
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    tracking_code: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    tracking_url: Option<String>,
    #[serde(default)]
    short_url: Option<String>,
    #[serde(default)]
    link: Option<String>,
}

fn normalize_base(base: &str) -> Result<String, String> {
    let b = base.trim().trim_end_matches('/').to_string();
    if b.is_empty() {
        return Err(MSG_UNAVAILABLE.into());
    }
    let lower = b.to_ascii_lowercase();
    if !(lower.starts_with("https://") || lower.starts_with("http://")) {
        return Err(MSG_UNAVAILABLE.into());
    }
    // الإنتاج التجاري: HTTPS فقط (loopback مسموح في نسخ التطوير)
    #[cfg(not(debug_assertions))]
    {
        let loopback = lower.contains("127.0.0.1") || lower.contains("localhost");
        if !lower.starts_with("https://") && !loopback {
            return Err(MSG_UNAVAILABLE.into());
        }
    }
    Ok(b)
}

/// مصدر العنوان: متغير تشغيل → مضمَّن عند البناء → الافتراضي العام.
pub fn resolve_endpoint_base() -> Result<String, String> {
    if let Ok(env) = std::env::var("OBR_TRACKING_ENDPOINT") {
        let t = env.trim();
        if !t.is_empty() {
            return normalize_base(t);
        }
    }
    if let Some(baked) = option_env!("OBR_TRACKING_ENDPOINT") {
        let t = baked.trim();
        if !t.is_empty() {
            return normalize_base(t);
        }
    }
    normalize_base(DEFAULT_ENDPOINT)
}

/// هوية التثبيت المرسلة للخدمة: تحديد لحدود الاستخدام — ليست مصادقة.
pub fn valid_installation_id(value: &str) -> bool {
    let t = value.trim();
    !t.is_empty() && t.len() <= 64 && t.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// ترويسة تحديد التثبيت — تُرسل مع كل طلبات الإدارة.
fn identification_headers(req: ureq::Request, installation_id: &str) -> ureq::Request {
    req.set("X-OBR-Installation", installation_id.trim())
        .set("X-OBR-Product", PRODUCT_ID)
}

pub fn is_tracking_configured() -> bool {
    resolve_endpoint_base().is_ok()
}

pub fn public_tracking_url(base: &str, code: &str) -> String {
    format!("{}/t/{}", base.trim_end_matches('/'), code)
}

fn code_from_tracking_url(url: &str) -> Option<String> {
    let trimmed = url.trim().trim_end_matches('/');
    let marker = "/t/";
    if let Some(idx) = trimmed.rfind(marker) {
        let code = &trimmed[idx + marker.len()..];
        if !code.is_empty() && !code.contains('/') {
            return Some(code.to_string());
        }
    }
    None
}

fn registered_from_response(base: &str, parsed: CreateLinkResponse) -> Result<RegisteredLink, String> {
    let tracking_url = parsed
        .tracking_url
        .or(parsed.url)
        .or(parsed.short_url)
        .or(parsed.link)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let tracking_code = parsed
        .tracking_code
        .or(parsed.code)
        .or(parsed.id)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| tracking_url.as_deref().and_then(code_from_tracking_url));

    let tracking_code = tracking_code.ok_or_else(|| MSG_CREATE_FAIL.to_string())?;
    let tracking_url = tracking_url.unwrap_or_else(|| public_tracking_url(base, &tracking_code));
    Ok(RegisteredLink {
        tracking_code,
        tracking_url,
    })
}

/// إنشاء رابط تتبع عبر `POST /v2/links`. الخدمة تولّد الرمز.
/// الهوية: installation_id + product_id — بلا أي مفتاح.
pub fn register_link(base: &str, installation_id: &str, destination_url: &str) -> Result<RegisteredLink, String> {
    let base = normalize_base(base)?;
    if !valid_installation_id(installation_id) {
        return Err(MSG_CREATE_FAIL.into());
    }
    let destination_url = require_https_destination(destination_url)?;
    let url = format!("{base}/v2/links");
    let body = json!({
        "destination_url": destination_url,
        "installation_id": installation_id.trim(),
        "product_id": PRODUCT_ID
    });
    let resp = identification_headers(
        ureq::post(&url)
            .set("Content-Type", "application/json")
            .timeout(std::time::Duration::from_secs(12)),
        installation_id,
    )
    .send_json(body)
    .map_err(|_| MSG_CREATE_FAIL.to_string())?;
    let status = resp.status();
    if !(200..300).contains(&status) {
        return Err(MSG_CREATE_FAIL.into());
    }
    let parsed: CreateLinkResponse = resp
        .into_json()
        .map_err(|_| MSG_CREATE_FAIL.to_string())?;
    registered_from_response(&base, parsed)
}

/// تعطيل/تمكين الرابط عبر `PATCH /v2/links/{code}`.
pub fn set_link_enabled(base: &str, installation_id: &str, code: &str, enabled: bool) -> Result<(), String> {
    let base = normalize_base(base)?;
    if !valid_installation_id(installation_id) {
        return Err(MSG_UPDATE_FAIL.into());
    }
    let code = code.trim();
    if code.is_empty() {
        return Err(MSG_UPDATE_FAIL.into());
    }
    let url = format!("{base}/v2/links/{code}");
    let body = json!({ "enabled": enabled });
    let resp = identification_headers(
        ureq::patch(&url)
            .set("Content-Type", "application/json")
            .timeout(std::time::Duration::from_secs(12)),
        installation_id,
    )
    .send_json(body)
    .map_err(|_| MSG_UPDATE_FAIL.to_string())?;
    if (200..300).contains(&resp.status()) {
        Ok(())
    } else {
        Err(MSG_UPDATE_FAIL.into())
    }
}

/// تحديث الوجهة عبر `PATCH /v2/links/{code}` — دون إعادة إنشاء الرابط.
pub fn update_destination(
    base: &str,
    installation_id: &str,
    code: &str,
    original_ad_url: &str,
) -> Result<RegisteredLink, String> {
    let base = normalize_base(base)?;
    if !valid_installation_id(installation_id) {
        return Err(MSG_UPDATE_FAIL.into());
    }
    let destination_url = require_https_destination(original_ad_url)?;
    let code = code.trim();
    if code.is_empty() {
        return Err(MSG_UPDATE_FAIL.into());
    }
    let url = format!("{base}/v2/links/{code}");
    let body = json!({ "original_ad_url": destination_url });
    let resp = identification_headers(
        ureq::patch(&url)
            .set("Content-Type", "application/json")
            .timeout(std::time::Duration::from_secs(12)),
        installation_id,
    )
    .send_json(body)
    .map_err(|_| MSG_UPDATE_FAIL.to_string())?;
    let status = resp.status();
    if !(200..300).contains(&status) {
        return Err(MSG_UPDATE_FAIL.into());
    }
    // الاستجابة قد تُعيد الرابط الكامل أو `{ ok: true }` فقط — الرمز معروف لدينا
    let parsed: serde_json::Value = resp.into_json().map_err(|_| MSG_UPDATE_FAIL.to_string())?;
    let response: CreateLinkResponse = serde_json::from_value(parsed).unwrap_or(CreateLinkResponse {
        code: None,
        tracking_code: None,
        id: None,
        url: None,
        tracking_url: None,
        short_url: None,
        link: None,
    });
    match registered_from_response(&base, response) {
        Ok(registered) => Ok(registered),
        Err(_) => Ok(RegisteredLink {
            tracking_code: code.to_string(),
            tracking_url: public_tracking_url(&base, code),
        }),
    }
}

pub fn fetch_stats(base: &str, installation_id: &str, code: &str) -> Result<TrackingStats, String> {
    let base = normalize_base(base)?;
    if !valid_installation_id(installation_id) {
        return Err(MSG_SYNC_FAIL.into());
    }
    let code = code.trim();
    if code.is_empty() {
        return Err(MSG_SYNC_FAIL.into());
    }

    let url = format!("{base}/v2/links/{code}/stats");
    let req = identification_headers(
        ureq::get(&url).timeout(std::time::Duration::from_secs(12)),
        installation_id,
    );
    match req.call() {
        Ok(resp) => {
            let status = resp.status();
            if (200..300).contains(&status) {
                return resp
                    .into_json::<TrackingStats>()
                    .map_err(|_| MSG_SYNC_FAIL.to_string());
            }
            Err(MSG_SYNC_FAIL.into())
        }
        Err(_) => Err(MSG_SYNC_FAIL.into()),
    }
}

pub fn generate_tracking_code() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 24];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

pub fn require_https_destination(url: &str) -> Result<String, String> {
    let t = url.trim();
    let lower = t.to_ascii_lowercase();
    if !lower.starts_with("https://") {
        return Err("أدخل رابط إعلان يبدأ بـ https://".into());
    }
    if lower.starts_with("javascript:") || lower.starts_with("data:") || lower.contains('\n') {
        return Err("رابط الإعلان غير مسموح.".into());
    }
    Ok(t.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracking_code_is_unguessable_length() {
        let a = generate_tracking_code();
        let b = generate_tracking_code();
        assert_eq!(a.len(), 48);
        assert_ne!(a, b);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn https_required_for_destination() {
        assert!(require_https_destination("https://sa.bayut.com/x").is_ok());
        assert!(require_https_destination("http://example.com").is_err());
    }

    #[test]
    fn default_endpoint_is_cloudflare() {
        let _lock = ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("OBR_TRACKING_ENDPOINT");
        let base = resolve_endpoint_base().expect("default endpoint");
        assert_eq!(base, DEFAULT_ENDPOINT);
        assert!(base.starts_with("https://"));
        assert!(!base.contains("127.0.0.1"));
        assert!(!base.contains("localhost"));
    }

    #[test]
    fn public_url_shape() {
        let u = public_tracking_url("https://obr-track.obr432.workers.dev/", "abc");
        assert_eq!(u, "https://obr-track.obr432.workers.dev/t/abc");
    }

    #[test]
    fn normalize_rejects_empty() {
        assert!(normalize_base("").is_err());
        assert_eq!(normalize_base("").unwrap_err(), MSG_UNAVAILABLE);
    }

    #[test]
    fn parse_create_response_variants() {
        let base = DEFAULT_ENDPOINT;
        let a = registered_from_response(
            base,
            CreateLinkResponse {
                code: Some("abcd".into()),
                tracking_code: None,
                id: None,
                url: None,
                tracking_url: None,
                short_url: None,
                link: None,
            },
        )
        .unwrap();
        assert_eq!(a.tracking_code, "abcd");
        assert_eq!(a.tracking_url, "https://obr-track.obr432.workers.dev/t/abcd");

        let b = registered_from_response(
            base,
            CreateLinkResponse {
                code: None,
                tracking_code: None,
                id: None,
                url: Some("https://obr-track.obr432.workers.dev/t/zz99".into()),
                tracking_url: None,
                short_url: None,
                link: None,
            },
        )
        .unwrap();
        assert_eq!(b.tracking_code, "zz99");
        assert_eq!(b.tracking_url, "https://obr-track.obr432.workers.dev/t/zz99");
    }

    #[test]
    fn no_secret_material_in_client() {
        // البروتوكول لا يقرأ OBR_API_KEY من أي مصدر — التحديد بلا مصادقة
        let _lock = ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("OBR_API_KEY", "should-be-ignored");
        let base = resolve_endpoint_base().expect("endpoint must resolve without any key");
        assert_eq!(base, DEFAULT_ENDPOINT);
        // البروتوكول بلا مفتاح: التهيئة تعتمد على العنوان فقط
        assert!(is_tracking_configured());
        std::env::remove_var("OBR_API_KEY");
    }

    #[test]
    fn installation_id_validation() {
        assert!(valid_installation_id("7f0d1c3e-5a2b-4c6d-8e9f-0a1b2c3d4e5f"));
        assert!(valid_installation_id("obr-install_1"));
        assert!(!valid_installation_id(""));
        assert!(!valid_installation_id("مع header injection\n"));
        assert!(!valid_installation_id(&"x".repeat(65)));
    }

    /// اختبار حي ضد الخدمة المحلية (v2 بلا مفتاح):
    /// `cargo test live_v2_tracking_loop -- --ignored --nocapture`
    /// مع تشغيل: `cd tracking-endpoint && npm start`
    #[test]
    #[ignore]
    fn live_v2_tracking_loop() {
        let _lock = ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("OBR_TRACKING_ENDPOINT", "http://127.0.0.1:8787");
        let base = resolve_endpoint_base().expect("local endpoint");
        let installation = "7f0d1c3e-5a2b-4c6d-8e9f-0a1b2c3d4e5f";

        let created = register_link(&base, installation, "https://example.com/obr-v2-live").expect("create link");
        assert!(created.tracking_url.contains("/t/"));
        assert!(!created.tracking_code.is_empty());

        // فتح الرابط العام → 302 (بدون متابعة التحويل)
        let no_follow = ureq::AgentBuilder::new().redirects(0).build();
        let status = match no_follow
            .get(&created.tracking_url)
            .timeout(std::time::Duration::from_secs(10))
            .call()
        {
            Ok(resp) => resp.status(),
            Err(ureq::Error::Status(code, _)) => code,
            Err(e) => panic!("open tracking link: {e}"),
        };
        assert!(status >= 300 && status < 400, "redirect expected, got {status}");

        // الإحصاءات تعكس النقرة
        let stats = fetch_stats(&base, installation, &created.tracking_code).expect("stats");
        assert!(stats.tracked_clicks >= 1, "click must be counted");

        // تحديث الوجهة عبر PATCH ثم تعطيل ثم تمكين
        update_destination(&base, installation, &created.tracking_code, "https://example.com/obr-v2-updated").expect("update");
        set_link_enabled(&base, installation, &created.tracking_code, false).expect("disable");
        let stats = fetch_stats(&base, installation, &created.tracking_code).expect("stats after disable");
        assert!(!stats.enabled);
        set_link_enabled(&base, installation, &created.tracking_code, true).expect("enable");

        std::env::remove_var("OBR_TRACKING_ENDPOINT");
    }
}

#[cfg(test)]
pub(crate) static ENV_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
