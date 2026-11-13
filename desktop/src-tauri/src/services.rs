// طبقة الخدمات - منطق الأعمال المستقل عن Tauri وقاعدة البيانات
// يمكن استخدامها لاحقًا في API مركزي أو نسخة جوال

use crate::campaigns::{
    compute_campaign_metrics, is_known_platform, json_nonneg_f64, json_nonneg_i64, json_opt_str,
    metrics_to_json, optional_iso_date, parse_iso_date, validate_ad_link, validate_hhmm,
};
use crate::db::*;
use bcrypt::{hash, verify};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
    // معرّف فريد للجلسة لضمان اختلاف التوكنات حتى لو صدرت في نفس الثانية
    jti: String,
}

// ===== مفتاح JWT لكل تثبيت =====
// يُولَّد عشوائيًا عند أول تشغيل ويُحفظ في مجلد بيانات التطبيق (خارج المصدر وGit)
// ولا يتغير عند إعادة التشغيل حتى لا تُبطل الجلسات بلا سبب.
static AUTH_SECRET: std::sync::OnceLock<String> = std::sync::OnceLock::new();

pub fn init_auth_secret(secret: String) -> Result<(), String> {
    AUTH_SECRET
        .set(secret)
        .map_err(|_| "مفتاح التوثيق مُهيأ مسبقًا".to_string())
}

/// توليد/تحميل مفتاح التوثيق الخاص بهذا التثبيت من مجلد بيانات التطبيق.
pub fn load_or_create_auth_secret(app_data_dir: &std::path::Path) -> Result<String, String> {
    let path = app_data_dir.join("auth-secret.key");
    if path.exists() {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("تعذر قراءة مفتاح التوثيق: {e}"))?;
        let trimmed = content.trim();
        if trimmed.len() >= 32 {
            return Ok(trimmed.to_string());
        }
        // ملف تالف أو قصير: يُعاد توليده. الجلسات القديمة ستصبح غير صالحة — سلوك مقصود للأمان.
    }
    let secret = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    std::fs::write(&path, &secret).map_err(|e| format!("تعذر حفظ مفتاح التوثيق: {e}"))?;
    Ok(secret)
}

fn auth_secret() -> Result<&'static str, String> {
    AUTH_SECRET
        .get()
        .map(|s| s.as_str())
        .ok_or_else(|| "مفتاح التوثيق غير مُهيأ".to_string())
}

// ===== أدوات مساعدة =====

pub fn generate_license_id() -> String {
    let chars = "ABCDEF0123456789";
    let mut result = String::new();
    for _ in 0..6 {
        let idx = rand_index(chars.len());
        result.push(chars.chars().nth(idx).unwrap());
    }
    format!("LIC-{}", result)
}

pub fn generate_license_key() -> String {
    let charset = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut groups: Vec<String> = Vec::new();
    for _ in 0..4 {
        let mut group = String::new();
        for _ in 0..4 {
            let idx = rand_index(charset.len());
            group.push(charset.chars().nth(idx).unwrap());
        }
        groups.push(group);
    }
    format!("RS-{}", groups.join("-"))
}

fn rand_index(len: usize) -> usize {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos() as usize;
    (nanos + Uuid::new_v4().as_u128() as usize) % len
}

pub fn sign_token(user_id: &str) -> Result<String, String> {
    let exp = chrono::Utc::now().timestamp() as usize + 30 * 24 * 60 * 60; // 30 يوم
    let claims = Claims {
        sub: user_id.to_string(),
        exp,
        jti: Uuid::new_v4().to_string(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(auth_secret()?.as_bytes()),
    )
    .map_err(|e| e.to_string())
}

pub fn verify_token(token: &str) -> Result<String, String> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(auth_secret()?.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| "رمز غير صالح أو منتهي".to_string())?;
    Ok(data.claims.sub)
}

/// التحقق الكامل من التوكن: صحة التوقيع + وجود الجلسة فعليًا في قاعدة البيانات
/// + تطابق المستخدم + عدم انتهاء صلاحية الجلسة. الخروج (logout) يحذف الجلسة
/// فيصبح التوكن غير صالح فورًا حتى لو لم تنتهِ صلاحيته الزمنية.
pub fn validate_session_token(db: &dyn Database, token: &str) -> Result<String, String> {
    let user_id = verify_token(token)?;
    let session = db
        .get_session_by_token(token)?
        .ok_or_else(|| "الجلسة غير موجودة أو منتهية. سجّل الدخول من جديد".to_string())?;
    if session.user_id != user_id {
        return Err("الجلسة لا تطابق المستخدم".to_string());
    }
    if let Some(expires_at) = session.expires_at.as_deref() {
        if let Ok(expiry) = chrono::DateTime::parse_from_rfc3339(expires_at) {
            if expiry < chrono::Utc::now() {
                return Err("انتهت صلاحية الجلسة. سجّل الدخول من جديد".to_string());
            }
        }
    }
    Ok(user_id)
}

pub fn hash_password(password: &str) -> Result<String, String> {
    hash(password, 10).map_err(|e| e.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, String> {
    verify(password, hash).map_err(|e| e.to_string())
}

// ===== خدمات المصادقة =====

pub struct AuthService<'a> {
    pub db: &'a dyn Database,
}

impl<'a> AuthService<'a> {
    pub fn new(db: &'a dyn Database) -> Self {
        Self { db }
    }

    pub fn register(
        &self,
        username: &str,
        email: &str,
        password: &str,
        full_name: Option<String>,
        phone: Option<String>,
        office_name: Option<String>,
    ) -> Result<serde_json::Value, String> {
        if username.is_empty() || email.is_empty() || password.is_empty() {
            return Err("اسم المستخدم والبريد وكلمة المرور مطلوبة".to_string());
        }
        if password.len() < 6 {
            return Err("كلمة المرور يجب أن تكون 6 أحرف على الأقل".to_string());
        }

        // التحقق من عدم وجود المستخدم
        if let Some(_) = self.db.get_user_by_username_or_email(username, email)? {
            return Err("اسم المستخدم أو البريد مستخدم بالفعل".to_string());
        }

        let user_id = Uuid::new_v4().to_string();
        let password_hash = hash_password(password)?;

        // عدد المستخدمين لتوليد USER-ID تسلسلي
        let count = self.db.count_users()?;
        let user_id_formatted = format!("USER-{:03}", count + 1);

        let user = User {
            id: user_id.clone(),
            username: username.to_string(),
            email: email.to_string(),
            password_hash,
            full_name,
            phone,
            office_name,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.db.create_user(&user)?;

        // إنشاء الترخيص تلقائيًا
        let license_id = generate_license_id();
        let license_key = generate_license_key();
        let lic_id = Uuid::new_v4().to_string();
        let license = License {
            id: lic_id.clone(),
            user_id: user_id.clone(),
            license_key: license_key.clone(),
            status: "active".to_string(),
            issued_at: chrono::Utc::now().to_rfc3339(),
            expires_at: None,
        };
        self.db.create_license(&license)?;
        // تحديث ID الترخيص إلى LIC-XXXX
        self.db.update_license_id(&license_id, &lic_id)?;

        // إنشاء الإعدادات الافتراضية
        self.db.create_settings(&user_id)?;

        // إنشاء جلسة
        let token = sign_token(&user_id)?;
        let session = Session {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.clone(),
            token: token.clone(),
            device_name: Some("Desktop App".to_string()),
            created_at: chrono::Utc::now().to_rfc3339(),
            expires_at: Some((chrono::Utc::now() + chrono::Duration::days(30)).to_rfc3339()),
        };
        self.db.create_session(&session)?;

        // تسجيل العملية
        self.db.create_audit_log(&AuditLog {
            id: Uuid::new_v4().to_string(),
            user_id: Some(user_id.clone()),
            action: "register".to_string(),
            entity_type: Some("user".to_string()),
            entity_id: None,
            details: Some(format!("تسجيل مستخدم جديد: {}", username)),
            created_at: chrono::Utc::now().to_rfc3339(),
        })?;

        Ok(serde_json::json!({
            "message": "تم إنشاء الحساب بنجاح",
            "user_id": user_id_formatted,
            "user": {
                "id": user_id,
                "username": username,
                "email": email,
                "full_name": user.full_name,
                "phone": user.phone,
                "office_name": user.office_name,
                "user_id_display": user_id_formatted
            },
            "license": {
                "id": license_id,
                "license_key": license_key,
                "status": "active"
            },
            "token": token
        }))
    }

    pub fn login(&self, username: &str, password: &str) -> Result<serde_json::Value, String> {
        if username.is_empty() || password.is_empty() {
            return Err("اسم المستخدم وكلمة المرور مطلوبة".to_string());
        }

        let user = self
            .db
            .get_user_by_username_or_email(username, username)?
            .ok_or("بيانات الدخول غير صحيحة".to_string())?;

        let valid = verify_password(password, &user.password_hash)?;
        if !valid {
            return Err("بيانات الدخول غير صحيحة".to_string());
        }

        let token = sign_token(&user.id)?;
        let session = Session {
            id: Uuid::new_v4().to_string(),
            user_id: user.id.clone(),
            token: token.clone(),
            device_name: Some("Desktop App".to_string()),
            created_at: chrono::Utc::now().to_rfc3339(),
            expires_at: Some((chrono::Utc::now() + chrono::Duration::days(30)).to_rfc3339()),
        };
        self.db.create_session(&session)?;

        let license = self.db.get_license_by_user_id(&user.id)?;

        self.db.create_audit_log(&AuditLog {
            id: Uuid::new_v4().to_string(),
            user_id: Some(user.id.clone()),
            action: "login".to_string(),
            entity_type: Some("user".to_string()),
            entity_id: None,
            details: Some(format!("تسجيل دخول: {}", user.username)),
            created_at: chrono::Utc::now().to_rfc3339(),
        })?;

        Ok(serde_json::json!({
            "token": token,
            "user": {
                "id": user.id,
                "username": user.username,
                "email": user.email,
                "full_name": user.full_name,
                "phone": user.phone,
                "office_name": user.office_name,
                "created_at": user.created_at
            },
            "license": license.map(|l| serde_json::json!({
                "id": l.id,
                "license_key": l.license_key,
                "status": l.status
            }))
        }))
    }

    pub fn logout(&self, token: &str) -> Result<(), String> {
        self.db.delete_session_by_token(token)?;
        Ok(())
    }

    pub fn activate_license(&self, user_id: &str, license_key: &str) -> Result<serde_json::Value, String> {
        if license_key.is_empty() {
            return Err("مفتاح الترخيص مطلوب".to_string());
        }

        let license = self
            .db
            .get_license_by_key(license_key)?
            .ok_or("مفتاح الترخيص غير صالح".to_string())?;

        if license.status != "active" {
            return Err("الترخيص غير نشط".to_string());
        }

        if license.user_id != user_id {
            let existing_user = self.db.get_user_by_id(&license.user_id)?;
            return Err(format!(
                "هذا الترخيص مرتبط بحساب المستخدم: {}",
                existing_user.map(|u| u.username).unwrap_or("غير معروف".to_string())
            ));
        }

        self.db.create_audit_log(&AuditLog {
            id: Uuid::new_v4().to_string(),
            user_id: Some(user_id.to_string()),
            action: "activate_license".to_string(),
            entity_type: Some("license".to_string()),
            entity_id: Some(license.id.clone()),
            details: Some(format!("تفعيل الترخيص: {}", license.id)),
            created_at: chrono::Utc::now().to_rfc3339(),
        })?;

        Ok(serde_json::json!({
            "message": "تم تفعيل الترخيص بنجاح",
            "license": {
                "id": license.id,
                "license_key": license.license_key,
                "status": license.status
            }
        }))
    }

    pub fn me(&self, user_id: &str) -> Result<serde_json::Value, String> {
        let user = self
            .db
            .get_user_by_id(user_id)?
            .ok_or("المستخدم غير موجود".to_string())?;
        let settings = self.db.get_settings(user_id)?;

        Ok(serde_json::json!({
            "user": user,
            "settings": settings
        }))
    }

    /// جلسة محلية مباشرة — لا تفعيل ولا حساب ولا بوابة هوية/ترخيص في هذه النسخة.
    /// فتح قاعدة البيانات يعتمد فقط على: وجودها وصلاحية المخطط والمهاجرات.
    /// ترتيب الاختيار: معرف مرتبط (إن وُجد) → مطابقة البريد → أقدم مستخدم قائم
    /// (تبنّي بلا حذف أي بيانات) → إنشاء مالك جديد لقاعدة فارغة.
    pub fn ensure_local_session(&self, owner_name: &str, owner_email: &str, bound_user_id: Option<&str>) -> Result<serde_json::Value, String> {
        let adopted = if let Some(id) = bound_user_id {
            self.db.get_user_by_id(id)?
        } else {
            None
        };
        let adopted = adopted.or_else(|| self.db.get_user_by_username_or_email("", owner_email).ok().flatten());
        let adopted = match adopted {
            Some(u) => Some(u),
            None => self.db.get_first_user()?,
        };

        let user = match adopted {
            Some(user) => user,
            None => {
                let user_id = Uuid::new_v4().to_string();
                let user = User {
                    id: user_id.clone(),
                    username: "owner".to_string(),
                    email: owner_email.to_string(),
                    password_hash: hash_password(&Uuid::new_v4().to_string())?,
                    full_name: Some(owner_name.to_string()),
                    phone: None,
                    office_name: None,
                    created_at: chrono::Utc::now().to_rfc3339(),
                };
                self.db.create_user(&user)?;
                self.db.create_settings(&user_id)?;
                user
            }
        };

        let token = sign_token(&user.id)?;
        let session = Session {
            id: Uuid::new_v4().to_string(),
            user_id: user.id.clone(),
            token: token.clone(),
            device_name: Some("Desktop App".to_string()),
            created_at: chrono::Utc::now().to_rfc3339(),
            expires_at: Some((chrono::Utc::now() + chrono::Duration::days(30)).to_rfc3339()),
        };
        self.db.create_session(&session)?;

        Ok(serde_json::json!({
            "token": token,
            "user": {
                "id": user.id,
                "username": user.username,
                "email": user.email,
                "full_name": user.full_name,
                "phone": user.phone,
                "office_name": user.office_name,
                "created_at": user.created_at
            }
        }))
    }
}

// ===== خدمات العروض =====

pub const MAX_PROPERTY_IMAGES: i64 = 4;

pub struct PropertyService<'a> {
    pub db: &'a dyn Database,
}

impl<'a> PropertyService<'a> {
    pub fn new(db: &'a dyn Database) -> Self {
        Self { db }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn list(
        &self,
        user_id: &str,
        page: i64,
        limit: i64,
        search: &str,
        status: &str,
        city: &str,
        property_type: &str,
        purpose: &str,
    ) -> Result<serde_json::Value, String> {
        if !purpose.is_empty() && !PROPERTY_PURPOSES.contains(&purpose) {
            return Err(format!("غرض غير معروف: {purpose}"));
        }
        let offset = (page - 1) * limit;
        let (items, total) = self.db.list_properties(user_id, search, status, city, property_type, purpose, limit, offset)?;
        let ids: Vec<String> = items.iter().map(|p| p.id.clone()).collect();
        let counts = self.db.count_images_for_ids(&ids)?;
        let json_items: Vec<serde_json::Value> = items
            .iter()
            .map(|p| {
                let mut obj = property_to_json(p);
                if let Some(map) = obj.as_object_mut() {
                    let n = counts.get(&p.id).copied().unwrap_or(0);
                    map.insert("image_count".to_string(), serde_json::json!(n));
                }
                obj
            })
            .collect();
        Ok(serde_json::json!({ "items": json_items, "total": total, "page": page, "limit": limit }))
    }

    pub fn get(&self, id: &str, user_id: &str) -> Result<serde_json::Value, String> {
        let property = self
            .db
            .get_property(id, user_id)?
            .ok_or("العرض غير موجود".to_string())?;
        let images = self.db.list_property_images(id)?;
        let updates = self.db.list_property_updates(id)?;
        let linked_clients = self.db.list_linked_clients_for_property(id)?;

        // جهة اتصال المالك تُقرأ من السجل الحقيقي، ولا تُنسخ نصياً.
        let owner_contact_record = match property.owner_client_id.as_deref() {
            Some(cid) => self.db.get_client(cid, user_id)?.map(|c| {
                serde_json::json!({ "id": c.id, "name": c.name, "phone": c.phone, "city": c.city })
            }),
            None => None,
        };

        let mut obj = property_to_json(&property);
        if let Some(map) = obj.as_object_mut() {
            map.insert("images".to_string(), serde_json::to_value(images).unwrap_or_default());
            map.insert("updates".to_string(), serde_json::to_value(updates).unwrap_or_default());
            map.insert("linked_clients".to_string(), serde_json::to_value(linked_clients).unwrap_or_default());
            map.insert("owner_contact_record".to_string(), serde_json::to_value(owner_contact_record).unwrap_or(serde_json::Value::Null));
        }
        Ok(obj)
    }

    fn owner_display_fields(
        &self,
        user_id: &str,
        owner_client_id: &Option<String>,
        owner_name: Option<String>,
        owner_phone: Option<String>,
    ) -> Result<(Option<String>, Option<String>), String> {
        let Some(oid) = owner_client_id.as_deref() else {
            return Ok((owner_name, owner_phone));
        };
        let Some(contact) = self.db.get_client(oid, user_id)? else {
            return Ok((owner_name, owner_phone));
        };
        Ok((
            owner_name.or(Some(contact.name)),
            owner_phone.or(contact.phone),
        ))
    }

    /// يتحقق أن جهة اتصال المالك موجودة فعلاً وتابعة لنفس المستخدم.
    /// نص فارغ = فك الارتباط.
    fn resolve_owner_client(&self, user_id: &str, data: &serde_json::Value) -> Result<Option<String>, String> {
        let raw = match data.get("owner_client_id").and_then(|v| v.as_str()) {
            Some(s) => s.trim().to_string(),
            None => return Ok(None),
        };
        if raw.is_empty() {
            return Ok(None);
        }
        if self.db.get_client(&raw, user_id)?.is_none() {
            return Err("جهة اتصال المالك غير موجودة".to_string());
        }
        Ok(Some(raw))
    }

    pub fn create(&self, user_id: &str, data: &serde_json::Value) -> Result<serde_json::Value, String> {
        let current = crate::country::RecordContext::from_settings(&self.db.get_settings(user_id)?)?;
        if data.get("country").and_then(|v|v.as_str()).is_some_and(|v| !v.is_empty() && v != current.country)
            || data.get("currency").and_then(|v|v.as_str()).is_some_and(|v| !v.is_empty() && v != current.currency) {
            return Err("دولة أو عملة البيانات المستوردة تختلف عن الإعدادات؛ اختر دولة البيانات قبل الاستيراد، دون تحويل المبالغ".into());
        }

        let name = data.get("name").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        if name.is_empty() {
            return Err("اسم العقار مطلوب".to_string());
        }
        validate_property_payload(data)?;

        let id = Uuid::new_v4().to_string();
        // الأرقام قد تصل كنص من نماذج HTML؛ json_number يتعامل مع الحالتين.
        let area = json_number(data, "area");
        let price = json_number(data, "price");
        let price_per_meter = json_number(data, "price_per_meter");
        let owner_client_id = self.resolve_owner_client(user_id, data)?;
        let (owner_name, owner_phone) =
            self.owner_display_fields(user_id, &owner_client_id, json_text(data, "owner_name"), json_text(data, "owner_phone"))?;

        let property = Property {
            context: crate::country::RecordContext::default(),
            id: id.clone(),
            user_id: user_id.to_string(),
            name,
            city: json_text(data, "city"),
            district: json_text(data, "district"),
            location: json_text(data, "location"),
            property_type: json_text(data, "property_type"),
            area,
            price,
            price_per_meter,
            ad_number: json_text(data, "ad_number"),
            ad_link: json_text(data, "ad_link"),
            owner_name,
            owner_phone,
            owner_contact: json_text(data, "owner_contact"),
            owner_notes: json_text(data, "owner_notes"),
            marketing_platform: json_text(data, "marketing_platform"),
            marketing_link: json_text(data, "marketing_link"),
            published_date: json_text(data, "published_date"),
            main_image: None,
            status: data.get("status").and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty()).unwrap_or("available").to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            owner_client_id,
            source_type: json_text(data, "source_type"),
            purpose: json_text(data, "purpose"),
            usage_type: json_text(data, "usage_type"),
            frontage: json_text(data, "frontage"),
            street_width: json_number(data, "street_width"),
            streets_count: json_int(data, "streets_count"),
            latitude: json_number(data, "latitude"),
            longitude: json_number(data, "longitude"),
            description: json_text(data, "description"),
            listing_expiry_date: json_text(data, "listing_expiry_date"),
            commission_agreement: json_text(data, "commission_agreement"),
            fal_ad_license: json_text(data, "fal_ad_license"),
            internal_price: json_number(data, "internal_price"),
            internal_notes: json_text(data, "internal_notes"),
            is_training: false,
        };
        self.db.create_property_atomic(
            &property,
            user_id,
            &AuditLog {
                id: Uuid::new_v4().to_string(),
                user_id: Some(user_id.to_string()),
                action: "property_create".to_string(),
                entity_type: Some("property".to_string()),
                entity_id: Some(id.clone()),
                details: Some(format!("إضافة عرض: {}", property.name)),
                created_at: chrono::Utc::now().to_rfc3339(),
            },
        )?;

        let created = self.db.get_property(&id, user_id)?.unwrap();
        Ok(property_to_json(&created))
    }

    pub fn update(&self, id: &str, user_id: &str, data: &serde_json::Value) -> Result<serde_json::Value, String> {
        let existing = self
            .db
            .get_property(id, user_id)?
            .ok_or("العرض غير موجود".to_string())?;

        validate_property_payload(data)?;

        let area = json_opt_number(data, "area", existing.area);
        let price = json_opt_number(data, "price", existing.price);
        let price_per_meter = json_opt_number(data, "price_per_meter", existing.price_per_meter);
        let previous_owner = existing.owner_client_id.clone();
        let owner_client_id = match data.get("owner_client_id") {
            Some(_) => self.resolve_owner_client(user_id, data)?,
            None => existing.owner_client_id,
        };
        let (owner_name, owner_phone) = self.owner_display_fields(
            user_id,
            &owner_client_id,
            json_opt_text(data, "owner_name", existing.owner_name.clone()),
            json_opt_text(data, "owner_phone", existing.owner_phone.clone()),
        )?;

        let name = match data.get("name").and_then(|v| v.as_str()) {
            Some(n) if n.trim().is_empty() => return Err("اسم العقار مطلوب".to_string()),
            Some(n) => n.trim().to_string(),
            None => existing.name,
        };

        let property = Property {
            context: crate::country::RecordContext::default(),
            id: id.to_string(),
            user_id: user_id.to_string(),
            name,
            city: json_opt_text(data, "city", existing.city),
            district: json_opt_text(data, "district", existing.district),
            location: json_opt_text(data, "location", existing.location),
            property_type: json_opt_text(data, "property_type", existing.property_type),
            area,
            price,
            price_per_meter,
            ad_number: json_opt_text(data, "ad_number", existing.ad_number),
            ad_link: json_opt_text(data, "ad_link", existing.ad_link),
            owner_name,
            owner_phone,
            owner_contact: json_opt_text(data, "owner_contact", existing.owner_contact),
            owner_notes: json_opt_text(data, "owner_notes", existing.owner_notes),
            marketing_platform: json_opt_text(data, "marketing_platform", existing.marketing_platform),
            marketing_link: json_opt_text(data, "marketing_link", existing.marketing_link),
            published_date: json_opt_text(data, "published_date", existing.published_date),
            main_image: existing.main_image,
            status: data.get("status").and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty()).map(|s| s.to_string()).unwrap_or(existing.status),
            created_at: existing.created_at,
            updated_at: chrono::Utc::now().to_rfc3339(),
            owner_client_id,
            source_type: json_opt_text(data, "source_type", existing.source_type),
            purpose: json_opt_text(data, "purpose", existing.purpose),
            usage_type: json_opt_text(data, "usage_type", existing.usage_type),
            frontage: json_opt_text(data, "frontage", existing.frontage),
            street_width: json_opt_number(data, "street_width", existing.street_width),
            streets_count: json_opt_int(data, "streets_count", existing.streets_count),
            latitude: json_opt_number(data, "latitude", existing.latitude),
            longitude: json_opt_number(data, "longitude", existing.longitude),
            description: json_opt_text(data, "description", existing.description),
            listing_expiry_date: json_opt_text(data, "listing_expiry_date", existing.listing_expiry_date),
            commission_agreement: json_opt_text(data, "commission_agreement", existing.commission_agreement),
            fal_ad_license: json_opt_text(data, "fal_ad_license", existing.fal_ad_license),
            internal_price: json_opt_number(data, "internal_price", existing.internal_price),
            internal_notes: json_opt_text(data, "internal_notes", existing.internal_notes),
            is_training: existing.is_training,
        };
        self.db.update_property_atomic(
            &property,
            previous_owner.as_deref(),
            user_id,
            &AuditLog {
                id: Uuid::new_v4().to_string(),
                user_id: Some(user_id.to_string()),
                action: "property_update".to_string(),
                entity_type: Some("property".to_string()),
                entity_id: Some(id.to_string()),
                details: Some(format!("تعديل عرض: {}", property.name)),
                created_at: chrono::Utc::now().to_rfc3339(),
            },
        )?;

        let updated = self.db.get_property(id, user_id)?.unwrap();
        Ok(property_to_json(&updated))
    }

    pub fn delete(&self, id: &str, user_id: &str) -> Result<serde_json::Value, String> {
        let existing = self
            .db
            .get_property(id, user_id)?
            .ok_or("العرض غير موجود".to_string())?;
        self.db.delete_property_atomic(
            id,
            &AuditLog {
                id: Uuid::new_v4().to_string(),
                user_id: Some(user_id.to_string()),
                action: "property_delete".to_string(),
                entity_type: Some("property".to_string()),
                entity_id: Some(id.to_string()),
                details: Some(format!("حذف عرض: {}", existing.name)),
                created_at: chrono::Utc::now().to_rfc3339(),
            },
        )?;
        Ok(serde_json::json!({ "message": "تم حذف العرض" }))
    }

    pub fn change_status(&self, id: &str, user_id: &str, status: &str) -> Result<serde_json::Value, String> {
        let existing = self
            .db
            .get_property(id, user_id)?
            .ok_or("العرض غير موجود".to_string())?;
        self.db.change_property_status_atomic(
            id,
            status,
            &PropertyUpdate {
                id: Uuid::new_v4().to_string(),
                property_id: id.to_string(),
                user_id: user_id.to_string(),
                title: Some("تغيير حالة العقار".to_string()),
                description: Some(format!("تم تغيير حالة العرض إلى: {}", status)),
                created_at: chrono::Utc::now().to_rfc3339(),
            },
        )?;
        Ok(serde_json::json!({ "message": "تم تحديث الحالة", "status": status }))
    }

    pub fn add_update(&self, id: &str, user_id: &str, title: &str, description: &str) -> Result<serde_json::Value, String> {
        let existing = self
            .db
            .get_property(id, user_id)?
            .ok_or("العرض غير موجود".to_string())?;
        let update = PropertyUpdate {
            id: Uuid::new_v4().to_string(),
            property_id: id.to_string(),
            user_id: user_id.to_string(),
            title: Some(if title.is_empty() { "تحديث يدوي".to_string() } else { title.to_string() }),
            description: Some(description.to_string()),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.db.create_property_update(&update)?;
        Ok(serde_json::json!({ "id": update.id, "title": update.title, "description": update.description, "created_at": update.created_at }))
    }

    pub fn add_image(&self, id: &str, user_id: &str, image_url: &str, is_main: bool) -> Result<serde_json::Value, String> {
        let image_url = image_url.trim();
        if image_url.is_empty() {
            return Err("ملف الصورة مطلوب".to_string());
        }
        let existing = self
            .db
            .get_property(id, user_id)?
            .ok_or("العرض غير موجود".to_string())?;
        let current = self.db.count_property_images(id)?;
        if current >= MAX_PROPERTY_IMAGES {
            return Err("الحد الأقصى 4 صور لكل عقار".to_string());
        }
        let image = PropertyImage {
            id: Uuid::new_v4().to_string(),
            property_id: id.to_string(),
            image_url: image_url.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        let no_main = existing
            .main_image
            .as_deref()
            .map(|s| s.trim().is_empty())
            .unwrap_or(true);
        let set_main = if is_main || no_main {
            Some(image_url)
        } else {
            None
        };
        self.db
            .add_property_image_atomic(&image, set_main)?;
        Ok(serde_json::json!({ "id": image.id, "image_url": image_url }))
    }

    pub fn delete_image(&self, image_id: &str, user_id: &str) -> Result<serde_json::Value, String> {
        let img = self
            .db
            .get_property_image(image_id)?
            .ok_or("الصورة غير موجودة".to_string())?;
        let property = self
            .db
            .get_property(&img.property_id, user_id)?
            .ok_or("العرض غير موجود".to_string())?;
        let remaining = self.db.list_property_images(&img.property_id)?;
        let was_main = property.main_image.as_deref() == Some(img.image_url.as_str());
        // list still includes the image we're deleting — exclude it for next main
        let next_main = if was_main {
            remaining
                .iter()
                .find(|r| r.id != image_id)
                .map(|r| r.image_url.as_str())
        } else {
            None
        };
        self.db.delete_property_image_atomic(
            image_id,
            &img.property_id,
            was_main,
            next_main,
        )?;
        Ok(serde_json::json!({ "message": "تم حذف الصورة" }))
    }

    pub fn duplicate(&self, id: &str, user_id: &str) -> Result<serde_json::Value, String> {
        let existing = self
            .db
            .get_property(id, user_id)?
            .ok_or("العرض غير موجود".to_string())?;

        let new_id = Uuid::new_v4().to_string();
        let main_image = existing.main_image.clone();
        let property = Property {
            context: crate::country::RecordContext::default(),
            city: existing.city,
            district: existing.district,
            location: existing.location,
            property_type: existing.property_type,
            area: existing.area,
            price: existing.price,
            price_per_meter: existing.price_per_meter,
            ad_number: existing.ad_number,
            ad_link: existing.ad_link,
            owner_name: existing.owner_name,
            owner_phone: existing.owner_phone,
            owner_contact: existing.owner_contact,
            owner_notes: existing.owner_notes,
            marketing_platform: existing.marketing_platform,
            marketing_link: existing.marketing_link,
            published_date: existing.published_date,
            owner_client_id: existing.owner_client_id,
            source_type: existing.source_type,
            purpose: existing.purpose,
            usage_type: existing.usage_type,
            frontage: existing.frontage,
            street_width: existing.street_width,
            streets_count: existing.streets_count,
            latitude: existing.latitude,
            longitude: existing.longitude,
            description: existing.description,
            listing_expiry_date: existing.listing_expiry_date,
            commission_agreement: existing.commission_agreement,
            fal_ad_license: existing.fal_ad_license,
            internal_price: existing.internal_price,
            internal_notes: existing.internal_notes,
            ..Property::blank(new_id.clone(), user_id.to_string(), format!("{} (نسخة)", existing.name))
        };
        // نسخ الصور (بحد أقصى 4)
        let images = self.db.list_property_images(id)?;
        let copied: Vec<PropertyImage> = images
            .into_iter()
            .take(MAX_PROPERTY_IMAGES as usize)
            .map(|img| PropertyImage {
                id: Uuid::new_v4().to_string(),
                property_id: new_id.clone(),
                image_url: img.image_url,
                created_at: chrono::Utc::now().to_rfc3339(),
            })
            .collect();
        self.db.duplicate_property_atomic(
            &property,
            &copied,
            main_image.as_deref(),
        )?;

        let created = self.db.get_property(&new_id, user_id)?.unwrap();
        Ok(property_to_json(&created))
    }
}

fn property_to_json(p: &Property) -> serde_json::Value {
    let mut result = serde_json::json!({
        "id": p.id,
        "user_id": p.user_id,
        "name": p.name,
        "city": p.city,
        "district": p.district,
        "location": p.location,
        "property_type": p.property_type,
        "area": p.area,
        "price": p.price,
        "price_per_meter": p.price_per_meter,
        "ad_number": p.ad_number,
        "ad_link": p.ad_link,
        "owner_name": p.owner_name,
        "owner_phone": p.owner_phone,
        "owner_contact": p.owner_contact,
        "owner_notes": p.owner_notes,
        "marketing_platform": p.marketing_platform,
        "marketing_link": p.marketing_link,
        "published_date": p.published_date,
        "main_image": p.main_image,
        "status": p.status,
        "created_at": p.created_at,
        "updated_at": p.updated_at,
        "owner_client_id": p.owner_client_id,
        "source_type": p.source_type,
        "purpose": p.purpose,
        "usage_type": p.usage_type,
        "frontage": p.frontage,
        "street_width": p.street_width,
        "streets_count": p.streets_count,
        "latitude": p.latitude,
        "longitude": p.longitude,
        "description": p.description,
        "listing_expiry_date": p.listing_expiry_date,
        "commission_agreement": p.commission_agreement,
        "fal_ad_license": p.fal_ad_license,
        "internal_price": p.internal_price,
        "internal_notes": p.internal_notes,
        "is_training": p.is_training,
    });
    result.as_object_mut().unwrap().extend(serde_json::to_value(&p.context).unwrap().as_object().unwrap().clone());
    result
}

/// أنواع مصدر العقار المعتمدة.
pub const PROPERTY_SOURCE_TYPES: &[&str] = &[
    "direct_owner", "authorized", "broker", "marketer", "developer", "other",
];

/// أغراض العقار المعتمدة.
pub const PROPERTY_PURPOSES: &[&str] = &["sale", "rent", "investment"];

/// الحقول الداخلية الحساسة التي يجب ألا تظهر في أي مخرج مشاركة.
pub const PROPERTY_INTERNAL_FIELDS: &[&str] = &[
    "owner_name", "owner_phone", "owner_contact", "owner_notes",
    "owner_client_id", "internal_price", "internal_notes", "commission_agreement",
];

/// نسخة قابلة للمشاركة من العقار: تُسقط الحقول الداخلية الحساسة.
pub fn property_to_shareable_json(p: &Property) -> serde_json::Value {
    let mut obj = property_to_json(p);
    if let Some(map) = obj.as_object_mut() {
        for field in PROPERTY_INTERNAL_FIELDS {
            map.remove(*field);
        }
    }
    obj
}

fn validate_property_payload(data: &serde_json::Value) -> Result<(), String> {
    if let Some(v) = data.get("source_type").and_then(|v| v.as_str()) {
        if !v.trim().is_empty() && !PROPERTY_SOURCE_TYPES.contains(&v.trim()) {
            return Err(format!("نوع مصدر غير معروف: {v}"));
        }
    }
    if let Some(v) = data.get("purpose").and_then(|v| v.as_str()) {
        if !v.trim().is_empty() && !PROPERTY_PURPOSES.contains(&v.trim()) {
            return Err(format!("غرض غير معروف: {v}"));
        }
    }
    for field in ["area", "price", "price_per_meter", "street_width", "internal_price"] {
        if let Some(n) = json_number(data, field) {
            if n < 0.0 {
                return Err("القيم الرقمية لا يمكن أن تكون سالبة".to_string());
            }
        }
    }
    if let (Some(lat), Some(lon)) = (json_number(data, "latitude"), json_number(data, "longitude")) {
        if !(-90.0..=90.0).contains(&lat) {
            return Err("خط العرض يجب أن يكون بين -90 و 90".to_string());
        }
        if !(-180.0..=180.0).contains(&lon) {
            return Err("خط الطول يجب أن يكون بين -180 و 180".to_string());
        }
        if lat.abs() < 0.05 && lon.abs() < 0.05 {
            return Err("الموقع غير صالح. انقر داخل المدينة على الخريطة وليس في المحيط.".to_string());
        }
    } else {
        if let Some(lat) = json_number(data, "latitude") {
            if !(-90.0..=90.0).contains(&lat) {
                return Err("خط العرض يجب أن يكون بين -90 و 90".to_string());
            }
        }
        if let Some(lon) = json_number(data, "longitude") {
            if !(-180.0..=180.0).contains(&lon) {
                return Err("خط الطول يجب أن يكون بين -180 و 180".to_string());
            }
        }
    }
    Ok(())
}

/// يقرأ رقماً قد يصل كنص من نموذج HTML. النص الفارغ = غير مُرسَل.
fn json_number(data: &serde_json::Value, key: &str) -> Option<f64> {
    match data.get(key) {
        Some(serde_json::Value::Number(n)) => n.as_f64(),
        Some(serde_json::Value::String(s)) => {
            let s = s.trim();
            if s.is_empty() {
                None
            } else {
                s.parse::<f64>().ok()
            }
        }
        _ => None,
    }
}

fn json_int(data: &serde_json::Value, key: &str) -> Option<i64> {
    json_number(data, key).map(|n| n.round() as i64)
}

fn json_text(data: &serde_json::Value, key: &str) -> Option<String> {
    data.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn json_opt_text(data: &serde_json::Value, key: &str, existing: Option<String>) -> Option<String> {
    match data.get(key) {
        None => existing,
        Some(serde_json::Value::Null) => None,
        Some(_) => json_text(data, key),
    }
}

fn json_opt_number(data: &serde_json::Value, key: &str, existing: Option<f64>) -> Option<f64> {
    match data.get(key) {
        None => existing,
        Some(serde_json::Value::Null) => None,
        Some(_) => json_number(data, key),
    }
}

fn json_opt_int(data: &serde_json::Value, key: &str, existing: Option<i64>) -> Option<i64> {
    json_opt_number(data, key, existing.map(|n| n as f64)).map(|n| n.round() as i64)
}

// ===== خدمات العملاء =====

pub struct ClientService<'a> {
    pub db: &'a dyn Database,
}

impl<'a> ClientService<'a> {
    pub fn new(db: &'a dyn Database) -> Self {
        Self { db }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn list(
        &self,
        user_id: &str,
        page: i64,
        limit: i64,
        search: &str,
        role: &str,
        city: &str,
        status: &str,
        needs_followup: bool,
    ) -> Result<serde_json::Value, String> {
        if !role.is_empty() && !CLIENT_ROLES.contains(&role) {
            return Err(format!("دور غير معروف: {role}"));
        }
        let offset = (page - 1) * limit;
        let (items, total) = self
            .db
            .list_clients(user_id, search, role, city, status, needs_followup, limit, offset)?;

        // الأدوار تُقرأ من client_roles في استعلام واحد لكل الصفحة.
        let ids: Vec<String> = items.iter().map(|c| c.id.clone()).collect();
        let pairs = self.db.list_roles_for_clients(&ids)?;
        let mut roles_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        for (client_id, role) in pairs {
            roles_map.entry(client_id).or_default().push(role);
        }

        let enriched: Vec<serde_json::Value> = items
            .iter()
            .map(|c| {
                let mut obj = client_to_json(c);
                if let Some(map) = obj.as_object_mut() {
                    map.insert(
                        "roles".to_string(),
                        serde_json::json!(roles_map.get(&c.id).cloned().unwrap_or_default()),
                    );
                }
                obj
            })
            .collect();

        Ok(serde_json::json!({ "items": enriched, "total": total, "page": page, "limit": limit }))
    }

    pub fn get(&self, id: &str, user_id: &str) -> Result<serde_json::Value, String> {
        let client = self
            .db
            .get_client(id, user_id)?
            .ok_or("العميل غير موجود".to_string())?;
        let communications = self.db.list_communications(id)?;
        let linked_properties = self.db.list_linked_properties_for_client(id)?;
        let roles = self.db.list_client_roles(id)?;

        let mut obj = client_to_json(&client);
        if let Some(map) = obj.as_object_mut() {
            map.insert("roles".to_string(), serde_json::json!(roles));
            map.insert("communications".to_string(), serde_json::to_value(communications).unwrap_or_default());
            map.insert("linked_properties".to_string(), serde_json::to_value(linked_properties).unwrap_or_default());
        }
        Ok(obj)
    }

    pub fn create(&self, user_id: &str, data: &serde_json::Value) -> Result<serde_json::Value, String> {
        let name = data.get("name").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        if name.is_empty() {
            return Err("اسم العميل مطلوب".to_string());
        }
        validate_client_payload(data)?;
        let roles = parse_roles(data)?.unwrap_or_default();

        let id = Uuid::new_v4().to_string();
        let client = Client {
            context: crate::country::RecordContext::default(),
            id: id.clone(),
            user_id: user_id.to_string(),
            name,
            phone: json_text(data, "phone"),
            whatsapp: json_text(data, "whatsapp"),
            relationship_type: roles.first().cloned(),
            city: json_text(data, "city"),
            interests: json_text(data, "interests"),
            required_property_type: json_text(data, "required_property_type"),
            budget: json_text(data, "budget"),
            notes: json_text(data, "notes"),
            last_contact: None,
            follow_up_date: json_text(data, "follow_up_date"),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            email: json_text(data, "email"),
            phone_secondary: json_text(data, "phone_secondary"),
            company: json_text(data, "company"),
            source: json_text(data, "source"),
            status: json_text(data, "status"),
            category: json_text(data, "category"),
            is_training: false,
        };
        self.db.create_client_atomic(
            &client,
            &roles,
            &AuditLog {
                id: Uuid::new_v4().to_string(),
                user_id: Some(user_id.to_string()),
                action: "client_create".to_string(),
                entity_type: Some("client".to_string()),
                entity_id: Some(id.clone()),
                details: Some(format!("إضافة عميل: {}", client.name)),
                created_at: chrono::Utc::now().to_rfc3339(),
            },
        )?;

        let created = self.db.get_client(&id, user_id)?.ok_or("تعذر قراءة العميل بعد الإنشاء")?;
        let mut obj = client_to_json(&created);
        if let Some(map) = obj.as_object_mut() {
            map.insert("roles".to_string(), serde_json::json!(self.db.list_client_roles(&id)?));
        }
        Ok(obj)
    }

    pub fn update(&self, id: &str, user_id: &str, data: &serde_json::Value) -> Result<serde_json::Value, String> {
        let existing = self
            .db
            .get_client(id, user_id)?
            .ok_or("العميل غير موجود".to_string())?;
        validate_client_payload(data)?;
        let roles = parse_roles(data)?;

        let name = match data.get("name").and_then(|v| v.as_str()) {
            Some(n) if n.trim().is_empty() => return Err("اسم العميل مطلوب".to_string()),
            Some(n) => n.trim().to_string(),
            None => existing.name,
        };

        let client = Client {
            context: crate::country::RecordContext::default(),
            id: id.to_string(),
            user_id: user_id.to_string(),
            name,
            phone: json_opt_text(data, "phone", existing.phone),
            whatsapp: json_opt_text(data, "whatsapp", existing.whatsapp),
            // relationship_type يُشتق من الأدوار؛ set_client_roles يحدّثه بعد الحفظ.
            relationship_type: roles
                .as_ref()
                .and_then(|r| r.first().cloned())
                .or(existing.relationship_type),
            city: json_opt_text(data, "city", existing.city),
            interests: json_opt_text(data, "interests", existing.interests),
            required_property_type: json_opt_text(data, "required_property_type", existing.required_property_type),
            budget: json_opt_text(data, "budget", existing.budget),
            notes: json_opt_text(data, "notes", existing.notes),
            last_contact: json_opt_text(data, "last_contact", existing.last_contact),
            follow_up_date: json_opt_text(data, "follow_up_date", existing.follow_up_date),
            created_at: existing.created_at,
            updated_at: chrono::Utc::now().to_rfc3339(),
            email: json_opt_text(data, "email", existing.email),
            phone_secondary: json_opt_text(data, "phone_secondary", existing.phone_secondary),
            company: json_opt_text(data, "company", existing.company),
            source: json_opt_text(data, "source", existing.source),
            status: json_opt_text(data, "status", existing.status),
            category: json_opt_text(data, "category", existing.category),
            is_training: existing.is_training,
        };
        self.db.update_client_atomic(
            &client,
            roles.as_deref(),
            &AuditLog {
                id: Uuid::new_v4().to_string(),
                user_id: Some(user_id.to_string()),
                action: "client_update".to_string(),
                entity_type: Some("client".to_string()),
                entity_id: Some(id.to_string()),
                details: Some(format!("تعديل عميل: {}", client.name)),
                created_at: chrono::Utc::now().to_rfc3339(),
            },
        )?;

        let updated = self.db.get_client(id, user_id)?.ok_or("تعذر قراءة العميل بعد التعديل")?;
        let mut obj = client_to_json(&updated);
        if let Some(map) = obj.as_object_mut() {
            map.insert("roles".to_string(), serde_json::json!(self.db.list_client_roles(id)?));
        }
        Ok(obj)
    }

    /// قائمة جهات الاتصال لاختيار مالك/مصدر عقار.
    pub fn options(&self, user_id: &str, search: &str) -> Result<Vec<serde_json::Value>, String> {
        self.db.list_client_options(user_id, search.trim(), 50)
    }

    pub fn delete(&self, id: &str, user_id: &str) -> Result<serde_json::Value, String> {
        let existing = self
            .db
            .get_client(id, user_id)?
            .ok_or("العميل غير موجود".to_string())?;
        self.db.delete_client_atomic(
            id,
            &AuditLog {
                id: Uuid::new_v4().to_string(),
                user_id: Some(user_id.to_string()),
                action: "client_delete".to_string(),
                entity_type: Some("client".to_string()),
                entity_id: Some(id.to_string()),
                details: Some(format!("حذف عميل: {}", existing.name)),
                created_at: chrono::Utc::now().to_rfc3339(),
            },
        )?;
        Ok(serde_json::json!({ "message": "تم حذف العميل" }))
    }

    pub fn add_communication(&self, id: &str, user_id: &str, comm_type: &str, subject: &str, notes: &str) -> Result<serde_json::Value, String> {
        let existing = self
            .db
            .get_client(id, user_id)?
            .ok_or("العميل غير موجود".to_string())?;
        let comm = Communication {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            client_id: id.to_string(),
            r#type: Some(if comm_type.is_empty() { "note".to_string() } else { comm_type.to_string() }),
            subject: Some(subject.to_string()),
            notes: Some(notes.to_string()),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.db.add_communication_atomic(&comm)?;
        Ok(serde_json::json!({ "id": comm.id, "type": comm.r#type, "subject": comm.subject, "notes": comm.notes, "created_at": comm.created_at }))
    }

    pub fn link_property(&self, id: &str, user_id: &str, property_id: &str, link_type: &str) -> Result<serde_json::Value, String> {
        let client = self
            .db
            .get_client(id, user_id)?
            .ok_or("العميل غير موجود".to_string())?;
        let property = self
            .db
            .get_property(property_id, user_id)?
            .ok_or("العرض غير موجود".to_string())?;

        let existing = self.db.get_client_property_link(id, property_id)?;
        if let Some(link) = existing {
            self.db.update_client_property_link_type(&link.id, if link_type.is_empty() { "sent" } else { link_type })?;
        } else {
            self.db.create_client_property_link(&ClientPropertyLink {
                id: Uuid::new_v4().to_string(),
                client_id: id.to_string(),
                property_id: property_id.to_string(),
                link_type: Some(if link_type.is_empty() { "sent".to_string() } else { link_type.to_string() }),
                created_at: chrono::Utc::now().to_rfc3339(),
            })?;
        }
        Ok(serde_json::json!({ "message": "تم ربط العرض بالعميل" }))
    }

    pub fn unlink_property(&self, id: &str, user_id: &str, property_id: &str) -> Result<serde_json::Value, String> {
        // تحقق ملكية صريح قبل الحذف — لا يكفي التحقق من الجلسة فقط
        let _client = self
            .db
            .get_client(id, user_id)?
            .ok_or_else(|| "العميل غير موجود".to_string())?;
        let _property = self
            .db
            .get_property(property_id, user_id)?
            .ok_or_else(|| "العرض غير موجود".to_string())?;

        let deleted = self.db.delete_client_property_link(id, property_id, user_id)?;
        if deleted == 0 {
            return Err("الربط غير موجود أو غير مصرح".into());
        }
        Ok(serde_json::json!({ "message": "تم إلغاء ربط العرض" }))
    }
}

/// الأدوار المعتمدة لجهة الاتصال. أي دور خارجها يُرفض بدل تخزينه بصمت.
pub const CLIENT_ROLES: &[&str] = &[
    "buyer", "tenant", "owner", "investor", "developer", "broker", "marketer", "company",
];

/// حالات العميل المعتمدة.
pub const CLIENT_STATUSES: &[&str] = &["new", "active", "negotiating", "closed", "inactive"];

/// يقرأ الأدوار من الحِمل الوارد ويتحقق منها. `None` = الحقل غير مُرسَل إطلاقاً.
fn parse_roles(data: &serde_json::Value) -> Result<Option<Vec<String>>, String> {
    let raw = match data.get("roles") {
        None | Some(serde_json::Value::Null) => return Ok(None),
        Some(v) => v,
    };
    let list: Vec<String> = match raw {
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        serde_json::Value::String(s) => s
            .split(',')
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect(),
        _ => return Err("صيغة الأدوار غير صحيحة".to_string()),
    };
    let mut unique: Vec<String> = Vec::new();
    for role in list {
        if !CLIENT_ROLES.contains(&role.as_str()) {
            return Err(format!("دور غير معروف: {role}"));
        }
        if !unique.contains(&role) {
            unique.push(role);
        }
    }
    Ok(Some(unique))
}

fn validate_client_payload(data: &serde_json::Value) -> Result<(), String> {
    if let Some(email) = data.get("email").and_then(|v| v.as_str()) {
        let email = email.trim();
        if !email.is_empty() && (!email.contains('@') || email.starts_with('@') || email.ends_with('@') || !email.contains('.')) {
            return Err("صيغة البريد الإلكتروني غير صحيحة".to_string());
        }
    }
    for field in ["phone", "phone_secondary", "whatsapp"] {
        if let Some(v) = data.get(field).and_then(|v| v.as_str()) {
            let digits = v.chars().filter(|c| c.is_ascii_digit()).count();
            if !v.trim().is_empty() && digits < 7 {
                return Err("رقم الجوال يجب أن يحتوي 7 أرقام على الأقل".to_string());
            }
        }
    }
    if let Some(status) = data.get("status").and_then(|v| v.as_str()) {
        if !status.trim().is_empty() && !CLIENT_STATUSES.contains(&status.trim()) {
            return Err(format!("حالة عميل غير معروفة: {status}"));
        }
    }
    Ok(())
}

fn client_to_json(c: &Client) -> serde_json::Value {
    serde_json::json!({
        "id": c.id,
        "user_id": c.user_id,
        "name": c.name,
        "phone": c.phone,
        "phone_secondary": c.phone_secondary,
        "whatsapp": c.whatsapp,
        "email": c.email,
        "company": c.company,
        "source": c.source,
        "status": c.status,
        "category": c.category,
        "relationship_type": c.relationship_type,
        "city": c.city,
        "interests": c.interests,
        "required_property_type": c.required_property_type,
        "budget": c.budget,
        "notes": c.notes,
        "last_contact": c.last_contact,
        "follow_up_date": c.follow_up_date,
        "created_at": c.created_at,
        "updated_at": c.updated_at,
        "is_training": c.is_training,
    })
}

// ===== خدمات المؤشر =====

pub struct MarketService<'a> {
    pub db: &'a dyn Database,
}

impl<'a> MarketService<'a> {
    pub fn new(db: &'a dyn Database) -> Self {
        Self { db }
    }

    pub fn list(&self, user_id: &str, page: i64, limit: i64, city: &str, district: &str, property_type: &str) -> Result<serde_json::Value, String> {
        let offset = (page - 1) * limit;
        let (items, total) = self.db.list_market_data(user_id, city, district, property_type, limit, offset)?;
        Ok(serde_json::json!({ "items": items, "total": total, "page": page, "limit": limit }))
    }

    pub fn stats(&self, user_id: &str, city: &str, district: &str, property_type: &str) -> Result<serde_json::Value, String> {
        self.db.get_market_stats(user_id, city, district, property_type)
    }

    pub fn trends(&self, user_id: &str, city: &str, district: &str, property_type: &str) -> Result<Vec<serde_json::Value>, String> {
        self.db.get_market_trends(user_id, city, district, property_type)
    }

    /// مدن المؤشر من SQLite الفعلية — مصدر ديناميكي للفلتر (لا قوائم ثابتة).
    pub fn filter_cities(&self, user_id: &str) -> Result<Vec<String>, String> {
        self.db.market_distinct_cities(user_id)
    }

    /// أحياء مدينة محددة — فلتر تابع للمدينة من SQLite.
    pub fn filter_districts(&self, user_id: &str, city: &str) -> Result<Vec<String>, String> {
        if city.trim().is_empty() {
            return Ok(Vec::new());
        }
        self.db.market_districts_of_city(user_id, city)
    }

    /// بطاقات (مدينة/حي) المجمعة لرأس المؤشر — أحدث المجموعات أولاً من SQLite.
    /// mode 'city' يجمع بالمدينة، وmode 'district' ينزل إلى أحياء المدينة المختارة.
    pub fn geo_summary(&self, user_id: &str, city: &str, district: &str, mode: &str, limit: i64) -> Result<Vec<serde_json::Value>, String> {
        self.db.market_geo_summary(user_id, city, district, mode, limit)
    }

    /// حذف جماعي لكل السجلات المطابقة للفلاتر الحالية — داخل SQLite بدون تحميل IDs.
    pub fn delete_filtered(&self, user_id: &str, city: &str, district: &str, property_type: &str) -> Result<i64, String> {
        self.db.delete_market_data_filtered(user_id, city, district, property_type)
    }

    pub fn create(&self, user_id: &str, data: &serde_json::Value) -> Result<serde_json::Value, String> {
        let current = crate::country::RecordContext::from_settings(&self.db.get_settings(user_id)?)?;
        if data.get("country").and_then(|v|v.as_str()).is_some_and(|v| !v.is_empty() && v != current.country)
            || data.get("currency").and_then(|v|v.as_str()).is_some_and(|v| !v.is_empty() && v != current.currency) {
            return Err("دولة أو عملة البيانات المستوردة تختلف عن الإعدادات؛ اختر دولة البيانات قبل الاستيراد، دون تحويل المبالغ".into());
        }

        let area = data.get("area").and_then(|v| v.as_f64());
        let sale_price = data.get("sale_price").and_then(|v| v.as_f64());
        let rent_value = data.get("rent_value").and_then(|v| v.as_f64());
        let mut price_per_meter_sale = match (area, sale_price) {
            (Some(a), Some(s)) if a > 0.0 => Some(s / a),
            _ => None,
        };
        // لا نقبل تخزين صفر مضلل كمتر — فقط قيمة موجبة صريحة إن وُجدت بدون مساحة صالحة
        if price_per_meter_sale.is_none() {
            if let Some(v) = data.get("price_per_meter_sale").and_then(|v| v.as_f64()) {
                if v > 0.0 {
                    price_per_meter_sale = Some(v);
                }
            }
        }
        let mut rent_per_meter = match (area, rent_value) {
            (Some(a), Some(r)) if a > 0.0 => Some(r / a),
            _ => None,
        };
        if rent_per_meter.is_none() {
            if let Some(v) = data.get("rent_per_meter").and_then(|v| v.as_f64()) {
                if v > 0.0 {
                    rent_per_meter = Some(v);
                }
            }
        }

        let id = Uuid::new_v4().to_string();
        let item = MarketReferenceData {
            context: crate::country::RecordContext::default(),
            id: id.clone(),
            user_id: user_id.to_string(),
            city: data.get("city").and_then(|v| v.as_str()).map(|s| s.to_string()),
            district: data.get("district").and_then(|v| v.as_str()).map(|s| s.to_string()),
            property_type: data.get("property_type").and_then(|v| v.as_str()).map(|s| s.to_string()),
            usage_type: data.get("usage_type").and_then(|v| v.as_str()).map(|s| s.to_string()),
            area,
            sale_price,
            price_per_meter_sale,
            rent_value,
            rent_per_meter,
            requests_count: data.get("requests_count").and_then(|v| v.as_i64()),
            offers_count: data.get("offers_count").and_then(|v| v.as_i64()),
            entry_date: data.get("entry_date").and_then(|v| v.as_str()).map(|s| s.to_string()),
            data_source: data.get("data_source").and_then(|v| v.as_str()).map(|s| s.to_string()),
            notes: data.get("notes").and_then(|v| v.as_str()).map(|s| s.to_string()),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.db.create_market_data(&item)?;
        Ok(serde_json::json!({ "id": id }))
    }

    pub fn update(&self, id: &str, user_id: &str, data: &serde_json::Value) -> Result<serde_json::Value, String> {
        let existing = self
            .db
            .get_market_data(id, user_id)?
            .ok_or_else(|| "السجل غير موجود".to_string())?;

        let area = data
            .get("area")
            .and_then(|v| v.as_f64())
            .or(existing.area);
        let sale_price = if data.get("sale_price").is_some() {
            data.get("sale_price").and_then(|v| v.as_f64())
        } else {
            existing.sale_price
        };
        let rent_value = if data.get("rent_value").is_some() {
            data.get("rent_value").and_then(|v| v.as_f64())
        } else {
            existing.rent_value
        };
        let price_per_meter_sale = match (area, sale_price) {
            (Some(a), Some(s)) if a > 0.0 => Some(s / a),
            _ => None,
        };
        let rent_per_meter = match (area, rent_value) {
            (Some(a), Some(r)) if a > 0.0 => Some(r / a),
            _ => None,
        };

        let item = MarketReferenceData {
            context: crate::country::RecordContext::default(),
            id: existing.id,
            user_id: existing.user_id,
            city: data
                .get("city")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or(existing.city),
            district: data
                .get("district")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or(existing.district),
            property_type: data
                .get("property_type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or(existing.property_type),
            usage_type: data
                .get("usage_type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or(existing.usage_type),
            area,
            sale_price,
            price_per_meter_sale,
            rent_value,
            rent_per_meter,
            requests_count: data
                .get("requests_count")
                .and_then(|v| v.as_i64())
                .or(existing.requests_count),
            offers_count: data
                .get("offers_count")
                .and_then(|v| v.as_i64())
                .or(existing.offers_count),
            entry_date: data
                .get("entry_date")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or(existing.entry_date),
            data_source: data
                .get("data_source")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or(existing.data_source),
            notes: data
                .get("notes")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or(existing.notes),
            created_at: existing.created_at,
        };
        self.db.update_market_data(&item)?;
        Ok(serde_json::json!({ "id": item.id, "message": "تم تحديث السجل" }))
    }

    pub fn bulk_create(&self, user_id: &str, items: &[serde_json::Value]) -> Result<serde_json::Value, String> {
        let (inserted, errors) = self.db.create_market_data_bulk(user_id, items)?;
        Ok(serde_json::json!({
            "message": format!("تم استيراد {} سجل", inserted),
            "inserted_count": inserted,
            "errors": errors
        }))
    }

    pub fn delete(&self, id: &str, user_id: &str) -> Result<serde_json::Value, String> {
        self.db.delete_market_data(id, user_id)?;
        Ok(serde_json::json!({ "message": "تم حذف السجل" }))
    }

    pub fn delete_many(&self, ids: &[String], user_id: &str) -> Result<serde_json::Value, String> {
        if ids.is_empty() {
            return Ok(serde_json::json!({ "deleted": 0, "message": "لم يتم تحديد سجلات" }));
        }
        let deleted = self.db.delete_many_market_data(ids, user_id)?;
        Ok(serde_json::json!({
            "deleted": deleted,
            "message": format!("تم حذف {} سجل", deleted)
        }))
    }

    pub fn purge_audit_leftovers(&self, user_id: &str) -> Result<serde_json::Value, String> {
        let deleted = self.db.purge_audit_market_data(user_id)?;
        Ok(serde_json::json!({
            "deleted": deleted,
            "message": if deleted > 0 {
                format!("تم حذف {} سجل تدقيق متبقٍ", deleted)
            } else {
                "لا توجد سجلات تدقيق للحذف".to_string()
            }
        }))
    }
}

// ===== خدمات الإعدادات =====

pub struct SettingsService<'a> {
    pub db: &'a dyn Database,
}

impl<'a> SettingsService<'a> {
    pub fn new(db: &'a dyn Database) -> Self {
        Self { db }
    }

    pub fn get(&self, user_id: &str) -> Result<serde_json::Value, String> {
        let settings = self.db.get_settings(user_id)?;
        let s = if let Some(s) = settings {
            s
        } else {
            self.db.create_settings(user_id)?;
            self.db.get_settings(user_id)?.unwrap()
        };
        let mut value = serde_json::json!(s);
        // عنوان التتبع داخلي فقط — لا يُعرَض للواجهة
        if let Some(obj) = value.as_object_mut() {
            obj.remove("tracking_endpoint_url");
        }
        Ok(value)
    }

    pub fn update(&self, user_id: &str, data: &serde_json::Value) -> Result<serde_json::Value, String> {
        let existing = self.db.get_settings(user_id)?;
        if existing.is_none() {
            self.db.create_settings(user_id)?;
        }
        let existing = self.db.get_settings(user_id)?.unwrap();

        let country = data.get("country").map(|v| v.as_str().ok_or("الدولة غير صالحة")).transpose()?.unwrap_or(&existing.country).to_string();
        let currency = crate::country::currency(&country)?;
        let changed_country = country != existing.country;
        if let Some(v) = data.get("display_currency") {
            if v.as_str() != Some(currency.as_str()) { return Err("تُحدد العملة من الدولة فقط".into()); }
        }
        if let Some(kind) = data.get("fal_license_type").and_then(|v| v.as_str()).filter(|v| !v.is_empty() && *v != "manual") {
            if !crate::country::profile(&country)?["licenses"].as_array().unwrap().iter().any(|l| l["id"] == kind) {
                return Err("نوع الترخيص غير معتمد للدولة المحددة؛ استخدم الإدخال اليدوي".into());
            }
        }
        let settings = Settings {
            country,
            broker_license_name: data.get("broker_license_name").and_then(|v|v.as_str()).map(str::to_owned)
                .or_else(|| if changed_country { None } else { existing.broker_license_name.clone() }),
            user_id: user_id.to_string(),
            office_name: data.get("office_name").and_then(|v| v.as_str()).map(|s| s.to_string()).or(existing.office_name),
            office_logo: data.get("office_logo").and_then(|v| v.as_str()).map(|s| s.to_string()).or(existing.office_logo),
            office_phone: data.get("office_phone").and_then(|v| v.as_str()).map(|s| s.to_string()).or(existing.office_phone),
            office_whatsapp: data.get("office_whatsapp").and_then(|v| v.as_str()).map(|s| s.to_string()).or(existing.office_whatsapp),
            office_email: data.get("office_email").and_then(|v| v.as_str()).map(|s| s.to_string()).or(existing.office_email),
            office_address: data.get("office_address").and_then(|v| v.as_str()).map(|s| s.to_string()).or(existing.office_address),
            broker_name: data.get("broker_name").and_then(|v| v.as_str()).map(|s| s.to_string()).or(existing.broker_name),
            broker_phone: data.get("broker_phone").and_then(|v| v.as_str()).map(|s| s.to_string()).or(existing.broker_phone),
            broker_whatsapp: data.get("broker_whatsapp").and_then(|v| v.as_str()).map(|s| s.to_string()).or(existing.broker_whatsapp),
            default_brochure_template: data.get("default_brochure_template").and_then(|v| v.as_str()).map(|s| s.to_string()).or(existing.default_brochure_template),
            fal_license_number: match data.get("fal_license_number") {
                Some(v) => v.as_str().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
                None => if changed_country { None } else { existing.fal_license_number },
            },
            fal_license_type: match data.get("fal_license_type") {
                Some(v) => v.as_str().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
                None => if changed_country { None } else { existing.fal_license_type },
            },
            ui_language: match data.get("ui_language") {
                Some(v) => v.as_str().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
                None => existing.ui_language,
            },
            display_currency: Some(currency),
            // لا يُقبل تعديل عنوان التتبع من الواجهة
            tracking_endpoint_url: existing.tracking_endpoint_url,
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        self.db.update_settings(&settings)?;
        self.get(user_id)
    }
}

// ===== خدمات لوحة المعلومات =====

pub struct DashboardService<'a> {
    pub db: &'a dyn Database,
}

impl<'a> DashboardService<'a> {
    pub fn new(db: &'a dyn Database) -> Self {
        Self { db }
    }

    pub fn stats(&self, user_id: &str) -> Result<serde_json::Value, String> {
        self.db.get_dashboard_stats(user_id)
    }

    pub fn activity(&self, user_id: &str) -> Result<Vec<serde_json::Value>, String> {
        let logs = self.db.list_audit_logs(user_id, 20)?;
        let action_labels: std::collections::HashMap<&str, &str> = [
            ("register", "تسجيل حساب"),
            ("login", "تسجيل دخول"),
            ("logout", "تسجيل خروج"),
            ("activate_license", "تفعيل ترخيص"),
            ("property_create", "إضافة عرض"),
            ("property_update", "تعديل عرض"),
            ("property_delete", "حذف عرض"),
            ("client_create", "إضافة عميل"),
            ("client_update", "تحديث عميل"),
            ("client_delete", "حذف عميل"),
        ]
        .iter()
        .cloned()
        .collect();

        let mut activities: Vec<serde_json::Value> = logs
            .iter()
            .map(|log| {
                serde_json::json!({
                    "id": log.id,
                    "action": log.action,
                    "label": action_labels.get(log.action.as_str()).unwrap_or(&log.action.as_str()),
                    "details": log.details,
                    "created_at": log.created_at,
                })
            })
            .collect();

        // إضافة أحدث التحديثات على العروض
        let recent_updates = self.db.get_recent_property_updates(user_id, 5)?;
        for u in recent_updates {
            activities.push(serde_json::json!({
                "id": u["id"],
                "action": "property_update",
                "label": "تحديث عرض",
                "details": format!("{}: {}", u["title"].as_str().unwrap_or(""), u["property_name"].as_str().unwrap_or("")),
                "created_at": u["created_at"],
            }));
        }

        // ترتيب تنازلي
        activities.sort_by(|a, b| {
            let a_time = a["created_at"].as_str().unwrap_or("");
            let b_time = b["created_at"].as_str().unwrap_or("");
            b_time.cmp(a_time)
        });
        activities.truncate(20);
        Ok(activities)
    }
}

// ===== خدمات البروشرات =====

pub struct BrochureService<'a> {
    pub db: &'a dyn Database,
}

impl<'a> BrochureService<'a> {
    pub fn new(db: &'a dyn Database) -> Self {
        Self { db }
    }

    pub fn templates() -> Vec<serde_json::Value> {
        vec![
            serde_json::json!({ "id": "royal-sheet", "name": "العرض التنفيذي", "description": "A4 طولي — رأس مكتبي وصورة عريضة وبيانات السعر" }),
            serde_json::json!({ "id": "investment-brief", "name": "الملف الاستثماري", "description": "A4 طولي — مؤشرات السعر والمساحة وسعر المتر" }),
            serde_json::json!({ "id": "gold-luxury", "name": "بروفايل السكن", "description": "A4 طولي — إطار فاخر وصورة كبيرة للسكن" }),
            serde_json::json!({ "id": "opportunity-brief", "name": "ملخص الفرصة", "description": "A5 — موجز سريع للتسليم في المعاينة" }),
            serde_json::json!({ "id": "modern-split", "name": "العرض الأفقي", "description": "A4 عرضي — صورة واسعة ولوحة بيانات" }),
            serde_json::json!({ "id": "unit-catalog", "name": "كتالوج الوحدات", "description": "A4 طولي — مواصفات مرتبة على عمودين" }),
            serde_json::json!({ "id": "viewing-dossier", "name": "ملف المعاينة", "description": "A4 طولي — الموقع والتواصل ورمز الإعلان" }),
            serde_json::json!({ "id": "commercial-offer", "name": "العرض التجاري", "description": "A4 طولي — الواجهة وعرض الشارع والاستخدام" }),
            serde_json::json!({ "id": "marketing-report", "name": "التقرير التسويقي", "description": "A4 طولي — وصف منظم وترخيص الإعلان" }),
            serde_json::json!({ "id": "emerald-card", "name": "نشرة الإطلاق", "description": "فلاير 120×180 للطباعة السريعة" }),
            serde_json::json!({ "id": "template1", "name": "أرشيف سابق", "description": "للتوافق مع الأرشيف السابق" }),
            serde_json::json!({ "id": "template2", "name": "أرشيف سابق", "description": "للتوافق مع الأرشيف السابق" }),
            serde_json::json!({ "id": "template3", "name": "أرشيف سابق", "description": "للتوافق مع الأرشيف السابق" }),
        ]
    }

    pub fn list(&self, user_id: &str, limit: i64, offset: i64) -> Result<Vec<serde_json::Value>, String> {
        self.db.list_brochures(user_id, limit, offset)
    }

    pub fn get(&self, id: &str, user_id: &str) -> Result<serde_json::Value, String> {
        self.db
            .get_brochure(id, user_id)?
            .ok_or("الملف غير موجود في مكتبة العروض".to_string())
    }

    pub fn create(
        &self,
        user_id: &str,
        property_id: Option<&str>,
        template_id: &str,
        title: Option<&str>,
        content: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let template = if template_id.is_empty() {
            "royal-sheet"
        } else {
            template_id
        };
        let resolved_property_id = property_id
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let mut resolved_title = title.unwrap_or("تصميم عرض").to_string();
        let mut content_value = content.cloned().unwrap_or_else(|| serde_json::json!({}));

        if let Some(ref pid) = resolved_property_id {
            let property = self
                .db
                .get_property(pid, user_id)?
                .ok_or("العقار غير موجود".to_string())?;
            if title.is_none() || title.unwrap_or("").is_empty() {
                resolved_title = property.name.clone();
            }
            if content.is_none() {
                // المحتوى يُبنى من النسخة القابلة للمشاركة فقط:
                // جوال المالك والسعر الداخلي والعمولة لا تدخل ملف العرض.
                let mut base = property_to_shareable_json(&property);
                if let Some(map) = base.as_object_mut() {
                    map.insert("property_name".to_string(), serde_json::json!(property.name));
                    map.insert("features".to_string(), serde_json::json!([]));
                    map.insert("images".to_string(), serde_json::json!([]));
                    map.insert("broker_info".to_string(), serde_json::json!({}));
                }
                content_value = base;
            }
        } else if content.is_none() {
            return Err("اربط عقارًا أو أدخل محتوى التصميم قبل الحفظ".to_string());
        }

        let id = Uuid::new_v4().to_string();
        let brochure = Brochure {
            context: crate::country::RecordContext::default(),
            id: id.clone(),
            user_id: user_id.to_string(),
            property_id: resolved_property_id.clone(),
            template_id: Some(template.to_string()),
            title: Some(resolved_title.clone()),
            content: Some(serde_json::to_string(&content_value).unwrap_or("{}".to_string())),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.db.create_brochure(&brochure)?;

        self.db.create_audit_log(&AuditLog {
            id: Uuid::new_v4().to_string(),
            user_id: Some(user_id.to_string()),
            action: "brochure_create".to_string(),
            entity_type: Some("brochure".to_string()),
            entity_id: Some(id.clone()),
            details: Some(format!("حفظ في مكتبة العروض: {}", resolved_title)),
            created_at: chrono::Utc::now().to_rfc3339(),
        })?;

        Ok(serde_json::json!({
            "id": id,
            "user_id": user_id,
            "property_id": resolved_property_id,
            "template_id": template,
            "title": resolved_title,
            "content": content_value,
            "created_at": brochure.created_at
        }))
    }

    pub fn update(
        &self,
        id: &str,
        user_id: &str,
        template_id: Option<&str>,
        title: Option<&str>,
        content: Option<&serde_json::Value>,
        property_id: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        let existing = self
            .db
            .get_brochure(id, user_id)?
            .ok_or("الملف غير موجود في مكتبة العروض".to_string())?;

        let next_template = template_id
            .map(|s| s.to_string())
            .or_else(|| {
                existing
                    .get("template_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "royal-sheet".to_string());
        let next_title = title
            .map(|s| s.to_string())
            .or_else(|| {
                existing
                    .get("title")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "تصميم عرض".to_string());
        let next_property = if let Some(pid) = property_id {
            let trimmed = pid.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        } else {
            existing
                .get("property_id")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
        };
        let next_content = if let Some(c) = content {
            serde_json::to_string(c).unwrap_or("{}".to_string())
        } else if let Some(c) = existing.get("content") {
            if c.is_string() {
                c.as_str().unwrap_or("{}").to_string()
            } else {
                serde_json::to_string(c).unwrap_or("{}".to_string())
            }
        } else {
            "{}".to_string()
        };

        self.db.update_brochure(
            id,
            user_id,
            next_property.as_deref(),
            &next_template,
            &next_title,
            &next_content,
        )?;

        self.get(id, user_id)
    }

    pub fn delete(&self, id: &str, user_id: &str) -> Result<serde_json::Value, String> {
        self.db.delete_brochure(id, user_id)?;
        Ok(serde_json::json!({ "message": "تم الحذف من مكتبة العروض" }))
    }
}

// ===== خدمات الحملات التسويقية =====

/// لقطة قراءة قصيرة قبل HTTP لمزامنة التتبع — تُرفض الكتابة إن تغيّرت.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackingSyncSnapshot {
    pub campaign_id: String,
    pub user_id: String,
    pub tracking_code: String,
    pub tracking_enabled: bool,
}

pub struct CampaignService<'a> {
    pub db: &'a dyn Database,
}

impl<'a> CampaignService<'a> {
    pub fn new(db: &'a dyn Database) -> Self {
        Self { db }
    }

    fn enrich(campaign: MarketingCampaign) -> serde_json::Value {
        let metrics = compute_campaign_metrics(
            campaign.views,
            campaign.inquiries,
            campaign.viewings,
            campaign.deals,
            campaign.cost,
        );
        let mut value = serde_json::to_value(&campaign).unwrap_or_else(|_| serde_json::json!({}));
        if let Some(obj) = value.as_object_mut() {
            if let Some(metrics_obj) = metrics_to_json(&metrics).as_object() {
                for (k, v) in metrics_obj {
                    obj.insert(k.clone(), v.clone());
                }
            }
        }
        value
    }

    fn parse_payload(
        &self,
        user_id: &str,
        data: &serde_json::Value,
        existing: Option<&MarketingCampaign>,
    ) -> Result<MarketingCampaign, String> {
        let name = json_opt_str(data, "name")
            .or_else(|| existing.map(|c| c.name.clone()))
            .unwrap_or_default();
        let name = name.trim().to_string();
        if name.chars().count() < 2 {
            return Err("اسم الحملة مطلوب (حرفان على الأقل).".to_string());
        }
        if name.chars().count() > 200 {
            return Err("اسم الحملة أطول من الحد المسموح.".to_string());
        }

        let platform = json_opt_str(data, "platform")
            .or_else(|| existing.map(|c| c.platform.clone()))
            .unwrap_or_default();
        let linked_property = match json_opt_str(data, "property_id") { Some(id) => self.db.get_property(&id, user_id)?, None => None };
        let country = existing.map(|r| r.context.country.clone()).or_else(|| linked_property.map(|p|p.context.country)).unwrap_or(crate::country::RecordContext::from_settings(&self.db.get_settings(user_id)?)?.country);
        let profile = crate::country::profile(&country)?;
        let catalog = crate::country::catalog();
        let allowed = profile["platforms"].as_array().unwrap().iter().chain(catalog["socialPlatforms"].as_array().unwrap().iter()).any(|p| p["id"] == platform);
        if !allowed && !existing.is_some_and(|r| r.platform == platform) {
            return Err("المنصة غير معروفة. اختر منصة من القائمة.".to_string());
        }

        let property_id = if data.get("property_id").is_some() {
            json_opt_str(data, "property_id")
        } else {
            existing.and_then(|c| c.property_id.clone())
        };
        if let Some(ref pid) = property_id {
            self.db
                .get_property(pid, user_id)?
                .ok_or_else(|| "العقار/العرض المرتبط غير موجود.".to_string())?;
        }

        let ad_link = if data.get("ad_link").is_some() {
            validate_ad_link(data.get("ad_link").and_then(|v| v.as_str()))?
        } else {
            existing.and_then(|c| c.ad_link.clone())
        };

        let start_date = if data.get("start_date").is_some() {
            optional_iso_date(data.get("start_date").and_then(|v| v.as_str()))?
        } else {
            existing.and_then(|c| c.start_date.clone())
        };
        let end_date = if data.get("end_date").is_some() {
            optional_iso_date(data.get("end_date").and_then(|v| v.as_str()))?
        } else {
            existing.and_then(|c| c.end_date.clone())
        };
        if let (Some(start), Some(end)) = (&start_date, &end_date) {
            if end < start {
                return Err("تاريخ النهاية يجب أن يكون بعد تاريخ البداية أو مساويًا له.".to_string());
            }
        }

        let views = json_nonneg_i64(data, "views", existing.map(|c| c.views).unwrap_or(0))?;
        let inquiries = json_nonneg_i64(data, "inquiries", existing.map(|c| c.inquiries).unwrap_or(0))?;
        let viewings = json_nonneg_i64(data, "viewings", existing.map(|c| c.viewings).unwrap_or(0))?;
        let deals = json_nonneg_i64(data, "deals", existing.map(|c| c.deals).unwrap_or(0))?;
        let cost = json_nonneg_f64(data, "cost", existing.map(|c| c.cost).unwrap_or(0.0))?;

        let notes = if data.get("notes").is_some() {
            json_opt_str(data, "notes")
        } else {
            existing.and_then(|c| c.notes.clone())
        };

        let now = chrono::Utc::now().to_rfc3339();
        Ok(MarketingCampaign {
            context: crate::country::RecordContext::default(),
            id: existing
                .map(|c| c.id.clone())
                .unwrap_or_else(|| Uuid::new_v4().to_string()),
            user_id: user_id.to_string(),
            name,
            property_id,
            platform,
            ad_link,
            start_date,
            end_date,
            views,
            inquiries,
            viewings,
            deals,
            cost,
            notes,
            created_at: existing
                .map(|c| c.created_at.clone())
                .unwrap_or_else(|| now.clone()),
            updated_at: now,
            property_name: None,
            original_ad_url: existing.and_then(|c| c.original_ad_url.clone()),
            tracking_code: existing.and_then(|c| c.tracking_code.clone()),
            tracking_url: existing.and_then(|c| c.tracking_url.clone()),
            tracked_clicks: existing.map(|c| c.tracked_clicks).unwrap_or(0),
            unique_clicks: existing.map(|c| c.unique_clicks).unwrap_or(0),
            last_click_at: existing.and_then(|c| c.last_click_at.clone()),
            tracking_enabled: existing.map(|c| c.tracking_enabled).unwrap_or(false),
        })
    }

    pub fn list(
        &self,
        user_id: &str,
        page: i64,
        limit: i64,
        platform: &str,
        property_id: &str,
        period_from: &str,
        period_to: &str,
    ) -> Result<serde_json::Value, String> {
        let page = page.max(1);
        let limit = limit.clamp(1, 200);
        let offset = (page - 1) * limit;
        let (items, total) = self.db.list_campaigns(
            user_id,
            platform,
            property_id,
            period_from,
            period_to,
            limit,
            offset,
        )?;
        let items: Vec<serde_json::Value> = items.into_iter().map(Self::enrich).collect();
        Ok(serde_json::json!({
            "items": items,
            "total": total,
            "page": page,
            "limit": limit,
        }))
    }

    pub fn stats(
        &self,
        user_id: &str,
        platform: &str,
        property_id: &str,
        period_from: &str,
        period_to: &str,
    ) -> Result<serde_json::Value, String> {
        let totals = self
            .db
            .get_campaign_totals(user_id, platform, property_id, period_from, period_to)?;
        let views = totals.get("views").and_then(|v| v.as_i64()).unwrap_or(0);
        let inquiries = totals.get("inquiries").and_then(|v| v.as_i64()).unwrap_or(0);
        let viewings = totals.get("viewings").and_then(|v| v.as_i64()).unwrap_or(0);
        let deals = totals.get("deals").and_then(|v| v.as_i64()).unwrap_or(0);
        let cost = totals.get("cost").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let metrics = compute_campaign_metrics(views, inquiries, viewings, deals, cost);
        let mut out = totals;
        if let Some(obj) = out.as_object_mut() {
            if let Some(metrics_obj) = metrics_to_json(&metrics).as_object() {
                for (k, v) in metrics_obj {
                    obj.insert(k.clone(), v.clone());
                }
            }
        }
        Ok(out)
    }

    pub fn get(&self, id: &str, user_id: &str) -> Result<serde_json::Value, String> {
        let campaign = self
            .db
            .get_campaign(id, user_id)?
            .ok_or_else(|| "الحملة غير موجودة".to_string())?;
        Ok(Self::enrich(campaign))
    }

    pub fn create(&self, user_id: &str, data: &serde_json::Value) -> Result<serde_json::Value, String> {
        let campaign = self.parse_payload(user_id, data, None)?;
        self.db.create_campaign(&campaign)?;
        self.get(&campaign.id, user_id)
    }

    /// تحديث محلي فقط — بدون HTTP. إن لزم تحديث الوجهة عن بُعد يُعاد الرابط للمرحلة الخارجية.
    pub fn update(
        &self,
        id: &str,
        user_id: &str,
        data: &serde_json::Value,
    ) -> Result<(serde_json::Value, Option<String>), String> {
        let existing = self
            .db
            .get_campaign(id, user_id)?
            .ok_or_else(|| "الحملة غير موجودة".to_string())?;
        let mut campaign = self.parse_payload(user_id, data, Some(&existing))?;
        let mut remote_destination: Option<String> = None;
        if campaign.tracking_enabled {
            if let Some(link) = campaign.ad_link.clone() {
                if let Ok(original) = crate::tracking_client::require_https_destination(&link) {
                    campaign.original_ad_url = Some(original.clone());
                    // سلوك سابق: محاولة تحديث الوجهة عبر الشبكة عند كل حفظ والحملة مفعّلة تتبعًا.
                    // HTTP يُنفَّذ بعد تحرير قفل قاعدة البيانات.
                    remote_destination = Some(original);
                }
            }
        }
        self.db.update_campaign(&campaign)?;
        Ok((self.get(id, user_id)?, remote_destination))
    }

    /// تطبيق رمز/رابط تتبع جديد بعد نجاح HTTP خارج القفل (تحديث وجهة).
    pub fn apply_remote_tracking_registration(
        &self,
        id: &str,
        user_id: &str,
        registered: &crate::tracking_client::RegisteredLink,
    ) -> Result<serde_json::Value, String> {
        let mut campaign = self
            .db
            .get_campaign(id, user_id)?
            .ok_or_else(|| "الحملة غير موجودة".to_string())?;
        campaign.tracking_code = Some(registered.tracking_code.clone());
        campaign.tracking_url = Some(registered.tracking_url.clone());
        campaign.updated_at = chrono::Utc::now().to_rfc3339();
        self.db.update_campaign(&campaign)?;
        self.get(id, user_id)
    }

    pub fn tracking_status(&self) -> serde_json::Value {
        serde_json::json!({
            "available": crate::tracking_client::is_tracking_configured()
        })
    }

    /// مرحلة 1 لتفعيل التتبع: قراءة قصيرة من DB فقط.
    pub fn tracking_enable_load_destination(
        &self,
        id: &str,
        user_id: &str,
    ) -> Result<String, String> {
        let campaign = self
            .db
            .get_campaign(id, user_id)?
            .ok_or_else(|| "الحملة غير موجودة".to_string())?;
        let source = campaign
            .ad_link
            .clone()
            .or_else(|| campaign.original_ad_url.clone())
            .ok_or_else(|| "أدخل رابط إعلان يبدأ بـ https://".to_string())?;
        crate::tracking_client::require_https_destination(&source)
    }

    /// مرحلة 3 لتفعيل التتبع: كتابة قصيرة بعد نجاح HTTP.
    /// تُرفض إن حُذفت الحملة أثناء الشبكة.
    pub fn tracking_enable_persist(
        &self,
        id: &str,
        user_id: &str,
        original: &str,
        registered: &crate::tracking_client::RegisteredLink,
    ) -> Result<(), String> {
        let mut campaign = self
            .db
            .get_campaign(id, user_id)?
            .ok_or_else(|| "الحملة غير موجودة".to_string())?;
        campaign.ad_link = Some(original.to_string());
        campaign.original_ad_url = Some(original.to_string());
        campaign.tracking_code = Some(registered.tracking_code.clone());
        campaign.tracking_url = Some(registered.tracking_url.clone());
        campaign.tracking_enabled = true;
        campaign.updated_at = chrono::Utc::now().to_rfc3339();
        self.db.update_campaign(&campaign)?;
        Ok(())
    }

    /// مرحلة 1 لتعطيل التتبع: قراءة الرمز فقط.
    pub fn tracking_disable_load_code(
        &self,
        id: &str,
        user_id: &str,
    ) -> Result<Option<String>, String> {
        let campaign = self
            .db
            .get_campaign(id, user_id)?
            .ok_or_else(|| "الحملة غير موجودة".to_string())?;
        Ok(campaign.tracking_code.filter(|c| !c.is_empty()))
    }

    /// مرحلة 3 لتعطيل التتبع: إيقاف محلي بعد HTTP (أو بدونه).
    pub fn tracking_disable_persist(
        &self,
        id: &str,
        user_id: &str,
    ) -> Result<serde_json::Value, String> {
        let mut campaign = self
            .db
            .get_campaign(id, user_id)?
            .ok_or_else(|| "الحملة غير موجودة".to_string())?;
        campaign.tracking_enabled = false;
        campaign.updated_at = chrono::Utc::now().to_rfc3339();
        self.db.update_campaign(&campaign)?;
        self.get(id, user_id)
    }

    /// مرحلة 1 لمزامنة الإحصاءات: قراءة لقطة التتبع فقط.
    pub fn tracking_sync_load_snapshot(
        &self,
        id: &str,
        user_id: &str,
    ) -> Result<TrackingSyncSnapshot, String> {
        let campaign = self
            .db
            .get_campaign(id, user_id)?
            .ok_or_else(|| "الحملة غير موجودة".to_string())?;
        let tracking_code = campaign
            .tracking_code
            .clone()
            .filter(|c| !c.is_empty())
            .ok_or_else(|| "لا يوجد رابط تتبع لهذه الحملة.".to_string())?;
        Ok(TrackingSyncSnapshot {
            campaign_id: campaign.id,
            user_id: campaign.user_id,
            tracking_code,
            tracking_enabled: campaign.tracking_enabled,
        })
    }

    /// توافق: رمز التتبع فقط من اللقطة.
    pub fn tracking_sync_load_code(&self, id: &str, user_id: &str) -> Result<String, String> {
        Ok(self.tracking_sync_load_snapshot(id, user_id)?.tracking_code)
    }

    /// مرحلة 3 لمزامنة الإحصاءات: كتابة مشروطة ببقاء اللقطة صالحة.
    /// إذا تغيّر الرمز/التفعيل أو حُذفت الحملة أثناء HTTP يُتجاهل الرد القديم.
    pub fn tracking_sync_apply_stats(
        &self,
        snapshot: &TrackingSyncSnapshot,
        stats: &crate::tracking_client::TrackingStats,
    ) -> Result<serde_json::Value, String> {
        let Some(mut campaign) = self
            .db
            .get_campaign(&snapshot.campaign_id, &snapshot.user_id)?
        else {
            // حُذفت أثناء HTTP — لا تطبيق على حالة جديدة
            return Err("الحملة غير موجودة".into());
        };
        let current_code = campaign
            .tracking_code
            .as_deref()
            .filter(|c| !c.is_empty())
            .unwrap_or("");
        let stale = campaign.id != snapshot.campaign_id
            || campaign.user_id != snapshot.user_id
            || current_code != snapshot.tracking_code
            || campaign.tracking_enabled != snapshot.tracking_enabled;
        if stale {
            // تجاهل الرد القديم وأعد الحالة الحالية دون تعديل الإحصاءات
            return self.get(&snapshot.campaign_id, &snapshot.user_id);
        }
        campaign.tracked_clicks = stats.tracked_clicks.max(0);
        campaign.unique_clicks = stats.unique_clicks.max(0);
        campaign.last_click_at = stats.last_click_at.clone();
        if !stats.enabled {
            campaign.tracking_enabled = false;
        }
        campaign.updated_at = chrono::Utc::now().to_rfc3339();
        self.db.update_campaign(&campaign)?;
        self.get(&snapshot.campaign_id, &snapshot.user_id)
    }

    /// قراءة رمز التتبع قبل الحذف (HTTP خارج القفل ثم delete_persist).
    pub fn delete_load_tracking_code(
        &self,
        id: &str,
        user_id: &str,
    ) -> Result<Option<String>, String> {
        Ok(self
            .db
            .get_campaign(id, user_id)?
            .and_then(|c| c.tracking_code)
            .filter(|c| !c.is_empty()))
    }

    pub fn delete_persist(&self, id: &str, user_id: &str) -> Result<serde_json::Value, String> {
        self.db.delete_campaign(id, user_id)?;
        Ok(serde_json::json!({ "message": "تم حذف الحملة" }))
    }

    /// توافق الاختبارات المباشرة: حذف بلا HTTP تحت القفل (HTTP يُستدعى من طبقة الأمر عند الحاجة).
    pub fn delete(&self, id: &str, user_id: &str) -> Result<serde_json::Value, String> {
        self.delete_persist(id, user_id)
    }
}

// ===== خدمات المواعيد =====

pub struct AppointmentService<'a> {
    pub db: &'a dyn Database,
}

impl<'a> AppointmentService<'a> {
    pub fn new(db: &'a dyn Database) -> Self {
        Self { db }
    }

    fn parse_payload(
        &self,
        user_id: &str,
        data: &serde_json::Value,
        existing: Option<&OfficeAppointment>,
    ) -> Result<OfficeAppointment, String> {
        let title = json_opt_str(data, "title")
            .or_else(|| existing.map(|a| a.title.clone()))
            .unwrap_or_default();
        let title = title.trim().to_string();
        if title.chars().count() < 2 {
            return Err("عنوان الموعد مطلوب (حرفان على الأقل).".to_string());
        }
        if title.chars().count() > 200 {
            return Err("عنوان الموعد أطول من الحد المسموح.".to_string());
        }

        let occurs_on = json_opt_str(data, "occurs_on")
            .or_else(|| existing.map(|a| a.occurs_on.clone()))
            .unwrap_or_default();
        parse_iso_date(&occurs_on)?;

        let starts_at = if data.get("starts_at").is_some() {
            validate_hhmm(data.get("starts_at").and_then(|v| v.as_str()))?
        } else {
            existing.and_then(|a| a.starts_at.clone())
        };

        let notes = if data.get("notes").is_some() {
            json_opt_str(data, "notes")
        } else {
            existing.and_then(|a| a.notes.clone())
        };

        let now = chrono::Utc::now().to_rfc3339();
        Ok(OfficeAppointment {
            id: existing
                .map(|a| a.id.clone())
                .unwrap_or_else(|| Uuid::new_v4().to_string()),
            user_id: user_id.to_string(),
            title,
            occurs_on,
            starts_at,
            notes,
            created_at: existing
                .map(|a| a.created_at.clone())
                .unwrap_or_else(|| now.clone()),
            updated_at: now,
        })
    }

    pub fn list(&self, user_id: &str, from: &str, to: &str) -> Result<Vec<OfficeAppointment>, String> {
        if !from.is_empty() {
            parse_iso_date(from)?;
        }
        if !to.is_empty() {
            parse_iso_date(to)?;
        }
        if !from.is_empty() && !to.is_empty() && to < from {
            return Err("نهاية الفترة يجب أن تكون بعد بدايتها.".to_string());
        }
        self.db.list_appointments(user_id, from, to)
    }

    pub fn get(&self, id: &str, user_id: &str) -> Result<serde_json::Value, String> {
        let item = self
            .db
            .get_appointment(id, user_id)?
            .ok_or_else(|| "الموعد غير موجود".to_string())?;
        serde_json::to_value(item).map_err(|e| e.to_string())
    }

    pub fn create(&self, user_id: &str, data: &serde_json::Value) -> Result<serde_json::Value, String> {
        let item = self.parse_payload(user_id, data, None)?;
        self.db.create_appointment(&item)?;
        self.get(&item.id, user_id)
    }

    pub fn update(
        &self,
        id: &str,
        user_id: &str,
        data: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let existing = self
            .db
            .get_appointment(id, user_id)?
            .ok_or_else(|| "الموعد غير موجود".to_string())?;
        let item = self.parse_payload(user_id, data, Some(&existing))?;
        self.db.update_appointment(&item)?;
        self.get(id, user_id)
    }

    pub fn delete(&self, id: &str, user_id: &str) -> Result<serde_json::Value, String> {
        self.db.delete_appointment(id, user_id)?;
        Ok(serde_json::json!({ "message": "تم حذف الموعد" }))
    }
}

// ===== خدمات التصدير =====

pub struct ExportService<'a> {
    pub db: &'a dyn Database,
}

impl<'a> ExportService<'a> {
    pub fn new(db: &'a dyn Database) -> Self {
        Self { db }
    }

    pub fn all(&self, user_id: &str) -> Result<serde_json::Value, String> {
        let data = self.db.export_all(user_id)?;
        self.db.create_audit_log(&AuditLog {
            id: Uuid::new_v4().to_string(),
            user_id: Some(user_id.to_string()),
            action: "export_data".to_string(),
            entity_type: Some("all".to_string()),
            entity_id: None,
            details: Some("تصدير بيانات المستخدم".to_string()),
            created_at: chrono::Utc::now().to_rfc3339(),
        })?;
        Ok(data)
    }

    pub fn properties(&self, user_id: &str) -> Result<Vec<Property>, String> {
        Ok(self.db.list_properties(user_id, "", "", "", "", "", 100000, 0)?.0)
    }

    pub fn clients(&self, user_id: &str) -> Result<Vec<Client>, String> {
        Ok(self.db.list_clients(user_id, "", "", "", "", false, 100000, 0)?.0)
    }

    pub fn market(&self, user_id: &str) -> Result<Vec<MarketReferenceData>, String> {
        Ok(self.db.list_market_data(user_id, "", "", "", 100000, 0)?.0)
    }

    /// استيراد يدوي لملف JSON بنفس بنية التصدير — يضيف السجلات لحساب المستخدم الحالي.
    pub fn import_all(&self, user_id: &str, payload: &serde_json::Value) -> Result<serde_json::Value, String> {
        let data = payload.get("data").unwrap_or(payload);
        let mut imported = serde_json::Map::new();
        let mut errors: Vec<String> = Vec::new();

        if let Some(arr) = data.get("market_reference_data").and_then(|v| v.as_array()) {
            let market = MarketService::new(self.db);
            let mut n = 0i64;
            for (i, item) in arr.iter().enumerate() {
                let mut body = item.clone();
                if let Some(obj) = body.as_object_mut() {
                    obj.remove("id");
                    obj.remove("user_id");
                    obj.remove("created_at");
                    obj.remove("seed_batch_id");
                }
                match market.create(user_id, &body) {
                    Ok(_) => n += 1,
                    Err(e) => errors.push(format!("market[{i}]: {e}")),
                }
            }
            imported.insert("market_reference_data".into(), serde_json::json!(n));
        }

        if let Some(arr) = data.get("properties").and_then(|v| v.as_array()) {
            let props = PropertyService::new(self.db);
            let mut n = 0i64;
            for (i, item) in arr.iter().enumerate() {
                let mut body = item.clone();
                if let Some(obj) = body.as_object_mut() {
                    obj.remove("id");
                    obj.remove("user_id");
                    obj.remove("created_at");
                    obj.remove("updated_at");
                    obj.remove("seed_batch_id");
                    obj.remove("images");
                }
                match props.create(user_id, &body) {
                    Ok(_) => n += 1,
                    Err(e) => errors.push(format!("properties[{i}]: {e}")),
                }
            }
            imported.insert("properties".into(), serde_json::json!(n));
        }

        if let Some(arr) = data.get("clients").and_then(|v| v.as_array()) {
            let clients = ClientService::new(self.db);
            let mut n = 0i64;
            for (i, item) in arr.iter().enumerate() {
                let mut body = item.clone();
                if let Some(obj) = body.as_object_mut() {
                    obj.remove("id");
                    obj.remove("user_id");
                    obj.remove("created_at");
                    obj.remove("updated_at");
                    obj.remove("seed_batch_id");
                }
                match clients.create(user_id, &body) {
                    Ok(_) => n += 1,
                    Err(e) => errors.push(format!("clients[{i}]: {e}")),
                }
            }
            imported.insert("clients".into(), serde_json::json!(n));
        }

        // دعم استيراد جزئي بنفس مفاتيح المسارات المنفصلة
        if data.get("market_reference_data").is_none() {
            if let Some(arr) = payload.as_array() {
                // ملف تصدير قسم واحد كمصفوفة — نخمّن من الحقول
                if arr.first().map(|x| x.get("sale_price").is_some() || x.get("price_per_meter_sale").is_some()).unwrap_or(false) {
                    let market = MarketService::new(self.db);
                    let mut n = 0i64;
                    for item in arr {
                        let mut body = item.clone();
                        if let Some(obj) = body.as_object_mut() {
                            obj.remove("id");
                            obj.remove("user_id");
                        }
                        if market.create(user_id, &body).is_ok() {
                            n += 1;
                        }
                    }
                    imported.insert("market_reference_data".into(), serde_json::json!(n));
                }
            }
        }

        self.db.create_audit_log(&AuditLog {
            id: Uuid::new_v4().to_string(),
            user_id: Some(user_id.to_string()),
            action: "import_data".to_string(),
            entity_type: Some("json".to_string()),
            entity_id: None,
            details: Some(format!("imported={imported:?}")),
            created_at: chrono::Utc::now().to_rfc3339(),
        })?;

        Ok(serde_json::json!({
            "imported": imported,
            "errors": errors,
            "ok": errors.is_empty()
        }))
    }
}
