// تنفيذ SQLite لواجهة قاعدة البيانات
// يمكن استبداله لاحقًا بتنفيذ PostgreSQL دون تغيير منطق التطبيق

use super::*;
use super::migrations;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

pub struct SqliteDatabase {
    conn: Connection,
}

impl SqliteDatabase {
    pub fn new(db_path: &Path) -> Result<Self, String> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let conn = super::open_configured(db_path)?;
        let mut db = SqliteDatabase { conn };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&mut self) -> Result<(), String> {
        migrations::run_migrations(&mut self.conn)?;
        Ok(())
    }

    pub fn schema_version(&self) -> Result<i32, String> {
        migrations::schema_version(&self.conn)
    }

    pub fn integrity_ok(&self) -> Result<bool, String> {
        let ok: String = self
            .conn
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        Ok(ok == "ok")
    }

    pub fn connection_path_hint(&self) -> String {
        "app_data_dir/mawasib.db".to_string()
    }
}

// ===== قوائم الأعمدة الصريحة =====
//
// لا يُستخدم `SELECT *` مع فهارس رقمية في هذا الملف.
// السبب: قواعد بيانات الإنتاج القديمة قد يختلف فيها ترتيب الأعمدة الفعلي
// عن ترتيب خط الأساس (مثال حقيقي: settings.updated_at قبل fal_license_number)،
// كما أن أي `ALTER TABLE ADD COLUMN` في Migration جديد يزحزح الفهارس.
// النتيجة كانت قراءة عمود خاطئ وظهور خطأ من نوع
// `Invalid column type Null` على fal_license_type.
// الحل الصحيح: تثبيت ترتيب الأعمدة في الاستعلام نفسه.

const USER_COLS: &str = "id, username, email, password_hash, full_name, phone, office_name, created_at";
const LICENSE_COLS: &str = "id, user_id, license_key, status, issued_at, expires_at";
const SESSION_COLS: &str = "id, user_id, token, device_name, created_at, expires_at";
const PROPERTY_COLS: &str = "id, user_id, name, city, district, location, property_type, area, price, \
     price_per_meter, ad_number, ad_link, owner_name, owner_phone, owner_contact, owner_notes, \
     marketing_platform, marketing_link, published_date, main_image, status, created_at, updated_at, \
     owner_client_id, source_type, purpose, usage_type, frontage, street_width, streets_count, \
     latitude, longitude, description, listing_expiry_date, commission_agreement, fal_ad_license, \
     internal_price, internal_notes, is_training, country, currency, broker_license_type, broker_license_name, broker_license_number";
const PROPERTY_IMAGE_COLS: &str = "id, property_id, image_url, created_at";
const PROPERTY_UPDATE_COLS: &str = "id, property_id, user_id, title, description, created_at";
const CLIENT_COLS: &str = "id, user_id, name, phone, whatsapp, relationship_type, city, interests, \
     required_property_type, budget, notes, last_contact, follow_up_date, created_at, updated_at, \
     email, phone_secondary, company, source, status, category, is_training, country, currency, broker_license_type, broker_license_name, broker_license_number";
const COMMUNICATION_COLS: &str = "id, user_id, client_id, type, subject, notes, created_at";
const CLIENT_PROPERTY_LINK_COLS: &str = "id, client_id, property_id, link_type, created_at";
const MARKET_COLS: &str = "id, user_id, city, district, property_type, usage_type, area, sale_price, \
     price_per_meter_sale, rent_value, rent_per_meter, requests_count, offers_count, entry_date, \
     data_source, notes, created_at, country, currency, broker_license_type, broker_license_name, broker_license_number";
const BROCHURE_COLS: &str = "id, user_id, property_id, template_id, title, content, created_at, updated_at, country, currency, broker_license_type, broker_license_name, broker_license_number";
const CAMPAIGN_COLS: &str = "id, user_id, name, property_id, platform, ad_link, start_date, end_date, \
     views, inquiries, viewings, deals, cost, notes, created_at, updated_at, \
     original_ad_url, tracking_code, tracking_url, tracked_clicks, unique_clicks, last_click_at, tracking_enabled, country, currency, broker_license_type, broker_license_name, broker_license_number";
const APPOINTMENT_COLS: &str =
    "id, user_id, title, occurs_on, starts_at, notes, created_at, updated_at";
const SETTINGS_COLS: &str = "user_id, office_name, office_logo, office_phone, office_whatsapp, \
     office_email, office_address, broker_name, broker_phone, broker_whatsapp, \
     default_brochure_template, fal_license_number, fal_license_type, updated_at, \
     ui_language, display_currency, tracking_endpoint_url, country, broker_license_name";
const AUDIT_LOG_COLS: &str = "id, user_id, action, entity_type, entity_id, details, created_at";

/// أعمدة البروشور مسبوقة بالبادئة `b.` للاستعلامات المدمجة مع properties.
fn brochure_cols_prefixed() -> String {
    BROCHURE_COLS
        .split(',')
        .map(|c| format!("b.{}", c.trim()))
        .collect::<Vec<_>>()
        .join(", ")
}

fn campaign_cols_prefixed() -> String {
    CAMPAIGN_COLS
        .split(',')
        .map(|c| format!("c.{}", c.trim()))
        .collect::<Vec<_>>()
        .join(", ")
}

fn campaign_filter_clause(
    platform: &str,
    property_id: &str,
    period_from: &str,
    period_to: &str,
    start_idx: usize,
) -> (String, Vec<Box<dyn rusqlite::ToSql>>, usize) {
    let mut clause = String::new();
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    let mut idx = start_idx;

    if !platform.is_empty() {
        clause.push_str(&format!(" AND c.platform = ?{idx}"));
        params.push(Box::new(platform.to_string()));
        idx += 1;
    }
    if !property_id.is_empty() {
        clause.push_str(&format!(" AND c.property_id = ?{idx}"));
        params.push(Box::new(property_id.to_string()));
        idx += 1;
    }
    if !period_from.is_empty() {
        clause.push_str(&format!(
            " AND date(COALESCE(NULLIF(c.start_date,''), date(c.created_at))) >= ?{idx}"
        ));
        params.push(Box::new(period_from.to_string()));
        idx += 1;
    }
    if !period_to.is_empty() {
        clause.push_str(&format!(
            " AND date(COALESCE(NULLIF(c.start_date,''), date(c.created_at))) <= ?{idx}"
        ));
        params.push(Box::new(period_to.to_string()));
        idx += 1;
    }

    (clause, params, idx)
}

fn row_to_campaign(row: &rusqlite::Row) -> rusqlite::Result<MarketingCampaign> {
    let tracking_enabled_i: i64 = row.get(22).unwrap_or(0);
    Ok(MarketingCampaign {
        context: crate::country::RecordContext::from_row(row)?,
        id: row.get(0)?,
        user_id: row.get(1)?,
        name: row.get(2)?,
        property_id: row.get(3)?,
        platform: row.get(4)?,
        ad_link: row.get(5)?,
        start_date: row.get(6)?,
        end_date: row.get(7)?,
        views: row.get(8)?,
        inquiries: row.get(9)?,
        viewings: row.get(10)?,
        deals: row.get(11)?,
        cost: row.get(12)?,
        notes: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
        original_ad_url: row.get(16)?,
        tracking_code: row.get(17)?,
        tracking_url: row.get(18)?,
        tracked_clicks: row.get(19).unwrap_or(0),
        unique_clicks: row.get(20).unwrap_or(0),
        last_click_at: row.get(21)?,
        tracking_enabled: tracking_enabled_i != 0,
        property_name: row.get::<_, Option<String>>(28).ok().flatten(),
    })
}

fn row_to_appointment(row: &rusqlite::Row) -> rusqlite::Result<OfficeAppointment> {
    Ok(OfficeAppointment {
        id: row.get(0)?,
        user_id: row.get(1)?,
        title: row.get(2)?,
        occurs_on: row.get(3)?,
        starts_at: row.get(4)?,
        notes: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

const FIN_OP_COLS: &str = "o.id, o.user_id, o.title, o.occurs_on, o.deal_value, o.client_id, o.property_id, \
     o.campaign_id, o.notes, o.created_at, o.updated_at, o.country, o.currency, o.broker_license_type, o.broker_license_name, o.broker_license_number";
const FIN_INV_COLS: &str = "i.id, i.user_id, i.number, i.title, i.issue_date, i.due_date, i.amount, \
     i.client_id, i.property_id, i.campaign_id, i.operation_id, i.notes, i.created_at, i.updated_at, i.country, i.currency, i.broker_license_type, i.broker_license_name, i.broker_license_number";
const FIN_COM_COLS: &str = "c.id, c.user_id, c.title, c.occurs_on, c.mode, c.basis_amount, c.rate_percent, \
     c.amount_fixed, c.office_share_pct, c.broker_share_pct, c.marketer_share_pct, c.partner_share_pct, \
     c.client_id, c.property_id, c.campaign_id, c.operation_id, c.invoice_id, c.notes, c.created_at, c.updated_at, c.country, c.currency, c.broker_license_type, c.broker_license_name, c.broker_license_number";
const FIN_EXP_COLS: &str = "e.id, e.user_id, e.title, e.occurs_on, e.amount, e.category, e.client_id, \
     e.property_id, e.campaign_id, e.reference, e.notes, e.created_at, e.updated_at, e.country, e.currency, e.broker_license_type, e.broker_license_name, e.broker_license_number";
const FIN_PAY_COLS: &str = "p.id, p.user_id, p.occurs_on, p.amount, p.target_type, p.target_id, p.method, \
     p.reference, p.notes, p.created_at, p.updated_at, p.country, p.currency, p.broker_license_type, p.broker_license_name, p.broker_license_number";

fn date_range_clause(
    date_expr: &str,
    period_from: &str,
    period_to: &str,
    start_idx: usize,
) -> (String, Vec<Box<dyn rusqlite::ToSql>>, usize) {
    let mut clause = String::new();
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    let mut idx = start_idx;
    if !period_from.is_empty() {
        clause.push_str(&format!(" AND date({date_expr}) >= ?{idx}"));
        params.push(Box::new(period_from.to_string()));
        idx += 1;
    }
    if !period_to.is_empty() {
        clause.push_str(&format!(" AND date({date_expr}) <= ?{idx}"));
        params.push(Box::new(period_to.to_string()));
        idx += 1;
    }
    (clause, params, idx)
}

fn row_to_finance_operation(row: &rusqlite::Row) -> rusqlite::Result<FinanceOperation> {
    Ok(FinanceOperation {
        context: crate::country::RecordContext::from_row(row)?,
        id: row.get(0)?,
        user_id: row.get(1)?,
        title: row.get(2)?,
        occurs_on: row.get(3)?,
        deal_value: row.get(4)?,
        client_id: row.get(5)?,
        property_id: row.get(6)?,
        campaign_id: row.get(7)?,
        notes: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        client_name: row.get(16)?,
        property_name: row.get(17)?,
        campaign_name: row.get(18)?,
    })
}

fn row_to_finance_invoice(row: &rusqlite::Row) -> rusqlite::Result<FinanceInvoice> {
    Ok(FinanceInvoice {
        context: crate::country::RecordContext::from_row(row)?,
        id: row.get(0)?,
        user_id: row.get(1)?,
        number: row.get(2)?,
        title: row.get(3)?,
        issue_date: row.get(4)?,
        due_date: row.get(5)?,
        amount: row.get(6)?,
        client_id: row.get(7)?,
        property_id: row.get(8)?,
        campaign_id: row.get(9)?,
        operation_id: row.get(10)?,
        notes: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
        client_name: row.get(19)?,
        property_name: row.get(20)?,
        campaign_name: row.get(21)?,
        operation_title: row.get(22)?,
        paid: row.get::<_, f64>(23)?,
    })
}

fn row_to_finance_commission(row: &rusqlite::Row) -> rusqlite::Result<FinanceCommission> {
    Ok(FinanceCommission {
        context: crate::country::RecordContext::from_row(row)?,
        id: row.get(0)?,
        user_id: row.get(1)?,
        title: row.get(2)?,
        occurs_on: row.get(3)?,
        mode: row.get(4)?,
        basis_amount: row.get(5)?,
        rate_percent: row.get(6)?,
        amount_fixed: row.get(7)?,
        office_share_pct: row.get(8)?,
        broker_share_pct: row.get(9)?,
        marketer_share_pct: row.get(10)?,
        partner_share_pct: row.get(11)?,
        client_id: row.get(12)?,
        property_id: row.get(13)?,
        campaign_id: row.get(14)?,
        operation_id: row.get(15)?,
        invoice_id: row.get(16)?,
        notes: row.get(17)?,
        created_at: row.get(18)?,
        updated_at: row.get(19)?,
        client_name: row.get(25)?,
        property_name: row.get(26)?,
        campaign_name: row.get(27)?,
        operation_title: row.get(28)?,
        invoice_number: row.get(29)?,
        paid: row.get::<_, f64>(30)?,
    })
}

fn row_to_finance_expense(row: &rusqlite::Row) -> rusqlite::Result<FinanceExpense> {
    Ok(FinanceExpense {
        context: crate::country::RecordContext::from_row(row)?,
        id: row.get(0)?,
        user_id: row.get(1)?,
        title: row.get(2)?,
        occurs_on: row.get(3)?,
        amount: row.get(4)?,
        category: row.get(5)?,
        client_id: row.get(6)?,
        property_id: row.get(7)?,
        campaign_id: row.get(8)?,
        reference: row.get(9)?,
        notes: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
        client_name: row.get(18)?,
        property_name: row.get(19)?,
        campaign_name: row.get(20)?,
    })
}

fn row_to_finance_payment(row: &rusqlite::Row) -> rusqlite::Result<FinancePayment> {
    Ok(FinancePayment {
        context: crate::country::RecordContext::from_row(row)?,
        id: row.get(0)?,
        user_id: row.get(1)?,
        occurs_on: row.get(2)?,
        amount: row.get(3)?,
        target_type: row.get(4)?,
        target_id: row.get(5)?,
        method: row.get(6)?,
        reference: row.get(7)?,
        notes: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        target_title: row.get(16)?,
    })
}

// ===== دوال مساعدة لتحويل الصفوف =====

fn row_to_user(row: &rusqlite::Row) -> rusqlite::Result<User> {
    Ok(User {
        id: row.get(0)?,
        username: row.get(1)?,
        email: row.get(2)?,
        password_hash: row.get(3)?,
        full_name: row.get(4)?,
        phone: row.get(5)?,
        office_name: row.get(6)?,
        created_at: row.get(7)?,
    })
}

fn row_to_license(row: &rusqlite::Row) -> rusqlite::Result<License> {
    Ok(License {
        id: row.get(0)?,
        user_id: row.get(1)?,
        license_key: row.get(2)?,
        status: row.get(3)?,
        issued_at: row.get(4)?,
        expires_at: row.get(5)?,
    })
}

fn row_to_session(row: &rusqlite::Row) -> rusqlite::Result<Session> {
    Ok(Session {
        id: row.get(0)?,
        user_id: row.get(1)?,
        token: row.get(2)?,
        device_name: row.get(3)?,
        created_at: row.get(4)?,
        expires_at: row.get(5)?,
    })
}

fn row_to_property(row: &rusqlite::Row) -> rusqlite::Result<Property> {
    // الفهارس مرتبطة بترتيب PROPERTY_COLS حصراً.
    Ok(Property {
        context: crate::country::RecordContext::from_row(row)?,
        id: row.get(0)?,
        user_id: row.get(1)?,
        name: row.get(2)?,
        city: row.get(3)?,
        district: row.get(4)?,
        location: row.get(5)?,
        property_type: row.get(6)?,
        area: row.get(7)?,
        price: row.get(8)?,
        price_per_meter: row.get(9)?,
        ad_number: row.get(10)?,
        ad_link: row.get(11)?,
        owner_name: row.get(12)?,
        owner_phone: row.get(13)?,
        owner_contact: row.get(14)?,
        owner_notes: row.get(15)?,
        marketing_platform: row.get(16)?,
        marketing_link: row.get(17)?,
        published_date: row.get(18)?,
        main_image: row.get(19)?,
        status: row.get(20)?,
        created_at: row.get(21)?,
        updated_at: row.get(22)?,
        owner_client_id: row.get(23)?,
        source_type: row.get(24)?,
        purpose: row.get(25)?,
        usage_type: row.get(26)?,
        frontage: row.get(27)?,
        street_width: row.get(28)?,
        streets_count: row.get(29)?,
        latitude: row.get(30)?,
        longitude: row.get(31)?,
        description: row.get(32)?,
        listing_expiry_date: row.get(33)?,
        commission_agreement: row.get(34)?,
        fal_ad_license: row.get(35)?,
        internal_price: row.get(36)?,
        internal_notes: row.get(37)?,
        is_training: row.get::<_, i64>(38)? != 0,
    })
}

fn row_to_property_image(row: &rusqlite::Row) -> rusqlite::Result<PropertyImage> {
    Ok(PropertyImage {
        id: row.get(0)?,
        property_id: row.get(1)?,
        image_url: row.get(2)?,
        created_at: row.get(3)?,
    })
}

fn row_to_property_update(row: &rusqlite::Row) -> rusqlite::Result<PropertyUpdate> {
    Ok(PropertyUpdate {
        id: row.get(0)?,
        property_id: row.get(1)?,
        user_id: row.get(2)?,
        title: row.get(3)?,
        description: row.get(4)?,
        created_at: row.get(5)?,
    })
}

fn row_to_client(row: &rusqlite::Row) -> rusqlite::Result<Client> {
    Ok(Client {
        context: crate::country::RecordContext::from_row(row)?,
        id: row.get(0)?,
        user_id: row.get(1)?,
        name: row.get(2)?,
        phone: row.get(3)?,
        whatsapp: row.get(4)?,
        relationship_type: row.get(5)?,
        city: row.get(6)?,
        interests: row.get(7)?,
        required_property_type: row.get(8)?,
        budget: row.get(9)?,
        notes: row.get(10)?,
        last_contact: row.get(11)?,
        follow_up_date: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
        email: row.get(15)?,
        phone_secondary: row.get(16)?,
        company: row.get(17)?,
        source: row.get(18)?,
        status: row.get(19)?,
        category: row.get(20)?,
        is_training: row.get::<_, i64>(21)? != 0,
    })
}

fn row_to_communication(row: &rusqlite::Row) -> rusqlite::Result<Communication> {
    Ok(Communication {
        id: row.get(0)?,
        user_id: row.get(1)?,
        client_id: row.get(2)?,
        r#type: row.get(3)?,
        subject: row.get(4)?,
        notes: row.get(5)?,
        created_at: row.get(6)?,
    })
}

fn row_to_client_property_link(row: &rusqlite::Row) -> rusqlite::Result<ClientPropertyLink> {
    Ok(ClientPropertyLink {
        id: row.get(0)?,
        client_id: row.get(1)?,
        property_id: row.get(2)?,
        link_type: row.get(3)?,
        created_at: row.get(4)?,
    })
}

fn row_to_market_data(row: &rusqlite::Row) -> rusqlite::Result<MarketReferenceData> {
    Ok(MarketReferenceData {
        context: crate::country::RecordContext::from_row(row)?,
        id: row.get(0)?,
        user_id: row.get(1)?,
        city: row.get(2)?,
        district: row.get(3)?,
        property_type: row.get(4)?,
        usage_type: row.get(5)?,
        area: row.get(6)?,
        sale_price: row.get(7)?,
        price_per_meter_sale: row.get(8)?,
        rent_value: row.get(9)?,
        rent_per_meter: row.get(10)?,
        requests_count: row.get(11)?,
        offers_count: row.get(12)?,
        entry_date: row.get(13)?,
        data_source: row.get(14)?,
        notes: row.get(15)?,
        created_at: row.get(16)?,
    })
}

fn row_to_brochure(row: &rusqlite::Row) -> rusqlite::Result<Brochure> {
    Ok(Brochure {
        context: crate::country::RecordContext::from_row(row)?,
        id: row.get(0)?,
        user_id: row.get(1)?,
        property_id: row.get(2)?,
        template_id: row.get(3)?,
        title: row.get(4)?,
        content: row.get(5)?,
        created_at: row.get(6)?,
    })
}

fn row_to_settings(row: &rusqlite::Row) -> rusqlite::Result<Settings> {
    Ok(Settings {
        country: row.get("country")?,
        broker_license_name: row.get("broker_license_name")?,
        user_id: row.get(0)?,
        office_name: row.get(1)?,
        office_logo: row.get(2)?,
        office_phone: row.get(3)?,
        office_whatsapp: row.get(4)?,
        office_email: row.get(5)?,
        office_address: row.get(6)?,
        broker_name: row.get(7)?,
        broker_phone: row.get(8)?,
        broker_whatsapp: row.get(9)?,
        default_brochure_template: row.get(10)?,
        fal_license_number: row.get(11)?,
        fal_license_type: row.get(12)?,
        updated_at: row.get(13)?,
        ui_language: row.get(14)?,
        display_currency: row.get(15)?,
        tracking_endpoint_url: row.get(16).ok().flatten(),
    })
}

fn row_to_audit_log(row: &rusqlite::Row) -> rusqlite::Result<AuditLog> {
    Ok(AuditLog {
        id: row.get(0)?,
        user_id: row.get(1)?,
        action: row.get(2)?,
        entity_type: row.get(3)?,
        entity_id: row.get(4)?,
        details: row.get(5)?,
        created_at: row.get(6)?,
    })
}

// ===== تنفيذ الواجهة =====

impl Database for SqliteDatabase {
    // ===== Users =====
    fn create_user(&self, user: &User) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO users (id, username, email, password_hash, full_name, phone, office_name) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![user.id, user.username, user.email, user.password_hash, user.full_name, user.phone, user.office_name],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn get_user_by_username_or_email(&self, username: &str, email: &str) -> Result<Option<User>, String> {
        let mut stmt = self
            .conn
            .prepare(&format!("SELECT {USER_COLS} FROM users WHERE username = ?1 OR email = ?2"))
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![username, email]).map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            Ok(Some(row_to_user(row).map_err(|e| e.to_string())?))
        } else {
            Ok(None)
        }
    }

    fn get_user_by_id(&self, id: &str) -> Result<Option<User>, String> {
        let mut stmt = self
            .conn
            .prepare(&format!("SELECT {USER_COLS} FROM users WHERE id = ?1"))
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![id]).map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            Ok(Some(row_to_user(row).map_err(|e| e.to_string())?))
        } else {
            Ok(None)
        }
    }

    fn get_first_user(&self) -> Result<Option<User>, String> {
        let mut stmt = self
            .conn
            .prepare(&format!("SELECT {USER_COLS} FROM users ORDER BY created_at ASC LIMIT 1"))
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            Ok(Some(row_to_user(row).map_err(|e| e.to_string())?))
        } else {
            Ok(None)
        }
    }

    fn count_users(&self) -> Result<i64, String> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        Ok(count)
    }

    // ===== Licenses =====
    fn create_license(&self, license: &License) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO licenses (id, user_id, license_key, status, issued_at, expires_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![license.id, license.user_id, license.license_key, license.status, license.issued_at, license.expires_at],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn update_license_id(&self, new_id: &str, old_id: &str) -> Result<(), String> {
        self.conn
            .execute("UPDATE licenses SET id = ?1 WHERE id = ?2", params![new_id, old_id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn get_license_by_user_id(&self, user_id: &str) -> Result<Option<License>, String> {
        let mut stmt = self
            .conn
            .prepare(&format!("SELECT {LICENSE_COLS} FROM licenses WHERE user_id = ?1"))
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![user_id]).map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            Ok(Some(row_to_license(row).map_err(|e| e.to_string())?))
        } else {
            Ok(None)
        }
    }

    fn get_license_by_key(&self, license_key: &str) -> Result<Option<License>, String> {
        let mut stmt = self
            .conn
            .prepare(&format!("SELECT {LICENSE_COLS} FROM licenses WHERE license_key = ?1"))
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![license_key]).map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            Ok(Some(row_to_license(row).map_err(|e| e.to_string())?))
        } else {
            Ok(None)
        }
    }

    // ===== Sessions =====
    fn create_session(&self, session: &Session) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO sessions (id, user_id, token, device_name, expires_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![session.id, session.user_id, session.token, session.device_name, session.expires_at],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn get_session_by_token(&self, token: &str) -> Result<Option<Session>, String> {
        let mut stmt = self
            .conn
            .prepare(&format!("SELECT {SESSION_COLS} FROM sessions WHERE token = ?1"))
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![token]).map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            Ok(Some(row_to_session(row).map_err(|e| e.to_string())?))
        } else {
            Ok(None)
        }
    }

    fn delete_session_by_token(&self, token: &str) -> Result<(), String> {
        self.conn
            .execute("DELETE FROM sessions WHERE token = ?1", params![token])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ===== Training data =====
    fn has_training_data(&self) -> Result<bool, String> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM seed_batches", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        Ok(count > 0)
    }

    // ===== Properties =====
    fn list_properties(&self, user_id: &str, search: &str, status: &str, city: &str, property_type: &str, purpose: &str, limit: i64, offset: i64) -> Result<(Vec<Property>, i64), String> {
        let mut where_clause = String::from("WHERE user_id = ?1");
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(user_id.to_string())];
        let mut idx = 2;

        if !search.is_empty() {
            where_clause.push_str(&format!(
                " AND (name LIKE ?{i} OR city LIKE ?{i} OR district LIKE ?{i} OR location LIKE ?{i} \
                 OR ad_number LIKE ?{i} OR fal_ad_license LIKE ?{i} OR owner_name LIKE ?{i})",
                i = idx
            ));
            // معامل واحد لكل الحقول: التكرار في القائمة يُفسد ترقيم rusqlite.
            params_vec.push(Box::new(format!("%{}%", search)));
            idx += 1;
        }
        if !status.is_empty() {
            where_clause.push_str(&format!(" AND status = ?{}", idx));
            params_vec.push(Box::new(status.to_string()));
            idx += 1;
        }
        if !city.is_empty() {
            where_clause.push_str(&format!(" AND city = ?{}", idx));
            params_vec.push(Box::new(city.to_string()));
            idx += 1;
        }
        if !property_type.is_empty() {
            where_clause.push_str(&format!(" AND property_type = ?{}", idx));
            params_vec.push(Box::new(property_type.to_string()));
            idx += 1;
        }
        if !purpose.is_empty() {
            where_clause.push_str(&format!(" AND purpose = ?{}", idx));
            params_vec.push(Box::new(purpose.to_string()));
            idx += 1;
        }

        let count_sql = format!("SELECT COUNT(*) FROM properties {}", where_clause);
        let total: i64 = self
            .conn
            .query_row(&count_sql, rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())), |r| r.get(0))
            .map_err(|e| e.to_string())?;

        let list_sql = format!("SELECT {} FROM properties {} ORDER BY created_at DESC LIMIT ?{} OFFSET ?{}", PROPERTY_COLS, where_clause, idx, idx + 1);
        let mut all_params: Vec<Box<dyn rusqlite::ToSql>> = params_vec;
        all_params.push(Box::new(limit));
        all_params.push(Box::new(offset));

        let mut stmt = self.conn.prepare(&list_sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(all_params.iter().map(|p| p.as_ref())), row_to_property)
            .map_err(|e| e.to_string())?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row.map_err(|e| e.to_string())?);
        }
        Ok((items, total))
    }

    fn get_property(&self, id: &str, user_id: &str) -> Result<Option<Property>, String> {
        let mut stmt = self
            .conn
            .prepare(&format!("SELECT {PROPERTY_COLS} FROM properties WHERE id = ?1 AND user_id = ?2"))
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![id, user_id]).map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            Ok(Some(row_to_property(row).map_err(|e| e.to_string())?))
        } else {
            Ok(None)
        }
    }

    fn create_property(&self, property: &Property) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO properties (id, user_id, name, city, district, location, property_type, \
                 area, price, price_per_meter, ad_number, ad_link, owner_name, owner_phone, \
                 owner_contact, owner_notes, marketing_platform, marketing_link, published_date, status, \
                 owner_client_id, source_type, purpose, usage_type, frontage, street_width, streets_count, \
                 latitude, longitude, description, listing_expiry_date, commission_agreement, \
                 fal_ad_license, internal_price, internal_notes) \
                 VALUES (?1, ?2, ?3, NULLIF(?4,''), NULLIF(?5,''), NULLIF(?6,''), NULLIF(?7,''), \
                 ?8, ?9, ?10, NULLIF(?11,''), NULLIF(?12,''), NULLIF(?13,''), NULLIF(?14,''), \
                 NULLIF(?15,''), NULLIF(?16,''), NULLIF(?17,''), NULLIF(?18,''), NULLIF(?19,''), ?20, \
                 NULLIF(?21,''), NULLIF(?22,''), NULLIF(?23,''), NULLIF(?24,''), NULLIF(?25,''), ?26, ?27, \
                 ?28, ?29, NULLIF(?30,''), NULLIF(?31,''), NULLIF(?32,''), NULLIF(?33,''), ?34, NULLIF(?35,''))",
                params![
                    property.id, property.user_id, property.name, property.city, property.district,
                    property.location, property.property_type, property.area, property.price,
                    property.price_per_meter, property.ad_number, property.ad_link, property.owner_name,
                    property.owner_phone, property.owner_contact, property.owner_notes,
                    property.marketing_platform, property.marketing_link, property.published_date,
                    property.status,
                    property.owner_client_id, property.source_type, property.purpose, property.usage_type,
                    property.frontage, property.street_width, property.streets_count,
                    property.latitude, property.longitude, property.description,
                    property.listing_expiry_date, property.commission_agreement, property.fal_ad_license,
                    property.internal_price, property.internal_notes
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn update_property(&self, property: &Property) -> Result<(), String> {
        // كتابة صريحة: الدمج مع القيم السابقة يحدث في طبقة الخدمة.
        let changed = self
            .conn
            .execute(
                "UPDATE properties SET name = ?1, city = NULLIF(?2,''), district = NULLIF(?3,''), \
                 location = NULLIF(?4,''), property_type = NULLIF(?5,''), area = ?6, price = ?7, \
                 price_per_meter = ?8, ad_number = NULLIF(?9,''), ad_link = NULLIF(?10,''), \
                 owner_name = NULLIF(?11,''), owner_phone = NULLIF(?12,''), owner_contact = NULLIF(?13,''), \
                 owner_notes = NULLIF(?14,''), marketing_platform = NULLIF(?15,''), \
                 marketing_link = NULLIF(?16,''), published_date = NULLIF(?17,''), status = ?18, \
                 owner_client_id = NULLIF(?19,''), source_type = NULLIF(?20,''), purpose = NULLIF(?21,''), \
                 usage_type = NULLIF(?22,''), frontage = NULLIF(?23,''), street_width = ?24, \
                 streets_count = ?25, latitude = ?26, longitude = ?27, description = NULLIF(?28,''), \
                 listing_expiry_date = NULLIF(?29,''), commission_agreement = NULLIF(?30,''), \
                 fal_ad_license = NULLIF(?31,''), internal_price = ?32, internal_notes = NULLIF(?33,''), \
                 updated_at = datetime('now') WHERE id = ?34 AND user_id = ?35",
                params![
                    property.name, property.city, property.district, property.location,
                    property.property_type, property.area, property.price, property.price_per_meter,
                    property.ad_number, property.ad_link, property.owner_name, property.owner_phone,
                    property.owner_contact, property.owner_notes, property.marketing_platform,
                    property.marketing_link, property.published_date, property.status,
                    property.owner_client_id, property.source_type, property.purpose,
                    property.usage_type, property.frontage, property.street_width,
                    property.streets_count, property.latitude, property.longitude,
                    property.description, property.listing_expiry_date, property.commission_agreement,
                    property.fal_ad_license, property.internal_price, property.internal_notes,
                    property.id, property.user_id
                ],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err("العرض غير موجود".to_string());
        }
        Ok(())
    }

    fn delete_property(&self, id: &str) -> Result<(), String> {
        self.conn
            .execute("DELETE FROM properties WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn update_property_status(&self, id: &str, status: &str) -> Result<(), String> {
        self.conn
            .execute("UPDATE properties SET status = ?1, updated_at = datetime('now') WHERE id = ?2", params![status, id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ===== Property Images =====
    fn list_property_images(&self, property_id: &str) -> Result<Vec<PropertyImage>, String> {
        let mut stmt = self
            .conn
            .prepare(&format!("SELECT {PROPERTY_IMAGE_COLS} FROM property_images WHERE property_id = ?1 ORDER BY created_at ASC"))
            .map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![property_id], row_to_property_image).map_err(|e| e.to_string())?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row.map_err(|e| e.to_string())?);
        }
        Ok(items)
    }

    fn get_property_image(&self, id: &str) -> Result<Option<PropertyImage>, String> {
        let mut stmt = self
            .conn
            .prepare(&format!("SELECT {PROPERTY_IMAGE_COLS} FROM property_images WHERE id = ?1"))
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![id]).map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            Ok(Some(row_to_property_image(row).map_err(|e| e.to_string())?))
        } else {
            Ok(None)
        }
    }

    fn count_property_images(&self, property_id: &str) -> Result<i64, String> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM property_images WHERE property_id = ?1",
                params![property_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())
    }

    fn count_images_for_ids(&self, ids: &[String]) -> Result<std::collections::HashMap<String, i64>, String> {
        let mut map = std::collections::HashMap::new();
        if ids.is_empty() {
            return Ok(map);
        }
        let placeholders = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT property_id, COUNT(*) FROM property_images WHERE property_id IN ({placeholders}) GROUP BY property_id"
        );
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let params_vec: Vec<&dyn rusqlite::ToSql> = ids.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
        let rows = stmt
            .query_map(params_vec.as_slice(), |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))
            .map_err(|e| e.to_string())?;
        for row in rows {
            let (pid, n) = row.map_err(|e| e.to_string())?;
            map.insert(pid, n);
        }
        Ok(map)
    }

    fn create_property_image(&self, image: &PropertyImage) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO property_images (id, property_id, image_url) VALUES (?1, ?2, ?3)",
                params![image.id, image.property_id, image.image_url],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn delete_property_image(&self, id: &str) -> Result<(), String> {
        self.conn
            .execute("DELETE FROM property_images WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn set_property_main_image(&self, property_id: &str, image_url: &str) -> Result<(), String> {
        self.conn
            .execute("UPDATE properties SET main_image = ?1 WHERE id = ?2", params![image_url, property_id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn clear_property_main_image(&self, property_id: &str) -> Result<(), String> {
        self.conn
            .execute("UPDATE properties SET main_image = NULL WHERE id = ?1", params![property_id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ===== Property Updates =====
    fn list_property_updates(&self, property_id: &str) -> Result<Vec<PropertyUpdate>, String> {
        let mut stmt = self
            .conn
            .prepare(&format!("SELECT {PROPERTY_UPDATE_COLS} FROM property_updates WHERE property_id = ?1 ORDER BY created_at DESC"))
            .map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![property_id], row_to_property_update).map_err(|e| e.to_string())?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row.map_err(|e| e.to_string())?);
        }
        Ok(items)
    }

    fn create_property_update(&self, update: &PropertyUpdate) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO property_updates (id, property_id, user_id, title, description) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![update.id, update.property_id, update.user_id, update.title, update.description],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ===== Clients =====
    fn list_clients(&self, user_id: &str, search: &str, role: &str, city: &str, status: &str, needs_followup: bool, limit: i64, offset: i64) -> Result<(Vec<Client>, i64), String> {
        let mut where_clause = String::from("WHERE user_id = ?1");
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(user_id.to_string())];
        let mut idx = 2;

        if !search.is_empty() {
            where_clause.push_str(&format!(
                " AND (name LIKE ?{i} OR phone LIKE ?{i} OR phone_secondary LIKE ?{i} OR whatsapp LIKE ?{i} OR email LIKE ?{i} OR company LIKE ?{i})",
                i = idx
            ));
            // نفس المعامل ?N يُعاد استخدامه في كل الحقول، لذلك يُربط مرة واحدة فقط.
            params_vec.push(Box::new(format!("%{}%", search)));
            idx += 1;
        }
        if !role.is_empty() {
            // الدور مصدره جدول client_roles، لذلك الفلترة عبر EXISTS وليس حقلاً نصياً واحداً.
            where_clause.push_str(&format!(
                " AND EXISTS (SELECT 1 FROM client_roles cr WHERE cr.client_id = clients.id AND cr.role = ?{})",
                idx
            ));
            params_vec.push(Box::new(role.to_string()));
            idx += 1;
        }
        if !city.is_empty() {
            where_clause.push_str(&format!(" AND city = ?{}", idx));
            params_vec.push(Box::new(city.to_string()));
            idx += 1;
        }
        if !status.is_empty() {
            where_clause.push_str(&format!(" AND status = ?{}", idx));
            params_vec.push(Box::new(status.to_string()));
            idx += 1;
        }
        if needs_followup {
            where_clause.push_str(" AND follow_up_date IS NOT NULL AND follow_up_date != '' AND follow_up_date <= date('now')");
        }

        let count_sql = format!("SELECT COUNT(*) FROM clients {}", where_clause);
        let total: i64 = self
            .conn
            .query_row(&count_sql, rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())), |r| r.get(0))
            .map_err(|e| e.to_string())?;

        let list_sql = format!("SELECT {} FROM clients {} ORDER BY created_at DESC LIMIT ?{} OFFSET ?{}", CLIENT_COLS, where_clause, idx, idx + 1);
        let mut all_params: Vec<Box<dyn rusqlite::ToSql>> = params_vec;
        all_params.push(Box::new(limit));
        all_params.push(Box::new(offset));

        let mut stmt = self.conn.prepare(&list_sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(all_params.iter().map(|p| p.as_ref())), row_to_client)
            .map_err(|e| e.to_string())?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row.map_err(|e| e.to_string())?);
        }
        Ok((items, total))
    }

    fn get_client(&self, id: &str, user_id: &str) -> Result<Option<Client>, String> {
        let mut stmt = self
            .conn
            .prepare(&format!("SELECT {CLIENT_COLS} FROM clients WHERE id = ?1 AND user_id = ?2"))
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![id, user_id]).map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            Ok(Some(row_to_client(row).map_err(|e| e.to_string())?))
        } else {
            Ok(None)
        }
    }

    fn create_client(&self, client: &Client) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO clients (id, user_id, name, phone, whatsapp, relationship_type, city, \
                 interests, required_property_type, budget, notes, follow_up_date, \
                 email, phone_secondary, company, source, status, category) \
                 VALUES (?1, ?2, ?3, NULLIF(?4,''), NULLIF(?5,''), NULLIF(?6,''), NULLIF(?7,''), \
                 NULLIF(?8,''), NULLIF(?9,''), NULLIF(?10,''), NULLIF(?11,''), NULLIF(?12,''), \
                 NULLIF(?13,''), NULLIF(?14,''), NULLIF(?15,''), NULLIF(?16,''), NULLIF(?17,''), NULLIF(?18,''))",
                params![
                    client.id, client.user_id, client.name, client.phone, client.whatsapp,
                    client.relationship_type, client.city, client.interests,
                    client.required_property_type, client.budget, client.notes, client.follow_up_date,
                    client.email, client.phone_secondary, client.company, client.source,
                    client.status, client.category
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn update_client(&self, client: &Client) -> Result<(), String> {
        // كتابة صريحة لكل حقل: الدمج مع القيم السابقة يحدث في طبقة الخدمة،
        // لذلك COALESCE هنا كان يمنع تفريغ أي حقل نهائياً.
        // NULLIF(?, '') يخزّن NULL حقيقياً بدل نص فارغ.
        let changed = self
            .conn
            .execute(
                "UPDATE clients SET name = ?1, phone = NULLIF(?2,''), whatsapp = NULLIF(?3,''), \
                 relationship_type = NULLIF(?4,''), city = NULLIF(?5,''), interests = NULLIF(?6,''), \
                 required_property_type = NULLIF(?7,''), budget = NULLIF(?8,''), notes = NULLIF(?9,''), \
                 follow_up_date = NULLIF(?10,''), last_contact = NULLIF(?11,''), \
                 email = NULLIF(?12,''), phone_secondary = NULLIF(?13,''), company = NULLIF(?14,''), \
                 source = NULLIF(?15,''), status = NULLIF(?16,''), category = NULLIF(?17,''), \
                 updated_at = datetime('now') WHERE id = ?18 AND user_id = ?19",
                params![
                    client.name, client.phone, client.whatsapp, client.relationship_type,
                    client.city, client.interests, client.required_property_type, client.budget,
                    client.notes, client.follow_up_date, client.last_contact,
                    client.email, client.phone_secondary, client.company, client.source,
                    client.status, client.category,
                    client.id, client.user_id
                ],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err("العميل غير موجود".to_string());
        }
        Ok(())
    }

    // ===== Client Roles =====
    fn list_client_roles(&self, client_id: &str) -> Result<Vec<String>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT role FROM client_roles WHERE client_id = ?1 ORDER BY created_at ASC, role ASC")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![client_id], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    fn set_client_roles(&self, client_id: &str, roles: &[String]) -> Result<(), String> {
        let apply = |conn: &rusqlite::Connection| -> Result<(), String> {
            conn.execute("DELETE FROM client_roles WHERE client_id = ?1", params![client_id])
                .map_err(|e| e.to_string())?;
            for role in roles {
                let role = role.trim();
                if role.is_empty() {
                    continue;
                }
                conn.execute(
                    "INSERT OR IGNORE INTO client_roles (id, client_id, role) VALUES (?1, ?2, ?3)",
                    params![uuid::Uuid::new_v4().to_string(), client_id, role],
                )
                .map_err(|e| e.to_string())?;
            }
            let primary = roles.iter().map(|r| r.trim()).find(|r| !r.is_empty());
            conn.execute(
                "UPDATE clients SET relationship_type = ?1 WHERE id = ?2",
                params![primary, client_id],
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        };

        // إن كنا أصلًا داخل معاملة خارجية (P11) لا نفتح BEGIN جديد.
        if !self.conn.is_autocommit() {
            return apply(&self.conn);
        }
        let tx = self.conn.unchecked_transaction().map_err(|e| e.to_string())?;
        apply(&tx)?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    fn list_roles_for_clients(&self, client_ids: &[String]) -> Result<Vec<(String, String)>, String> {
        if client_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = (1..=client_ids.len()).map(|i| format!("?{}", i)).collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT client_id, role FROM client_roles WHERE client_id IN ({}) ORDER BY created_at ASC, role ASC",
            placeholders
        );
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(client_ids.iter()), |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    fn list_client_options(&self, user_id: &str, search: &str, limit: i64) -> Result<Vec<serde_json::Value>, String> {
        let like = format!("%{}%", search);
        let mut stmt = self
            .conn
            .prepare(
                "SELECT c.id, c.name, c.phone, c.city, \
                 (SELECT GROUP_CONCAT(cr.role) FROM client_roles cr WHERE cr.client_id = c.id) AS roles \
                 FROM clients c \
                 WHERE c.user_id = ?1 AND (?2 = '' OR c.name LIKE ?3 OR c.phone LIKE ?3) \
                 ORDER BY c.name ASC LIMIT ?4",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![user_id, search, like, limit], |r| {
                Ok(serde_json::json!({
                    "id": r.get::<_, String>(0)?,
                    "name": r.get::<_, String>(1)?,
                    "phone": r.get::<_, Option<String>>(2)?,
                    "city": r.get::<_, Option<String>>(3)?,
                    "roles": r.get::<_, Option<String>>(4)?
                        .map(|s| s.split(',').map(|x| x.to_string()).collect::<Vec<_>>())
                        .unwrap_or_default(),
                }))
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    fn delete_client(&self, id: &str) -> Result<(), String> {
        self.conn
            .execute("DELETE FROM clients WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn update_client_last_contact(&self, id: &str) -> Result<(), String> {
        self.conn
            .execute("UPDATE clients SET last_contact = datetime('now') WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ===== Communications =====
    fn list_communications(&self, client_id: &str) -> Result<Vec<Communication>, String> {
        let mut stmt = self
            .conn
            .prepare(&format!("SELECT {COMMUNICATION_COLS} FROM communications WHERE client_id = ?1 ORDER BY created_at DESC"))
            .map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![client_id], row_to_communication).map_err(|e| e.to_string())?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row.map_err(|e| e.to_string())?);
        }
        Ok(items)
    }

    fn create_communication(&self, comm: &Communication) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO communications (id, user_id, client_id, type, subject, notes) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![comm.id, comm.user_id, comm.client_id, comm.r#type, comm.subject, comm.notes],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ===== Client-Property Links =====
    fn get_client_property_link(&self, client_id: &str, property_id: &str) -> Result<Option<ClientPropertyLink>, String> {
        let mut stmt = self
            .conn
            .prepare(&format!("SELECT {CLIENT_PROPERTY_LINK_COLS} FROM client_property_links WHERE client_id = ?1 AND property_id = ?2"))
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![client_id, property_id]).map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            Ok(Some(row_to_client_property_link(row).map_err(|e| e.to_string())?))
        } else {
            Ok(None)
        }
    }

    fn create_client_property_link(&self, link: &ClientPropertyLink) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO client_property_links (id, client_id, property_id, link_type) VALUES (?1, ?2, ?3, ?4)",
                params![link.id, link.client_id, link.property_id, link.link_type],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn update_client_property_link_type(&self, id: &str, link_type: &str) -> Result<(), String> {
        self.conn
            .execute("UPDATE client_property_links SET link_type = ?1 WHERE id = ?2", params![link_type, id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn delete_client_property_link(&self, client_id: &str, property_id: &str, user_id: &str) -> Result<usize, String> {
        // الحذف يشترط ملكية العميل والعقار لنفس المستخدم — لا يكفي تطابق المعرّفات وحدها
        let n = self
            .conn
            .execute(
                "DELETE FROM client_property_links
                 WHERE client_id = ?1
                   AND property_id = ?2
                   AND EXISTS (
                     SELECT 1 FROM clients c
                     WHERE c.id = client_property_links.client_id AND c.user_id = ?3
                   )
                   AND EXISTS (
                     SELECT 1 FROM properties p
                     WHERE p.id = client_property_links.property_id AND p.user_id = ?3
                   )",
                params![client_id, property_id, user_id],
            )
            .map_err(|e| e.to_string())?;
        Ok(n)
    }

    fn list_linked_properties_for_client(&self, client_id: &str) -> Result<Vec<serde_json::Value>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT cpl.id, cpl.client_id, cpl.property_id, cpl.link_type, cpl.created_at,
                        p.name as property_name, p.price, p.city, p.district, p.status, p.main_image, p.country, p.currency
                 FROM client_property_links cpl
                 JOIN properties p ON p.id = cpl.property_id
                 WHERE cpl.client_id = ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![client_id], |row| {
            Ok(serde_json::json!({
                "country":row.get::<_,String>(11)?,"currency":row.get::<_,String>(12)?,
                "id": row.get::<_, String>(0)?,
                "client_id": row.get::<_, String>(1)?,
                "property_id": row.get::<_, String>(2)?,
                "link_type": row.get::<_, Option<String>>(3)?,
                "created_at": row.get::<_, String>(4)?,
                "property_name": row.get::<_, String>(5)?,
                "price": row.get::<_, Option<f64>>(6)?,
                "city": row.get::<_, Option<String>>(7)?,
                "district": row.get::<_, Option<String>>(8)?,
                "status": row.get::<_, String>(9)?,
                "main_image": row.get::<_, Option<String>>(10)?,
            }))
        }).map_err(|e| e.to_string())?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row.map_err(|e| e.to_string())?);
        }
        Ok(items)
    }

    fn list_linked_clients_for_property(&self, property_id: &str) -> Result<Vec<serde_json::Value>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT cpl.id, cpl.client_id, cpl.property_id, cpl.link_type, cpl.created_at,
                        c.name as client_name, c.phone as client_phone
                 FROM client_property_links cpl
                 JOIN clients c ON c.id = cpl.client_id
                 WHERE cpl.property_id = ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![property_id], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "client_id": row.get::<_, String>(1)?,
                "property_id": row.get::<_, String>(2)?,
                "link_type": row.get::<_, Option<String>>(3)?,
                "created_at": row.get::<_, String>(4)?,
                "client_name": row.get::<_, String>(5)?,
                "client_phone": row.get::<_, Option<String>>(6)?,
            }))
        }).map_err(|e| e.to_string())?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row.map_err(|e| e.to_string())?);
        }
        Ok(items)
    }

    // ===== Market Reference Data =====
    fn list_market_data(&self, user_id: &str, city: &str, district: &str, property_type: &str, limit: i64, offset: i64) -> Result<(Vec<MarketReferenceData>, i64), String> {
        let mut where_clause = String::from("WHERE user_id = ?1");
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(user_id.to_string())];
        let mut idx = 2;

        if !city.is_empty() {
            where_clause.push_str(&format!(" AND city = ?{}", idx));
            params_vec.push(Box::new(city.to_string()));
            idx += 1;
        }
        if !district.is_empty() {
            where_clause.push_str(&format!(" AND district = ?{}", idx));
            params_vec.push(Box::new(district.to_string()));
            idx += 1;
        }
        if !property_type.is_empty() {
            where_clause.push_str(&format!(" AND property_type = ?{}", idx));
            params_vec.push(Box::new(property_type.to_string()));
            idx += 1;
        }

        let count_sql = format!("SELECT COUNT(*) FROM market_reference_data {}", where_clause);
        let total: i64 = self
            .conn
            .query_row(&count_sql, rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())), |r| r.get(0))
            .map_err(|e| e.to_string())?;

        let list_sql = format!("SELECT {} FROM market_reference_data {} ORDER BY entry_date DESC LIMIT ?{} OFFSET ?{}", MARKET_COLS, where_clause, idx, idx + 1);
        let mut all_params: Vec<Box<dyn rusqlite::ToSql>> = params_vec;
        all_params.push(Box::new(limit));
        all_params.push(Box::new(offset));

        let mut stmt = self.conn.prepare(&list_sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(all_params.iter().map(|p| p.as_ref())), row_to_market_data)
            .map_err(|e| e.to_string())?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row.map_err(|e| e.to_string())?);
        }
        Ok((items, total))
    }

    fn get_market_stats(&self, user_id: &str, city: &str, district: &str, property_type: &str) -> Result<serde_json::Value, String> {
        let context = crate::country::RecordContext::from_settings(&self.get_settings(user_id)?)?;
        let mut where_clause = String::from("WHERE user_id = ?1 AND country = COALESCE((SELECT country FROM settings WHERE user_id=?1),'SA')");
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(user_id.to_string())];
        let mut idx = 2;

        if !city.is_empty() {
            where_clause.push_str(&format!(" AND city = ?{}", idx));
            params_vec.push(Box::new(city.to_string()));
            idx += 1;
        }
        if !district.is_empty() {
            where_clause.push_str(&format!(" AND district = ?{}", idx));
            params_vec.push(Box::new(district.to_string()));
            idx += 1;
        }
        if !property_type.is_empty() {
            where_clause.push_str(&format!(" AND property_type = ?{}", idx));
            params_vec.push(Box::new(property_type.to_string()));
            idx += 1;
        }

        let sql = format!(
            "SELECT COUNT(*) as count, \
             AVG(CASE WHEN price_per_meter_sale IS NOT NULL AND price_per_meter_sale > 0 THEN price_per_meter_sale END) as avg_price_per_meter, \
             MIN(CASE WHEN price_per_meter_sale IS NOT NULL AND price_per_meter_sale > 0 THEN price_per_meter_sale END) as min_price_per_meter, \
             MAX(CASE WHEN price_per_meter_sale IS NOT NULL AND price_per_meter_sale > 0 THEN price_per_meter_sale END) as max_price_per_meter, \
             AVG(CASE WHEN rent_per_meter IS NOT NULL AND rent_per_meter > 0 THEN rent_per_meter END) as avg_rent_per_meter, \
             SUM(COALESCE(requests_count, 0)) as total_requests, \
             SUM(COALESCE(offers_count, 0)) as total_offers, \
             MAX(CASE WHEN entry_date IS NOT NULL AND entry_date != '' AND entry_date NOT LIKE '1900%' AND entry_date NOT LIKE '1899%' THEN entry_date END) as last_updated \
             FROM market_reference_data {}",
            where_clause
        );

        let row = self
            .conn
            .query_row(&sql, rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())), |r| {
                Ok(serde_json::json!({
                    "country":context.country,"currency":context.currency,
                    "count": r.get::<_, i64>(0)?,
                    "avg_price_per_meter": r.get::<_, Option<f64>>(1)?,
                    "min_price_per_meter": r.get::<_, Option<f64>>(2)?,
                    "max_price_per_meter": r.get::<_, Option<f64>>(3)?,
                    "avg_rent_per_meter": r.get::<_, Option<f64>>(4)?,
                    "total_requests": r.get::<_, Option<i64>>(5)?,
                    "total_offers": r.get::<_, Option<i64>>(6)?,
                    "last_updated": r.get::<_, Option<String>>(7)?,
                }))
            })
            .map_err(|e| e.to_string())?;
        Ok(row)
    }

    fn get_market_trends(&self, user_id: &str, city: &str, district: &str, property_type: &str) -> Result<Vec<serde_json::Value>, String> {
        let mut where_clause = String::from("WHERE user_id = ?1 AND country = COALESCE((SELECT country FROM settings WHERE user_id=?1),'SA')");
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(user_id.to_string())];
        let mut idx = 2;

        if !city.is_empty() {
            where_clause.push_str(&format!(" AND city = ?{}", idx));
            params_vec.push(Box::new(city.to_string()));
            idx += 1;
        }
        if !district.is_empty() {
            where_clause.push_str(&format!(" AND district = ?{}", idx));
            params_vec.push(Box::new(district.to_string()));
            idx += 1;
        }
        if !property_type.is_empty() {
            where_clause.push_str(&format!(" AND property_type = ?{}", idx));
            params_vec.push(Box::new(property_type.to_string()));
            idx += 1;
        }

        // تفصيل زمني تلقائي: أيام إذا كانت الفترة ≤ 90 يومًا مختلفًا، وإلا شهور (substr YYYY-MM)
        // — دائماً ≤ 90 مقطعاً مهما طال الفترة، ولا يُرجَع record-per-point إلى React.
        let distinct_days_sql = format!(
            "SELECT COUNT(DISTINCT entry_date) FROM market_reference_data {} AND entry_date IS NOT NULL AND entry_date != ''",
            where_clause
        );
        let distinct_days: i64 = self
            .conn
            .query_row(
                &distinct_days_sql,
                rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())),
                |r| r.get(0),
            )
            .unwrap_or(0);
        let bucket_expr = if distinct_days > 90 {
            "substr(entry_date, 1, 7)"
        } else {
            "entry_date"
        };
        let sql = format!(
            "SELECT {bucket_expr} AS entry_date, AVG(price_per_meter_sale) as avg_price_per_meter, COUNT(*) as count \
             FROM market_reference_data {} AND entry_date IS NOT NULL AND entry_date != '' \
             GROUP BY {bucket_expr} ORDER BY entry_date DESC LIMIT 90",
            where_clause
        );

        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())), |r| {
                Ok(serde_json::json!({
                    "entry_date": r.get::<_, Option<String>>(0)?,
                    "avg_price_per_meter": r.get::<_, Option<f64>>(1)?,
                    "count": r.get::<_, i64>(2)?,
                }))
            })
            .map_err(|e| e.to_string())?;
        let mut items = Vec::new();
        for row in rows {
            items.insert(0, row.map_err(|e| e.to_string())?);
        }
        Ok(items)
    }

    fn market_distinct_cities(&self, user_id: &str) -> Result<Vec<String>, String> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT city FROM market_reference_data WHERE user_id = ?1 AND city IS NOT NULL AND city != '' ORDER BY city ASC LIMIT 500",
        ).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![user_id], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    fn market_districts_of_city(&self, user_id: &str, city: &str) -> Result<Vec<String>, String> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT district FROM market_reference_data WHERE user_id = ?1 AND city = ?2 AND district IS NOT NULL AND district != '' ORDER BY district ASC LIMIT 500",
        ).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![user_id, city], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    fn market_geo_summary(&self, user_id: &str, city: &str, district: &str, mode: &str, limit: i64) -> Result<Vec<serde_json::Value>, String> {
        // بطاقات مجمعة لرأس المؤشر — التجميع والتقييم داخل SQLite فقط.
        // مهما كبر الجدول: عدد المجموعات المعروض محدود (limit) ويُختار بأحدث بيانات.
        let safe_limit = limit.clamp(1, 500);
        let group_district = mode == "district";
        // تعبير الفئة وتعبير التجميع:
        //   mode city     → فئة = المدينة (تجميع بالمدينة)
        //   mode district → فئة = الحي داخل المدينة المختارة (تجميع بالمدينة والحي)
        let label_expr = if group_district {
            "IFNULL(NULLIF(TRIM(district),''),'غير محدد')"
        } else {
            "IFNULL(NULLIF(TRIM(city),''),'غير محدد')"
        };
        let group_by_expr = if group_district { "city, district" } else { "city" };

        let mut main_sql = format!(
            "SELECT {label_expr} AS label, COUNT(*) AS cnt, \
             AVG(CASE WHEN price_per_meter_sale IS NOT NULL AND price_per_meter_sale > 0 THEN price_per_meter_sale END) AS avg_ppm, \
             MIN(CASE WHEN price_per_meter_sale IS NOT NULL AND price_per_meter_sale > 0 THEN price_per_meter_sale END) AS min_ppm, \
             MAX(CASE WHEN price_per_meter_sale IS NOT NULL AND price_per_meter_sale > 0 THEN price_per_meter_sale END) AS max_ppm, \
             AVG(CASE WHEN rent_per_meter IS NOT NULL AND rent_per_meter > 0 THEN rent_per_meter END) AS avg_rpm, \
             SUM(COALESCE(requests_count, 0)) AS total_requests, \
             SUM(COALESCE(offers_count, 0)) AS total_offers, \
             MAX(CASE WHEN entry_date IS NOT NULL AND entry_date != '' THEN entry_date END) AS last_date \
             FROM market_reference_data \
             WHERE user_id = ?1 \
             AND (?2 = '' OR city = ?2) \
             AND (?3 = '' OR district = ?3) \
             AND country = COALESCE((SELECT country FROM settings WHERE user_id = ?1), 'SA')"
        );
        main_sql.push_str(" AND city IS NOT NULL AND NULLIF(TRIM(city),'') IS NOT NULL");
        main_sql.push_str(&format!(
            " GROUP BY {group_by_expr} ORDER BY last_date DESC, cnt DESC LIMIT {safe_limit}"
        ));

        let mut stmt = self.conn.prepare(&main_sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![user_id, city, district], |r| {
                Ok(serde_json::json!({
                    "label": r.get::<_, String>(0)?,
                    "count": r.get::<_, i64>(1)?,
                    "avg_price_per_meter": r.get::<_, Option<f64>>(2)?,
                    "min_price_per_meter": r.get::<_, Option<f64>>(3)?,
                    "max_price_per_meter": r.get::<_, Option<f64>>(4)?,
                    "avg_rent_per_meter": r.get::<_, Option<f64>>(5)?,
                    "total_requests": r.get::<_, Option<i64>>(6)?,
                    "total_offers": r.get::<_, Option<i64>>(7)?,
                    "last_date": r.get::<_, Option<String>>(8)?,
                }))
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| e.to_string())?);
        }

        // نسبة التغير لكل فئة: متوسط سعر المتر بأحدث تاريخ مقابل أقدم تاريخ (نفس قواعد العرض)
        let mut filter_sql = String::from(
            "WHERE user_id = ?1 AND country = COALESCE((SELECT country FROM settings WHERE user_id = ?1), 'SA')"
        );
        if !city.trim().is_empty() {
            filter_sql.push_str(" AND city = ?2");
        }
        if !district.trim().is_empty() {
            filter_sql.push_str(" AND district = ?3");
        }
        let day_sql = format!(
            "SELECT {label_expr} AS label, entry_date AS d, AVG(price_per_meter_sale) AS v \
             FROM market_reference_data {filter_sql} \
             AND price_per_meter_sale IS NOT NULL AND price_per_meter_sale > 0 \
             AND entry_date IS NOT NULL AND entry_date != '' \
             AND {label_expr} IN (SELECT label FROM ({main_sql})) \
             GROUP BY {group_by_expr}, entry_date ORDER BY entry_date ASC"
        );
        if let Ok(mut day_stmt) = self.conn.prepare(&day_sql) {
            let params: Vec<Box<dyn rusqlite::ToSql>> = {
                let mut v: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(user_id.to_string())];
                if !city.trim().is_empty() {
                    v.push(Box::new(city.to_string()));
                }
                if !district.trim().is_empty() {
                    v.push(Box::new(district.to_string()));
                }
                v
            };
            let day_rows = day_stmt.query_map(
                rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
                |r| {
                    let label: String = r.get(0)?;
                    let d: String = r.get(1)?;
                    let v: Option<f64> = r.get(2)?;
                    Ok((label, d, v))
                },
            );
            if let Ok(day_rows) = day_rows {
                // اجمع سلسلة (فئة → (تاريخ → متوسطات)) ثم احسب نسبة التغير لكل فئة
                let mut series: std::collections::HashMap<String, std::collections::BTreeMap<String, Vec<f64>>> = std::collections::HashMap::new();
                for row in day_rows.flatten() {
                    let (label, d, v) = row;
                    if let Some(v) = v {
                        series.entry(label).or_default().entry(d).or_default().push(v);
                    }
                }
                for item in out.iter_mut() {
                    let obj = item.as_object_mut().expect("geo row object");
                    let label = obj.get("label").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let change = series.get(&label).and_then(|days| {
                        if days.len() < 2 {
                            return None;
                        }
                        let mut dates: Vec<(&String, &Vec<f64>)> = days.iter().collect();
                        dates.sort_by(|a, b| a.0.cmp(b.0));
                        let avg = |xs: &Vec<f64>| xs.iter().sum::<f64>() / xs.len() as f64;
                        let first = avg(dates.first()?.1);
                        let last = avg(dates.last()?.1);
                        if first == 0.0 {
                            return None;
                        }
                        Some(((last - first) / first) * 100.0)
                    });
                    obj.insert("change_pct".to_string(), match change {
                        Some(v) => serde_json::json!(v),
                        None => serde_json::Value::Null,
                    });
                }
            }
        }

        Ok(out)
    }

    fn get_market_data(&self, id: &str, user_id: &str) -> Result<Option<MarketReferenceData>, String> {
        let mut stmt = self
            .conn
            .prepare(&format!(
                "SELECT {} FROM market_reference_data WHERE id = ?1 AND user_id = ?2",
                MARKET_COLS
            ))
            .map_err(|e| e.to_string())?;
        let mut rows = stmt
            .query(params![id, user_id])
            .map_err(|e| e.to_string())?;
        match rows.next().map_err(|e| e.to_string())? {
            Some(row) => Ok(Some(row_to_market_data(row).map_err(|e| e.to_string())?)),
            None => Ok(None),
        }
    }

    fn create_market_data(&self, data: &MarketReferenceData) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO market_reference_data (id, user_id, city, district, property_type, usage_type, area, sale_price, price_per_meter_sale, rent_value, rent_per_meter, requests_count, offers_count, entry_date, data_source, notes) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                params![data.id, data.user_id, data.city, data.district, data.property_type, data.usage_type, data.area, data.sale_price, data.price_per_meter_sale, data.rent_value, data.rent_per_meter, data.requests_count, data.offers_count, data.entry_date, data.data_source, data.notes],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn create_market_data_bulk(&self, user_id: &str, items: &[serde_json::Value]) -> Result<(i64, Vec<String>), String> {
        let mut inserted: i64 = 0;
        let mut errors: Vec<String> = Vec::new();

        for (i, item) in items.iter().enumerate() {
            let row_num = i + 2;
            let city = item.get("city").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let district = item.get("district").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let property_type = item.get("property_type").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let sale_price = item.get("sale_price").and_then(|v| v.as_f64());

            if city.is_empty() && district.is_empty() && property_type.is_empty() && sale_price.is_none() {
                errors.push(format!("الصف {}: صف فارغ", row_num));
                continue;
            }

            let area = item.get("area").and_then(|v| v.as_f64());
            let rent_value = item.get("rent_value").and_then(|v| v.as_f64());
            let price_per_meter_sale = match (area, sale_price) {
                (Some(a), Some(s)) if a > 0.0 => Some(s / a),
                _ => None,
            };
            let rent_per_meter = match (area, rent_value) {
                (Some(a), Some(r)) if a > 0.0 => Some(r / a),
                _ => None,
            };

            let id = uuid::Uuid::new_v4().to_string();
            let usage_type = item.get("usage_type").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let requests_count = item.get("requests_count").and_then(|v| v.as_i64()).unwrap_or(0);
            let offers_count = item.get("offers_count").and_then(|v| v.as_i64()).unwrap_or(0);
            let entry_date = item.get("entry_date").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let data_source = item.get("data_source").and_then(|v| v.as_str()).unwrap_or("استيراد Excel").to_string();
            let notes = item.get("notes").and_then(|v| v.as_str()).unwrap_or("").to_string();

            let result = self.conn.execute(
                "INSERT INTO market_reference_data (id, user_id, city, district, property_type, usage_type, area, sale_price, price_per_meter_sale, rent_value, rent_per_meter, requests_count, offers_count, entry_date, data_source, notes) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                params![id, user_id, if city.is_empty() { None } else { Some(&city) }, if district.is_empty() { None } else { Some(&district) }, if property_type.is_empty() { None } else { Some(&property_type) }, if usage_type.is_empty() { None } else { Some(&usage_type) }, area, sale_price, price_per_meter_sale, rent_value, rent_per_meter, requests_count, offers_count, if entry_date.is_empty() { None } else { Some(&entry_date) }, if data_source.is_empty() { None } else { Some(&data_source) }, if notes.is_empty() { None } else { Some(&notes) }],
            );

            match result {
                Ok(_) => inserted += 1,
                Err(_) => errors.push(format!("الصف {}: خطأ في الحفظ", row_num)),
            }
        }

        Ok((inserted, errors))
    }

    fn update_market_data(&self, data: &MarketReferenceData) -> Result<(), String> {
        let n = self
            .conn
            .execute(
                "UPDATE market_reference_data SET city = ?1, district = ?2, property_type = ?3, usage_type = ?4, area = ?5, sale_price = ?6, price_per_meter_sale = ?7, rent_value = ?8, rent_per_meter = ?9, requests_count = ?10, offers_count = ?11, entry_date = ?12, data_source = ?13, notes = ?14 WHERE id = ?15 AND user_id = ?16",
                params![
                    data.city,
                    data.district,
                    data.property_type,
                    data.usage_type,
                    data.area,
                    data.sale_price,
                    data.price_per_meter_sale,
                    data.rent_value,
                    data.rent_per_meter,
                    data.requests_count,
                    data.offers_count,
                    data.entry_date,
                    data.data_source,
                    data.notes,
                    data.id,
                    data.user_id
                ],
            )
            .map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("السجل غير موجود".to_string());
        }
        Ok(())
    }

    fn delete_market_data(&self, id: &str, user_id: &str) -> Result<(), String> {
        let n = self
            .conn
            .execute(
                "DELETE FROM market_reference_data WHERE id = ?1 AND user_id = ?2",
                params![id, user_id],
            )
            .map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("السجل غير موجود".to_string());
        }
        Ok(())
    }

    fn delete_many_market_data(&self, ids: &[String], user_id: &str) -> Result<i64, String> {
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        let mut deleted: i64 = 0;
        {
            let mut stmt = tx
                .prepare("DELETE FROM market_reference_data WHERE id = ?1 AND user_id = ?2")
                .map_err(|e| e.to_string())?;
            for id in ids {
                deleted += stmt
                    .execute(params![id, user_id])
                    .map_err(|e| e.to_string())? as i64;
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(deleted)
    }

    fn delete_market_data_filtered(&self, user_id: &str, city: &str, district: &str, property_type: &str) -> Result<i64, String> {
        // حذف جماعي بكل السجلات المطابقة للفلاتر الحالية — داخل SQLite دون تحميل IDs إلى الواجهة.
        // نفس بناء WHERE المستخدم في list_market_data لضمان تطابق نطاق الحذف مع نتائج الفلتر.
        let mut where_clause = String::from("WHERE user_id = ?1");
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(user_id.to_string())];
        let mut idx = 2;

        if !city.is_empty() {
            where_clause.push_str(&format!(" AND city = ?{}", idx));
            params_vec.push(Box::new(city.to_string()));
            idx += 1;
        }
        if !district.is_empty() {
            where_clause.push_str(&format!(" AND district = ?{}", idx));
            params_vec.push(Box::new(district.to_string()));
            idx += 1;
        }
        if !property_type.is_empty() {
            where_clause.push_str(&format!(" AND property_type = ?{}", idx));
            params_vec.push(Box::new(property_type.to_string()));
            idx += 1;
        }

        let sql = format!("DELETE FROM market_reference_data {}", where_clause);
        let n = self
            .conn
            .execute(&sql, rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())))
            .map_err(|e| e.to_string())?;
        Ok(n as i64)
    }

    fn purge_audit_market_data(&self, user_id: &str) -> Result<i64, String> {
        let n = self
            .conn
            .execute(
                "DELETE FROM market_reference_data WHERE user_id = ?1 AND (
                    IFNULL(city, '') LIKE '%AUDIT%'
                    OR IFNULL(district, '') LIKE '%AUDIT%'
                    OR IFNULL(city, '') LIKE '%تدقيق%'
                    OR IFNULL(district, '') LIKE '%تدقيق%'
                    OR IFNULL(notes, '') LIKE '%AUDIT%'
                )",
                params![user_id],
            )
            .map_err(|e| e.to_string())?;
        Ok(n as i64)
    }

    // ===== Marketing Campaigns =====
    fn list_campaigns(
        &self,
        user_id: &str,
        platform: &str,
        property_id: &str,
        period_from: &str,
        period_to: &str,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<MarketingCampaign>, i64), String> {
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(user_id.to_string())];
        let (extra, extra_params, next_idx) =
            campaign_filter_clause(platform, property_id, period_from, period_to, 2);
        params_vec.extend(extra_params);
        let where_clause = format!("WHERE c.user_id = ?1{extra}");

        let count_sql = format!("SELECT COUNT(*) FROM marketing_campaigns c {where_clause}");
        let total: i64 = self
            .conn
            .query_row(
                &count_sql,
                rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())),
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;

        let list_sql = format!(
            "SELECT {}, p.name as property_name \
             FROM marketing_campaigns c \
             LEFT JOIN properties p ON p.id = c.property_id \
             {where_clause} \
             ORDER BY c.updated_at DESC LIMIT ?{next_idx} OFFSET ?{}",
            campaign_cols_prefixed(),
            next_idx + 1
        );
        let mut all_params = params_vec;
        all_params.push(Box::new(limit));
        all_params.push(Box::new(offset));

        let mut stmt = self.conn.prepare(&list_sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(
                rusqlite::params_from_iter(all_params.iter().map(|p| p.as_ref())),
                row_to_campaign,
            )
            .map_err(|e| e.to_string())?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row.map_err(|e| e.to_string())?);
        }
        Ok((items, total))
    }

    fn get_campaign(&self, id: &str, user_id: &str) -> Result<Option<MarketingCampaign>, String> {
        let sql = format!(
            "SELECT {}, p.name as property_name \
             FROM marketing_campaigns c \
             LEFT JOIN properties p ON p.id = c.property_id \
             WHERE c.id = ?1 AND c.user_id = ?2",
            campaign_cols_prefixed()
        );
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![id, user_id]).map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            Ok(Some(row_to_campaign(row).map_err(|e| e.to_string())?))
        } else {
            Ok(None)
        }
    }

    fn create_campaign(&self, campaign: &MarketingCampaign) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO marketing_campaigns (
                    id, user_id, name, property_id, platform, ad_link, start_date, end_date,
                    views, inquiries, viewings, deals, cost, notes, created_at, updated_at,
                    original_ad_url, tracking_code, tracking_url, tracked_clicks, unique_clicks,
                    last_click_at, tracking_enabled
                 ) VALUES (
                    ?1, ?2, ?3, NULLIF(?4,''), ?5, NULLIF(?6,''), NULLIF(?7,''), NULLIF(?8,''),
                    ?9, ?10, ?11, ?12, ?13, NULLIF(?14,''), ?15, ?16,
                    NULLIF(?17,''), NULLIF(?18,''), NULLIF(?19,''), ?20, ?21, NULLIF(?22,''), ?23
                 )",
                params![
                    campaign.id,
                    campaign.user_id,
                    campaign.name,
                    campaign.property_id.clone().unwrap_or_default(),
                    campaign.platform,
                    campaign.ad_link.clone().unwrap_or_default(),
                    campaign.start_date.clone().unwrap_or_default(),
                    campaign.end_date.clone().unwrap_or_default(),
                    campaign.views,
                    campaign.inquiries,
                    campaign.viewings,
                    campaign.deals,
                    campaign.cost,
                    campaign.notes.clone().unwrap_or_default(),
                    campaign.created_at,
                    campaign.updated_at,
                    campaign.original_ad_url.clone().unwrap_or_default(),
                    campaign.tracking_code.clone().unwrap_or_default(),
                    campaign.tracking_url.clone().unwrap_or_default(),
                    campaign.tracked_clicks,
                    campaign.unique_clicks,
                    campaign.last_click_at.clone().unwrap_or_default(),
                    if campaign.tracking_enabled { 1 } else { 0 },
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn update_campaign(&self, campaign: &MarketingCampaign) -> Result<(), String> {
        let changed = self
            .conn
            .execute(
                "UPDATE marketing_campaigns SET
                    name = ?1,
                    property_id = NULLIF(?2,''),
                    platform = ?3,
                    ad_link = NULLIF(?4,''),
                    start_date = NULLIF(?5,''),
                    end_date = NULLIF(?6,''),
                    views = ?7,
                    inquiries = ?8,
                    viewings = ?9,
                    deals = ?10,
                    cost = ?11,
                    notes = NULLIF(?12,''),
                    updated_at = ?13,
                    original_ad_url = NULLIF(?14,''),
                    tracking_code = NULLIF(?15,''),
                    tracking_url = NULLIF(?16,''),
                    tracked_clicks = ?17,
                    unique_clicks = ?18,
                    last_click_at = NULLIF(?19,''),
                    tracking_enabled = ?20
                 WHERE id = ?21 AND user_id = ?22",
                params![
                    campaign.name,
                    campaign.property_id.clone().unwrap_or_default(),
                    campaign.platform,
                    campaign.ad_link.clone().unwrap_or_default(),
                    campaign.start_date.clone().unwrap_or_default(),
                    campaign.end_date.clone().unwrap_or_default(),
                    campaign.views,
                    campaign.inquiries,
                    campaign.viewings,
                    campaign.deals,
                    campaign.cost,
                    campaign.notes.clone().unwrap_or_default(),
                    campaign.updated_at,
                    campaign.original_ad_url.clone().unwrap_or_default(),
                    campaign.tracking_code.clone().unwrap_or_default(),
                    campaign.tracking_url.clone().unwrap_or_default(),
                    campaign.tracked_clicks,
                    campaign.unique_clicks,
                    campaign.last_click_at.clone().unwrap_or_default(),
                    if campaign.tracking_enabled { 1 } else { 0 },
                    campaign.id,
                    campaign.user_id,
                ],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err("الحملة غير موجودة".to_string());
        }
        Ok(())
    }

    fn delete_campaign(&self, id: &str, user_id: &str) -> Result<(), String> {
        let changed = self
            .conn
            .execute(
                "DELETE FROM marketing_campaigns WHERE id = ?1 AND user_id = ?2",
                params![id, user_id],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err("الحملة غير موجودة".to_string());
        }
        Ok(())
    }

    fn get_campaign_totals(
        &self,
        user_id: &str,
        platform: &str,
        property_id: &str,
        period_from: &str,
        period_to: &str,
    ) -> Result<serde_json::Value, String> {
        let context = crate::country::RecordContext::from_settings(&self.get_settings(user_id)?)?;
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(user_id.to_string())];
        let (extra, extra_params, _) =
            campaign_filter_clause(platform, property_id, period_from, period_to, 2);
        params_vec.extend(extra_params);
        let sql = format!(
            "SELECT COUNT(*), COALESCE(SUM(c.views),0), COALESCE(SUM(c.inquiries),0), \
                    COALESCE(SUM(c.viewings),0), COALESCE(SUM(c.deals),0), COALESCE(SUM(c.cost),0), \
                    MAX(c.updated_at)
             FROM marketing_campaigns c
             WHERE c.user_id = ?1 AND c.country = COALESCE((SELECT country FROM settings WHERE user_id=?1),'SA'){extra}"
        );
        self.conn
            .query_row(
                &sql,
                rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())),
                |r| {
                    Ok(serde_json::json!({
                        "country":context.country,"currency":context.currency,
                    "count": r.get::<_, i64>(0)?,
                        "views": r.get::<_, i64>(1)?,
                        "inquiries": r.get::<_, i64>(2)?,
                        "viewings": r.get::<_, i64>(3)?,
                        "deals": r.get::<_, i64>(4)?,
                        "cost": r.get::<_, f64>(5)?,
                        "last_updated": r.get::<_, Option<String>>(6)?,
                    }))
                },
            )
            .map_err(|e| e.to_string())
    }

    fn list_appointments(
        &self,
        user_id: &str,
        from: &str,
        to: &str,
    ) -> Result<Vec<OfficeAppointment>, String> {
        let mut sql = format!(
            "SELECT {APPOINTMENT_COLS} FROM office_appointments WHERE user_id = ?1"
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(user_id.to_string())];
        let mut idx = 2;
        if !from.is_empty() {
            sql.push_str(&format!(" AND occurs_on >= ?{idx}"));
            params_vec.push(Box::new(from.to_string()));
            idx += 1;
        }
        if !to.is_empty() {
            sql.push_str(&format!(" AND occurs_on <= ?{idx}"));
            params_vec.push(Box::new(to.to_string()));
        }
        sql.push_str(" ORDER BY occurs_on ASC, COALESCE(starts_at, '99:99') ASC, created_at ASC");

        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(
                rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())),
                row_to_appointment,
            )
            .map_err(|e| e.to_string())?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row.map_err(|e| e.to_string())?);
        }
        Ok(items)
    }

    fn get_appointment(&self, id: &str, user_id: &str) -> Result<Option<OfficeAppointment>, String> {
        let mut stmt = self
            .conn
            .prepare(&format!(
                "SELECT {APPOINTMENT_COLS} FROM office_appointments WHERE id = ?1 AND user_id = ?2"
            ))
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![id, user_id]).map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            Ok(Some(row_to_appointment(row).map_err(|e| e.to_string())?))
        } else {
            Ok(None)
        }
    }

    fn create_appointment(&self, appointment: &OfficeAppointment) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO office_appointments (
                    id, user_id, title, occurs_on, starts_at, notes, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, NULLIF(?5,''), NULLIF(?6,''), ?7, ?8)",
                params![
                    appointment.id,
                    appointment.user_id,
                    appointment.title,
                    appointment.occurs_on,
                    appointment.starts_at.clone().unwrap_or_default(),
                    appointment.notes.clone().unwrap_or_default(),
                    appointment.created_at,
                    appointment.updated_at,
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn update_appointment(&self, appointment: &OfficeAppointment) -> Result<(), String> {
        let changed = self
            .conn
            .execute(
                "UPDATE office_appointments SET
                    title = ?1,
                    occurs_on = ?2,
                    starts_at = NULLIF(?3,''),
                    notes = NULLIF(?4,''),
                    updated_at = ?5
                 WHERE id = ?6 AND user_id = ?7",
                params![
                    appointment.title,
                    appointment.occurs_on,
                    appointment.starts_at.clone().unwrap_or_default(),
                    appointment.notes.clone().unwrap_or_default(),
                    appointment.updated_at,
                    appointment.id,
                    appointment.user_id,
                ],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err("الموعد غير موجود".to_string());
        }
        Ok(())
    }

    fn delete_appointment(&self, id: &str, user_id: &str) -> Result<(), String> {
        let changed = self
            .conn
            .execute(
                "DELETE FROM office_appointments WHERE id = ?1 AND user_id = ?2",
                params![id, user_id],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err("الموعد غير موجود".to_string());
        }
        Ok(())
    }

    // ===== Broker finance =====
    fn list_finance_operations(
        &self,
        user_id: &str,
        period_from: &str,
        period_to: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<FinanceOperation>, String> {
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(user_id.to_string())];
        let (extra, extra_params, mut idx) = date_range_clause("o.occurs_on", period_from, period_to, 2);
        params_vec.extend(extra_params);
        let mut limit_sql = String::new();
        if limit > 0 {
            limit_sql = format!(" LIMIT ?{idx} OFFSET ?{}", idx + 1);
            params_vec.push(Box::new(limit));
            params_vec.push(Box::new(offset.max(0)));
        }
        let sql = format!(
            "SELECT {FIN_OP_COLS}, cl.name, pr.name, mc.name \
             FROM finance_operations o \
             LEFT JOIN clients cl ON cl.id = o.client_id \
             LEFT JOIN properties pr ON pr.id = o.property_id \
             LEFT JOIN marketing_campaigns mc ON mc.id = o.campaign_id \
             WHERE o.user_id = ?1{extra} ORDER BY o.occurs_on DESC, o.updated_at DESC{limit_sql}"
        );
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(
                rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())),
                row_to_finance_operation,
            )
            .map_err(|e| e.to_string())?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row.map_err(|e| e.to_string())?);
        }
        Ok(items)
    }

    fn get_finance_operation(&self, id: &str, user_id: &str) -> Result<Option<FinanceOperation>, String> {
        let sql = format!(
            "SELECT {FIN_OP_COLS}, cl.name, pr.name, mc.name \
             FROM finance_operations o \
             LEFT JOIN clients cl ON cl.id = o.client_id \
             LEFT JOIN properties pr ON pr.id = o.property_id \
             LEFT JOIN marketing_campaigns mc ON mc.id = o.campaign_id \
             WHERE o.id = ?1 AND o.user_id = ?2"
        );
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![id, user_id]).map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            Ok(Some(row_to_finance_operation(row).map_err(|e| e.to_string())?))
        } else {
            Ok(None)
        }
    }

    fn create_finance_operation(&self, item: &FinanceOperation) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO finance_operations (
                    id, user_id, title, occurs_on, deal_value, client_id, property_id, campaign_id, notes, created_at, updated_at
                 ) VALUES (?1,?2,?3,?4,?5,NULLIF(?6,''),NULLIF(?7,''),NULLIF(?8,''),NULLIF(?9,''),?10,?11)",
                params![
                    item.id,
                    item.user_id,
                    item.title,
                    item.occurs_on,
                    item.deal_value,
                    item.client_id.clone().unwrap_or_default(),
                    item.property_id.clone().unwrap_or_default(),
                    item.campaign_id.clone().unwrap_or_default(),
                    item.notes.clone().unwrap_or_default(),
                    item.created_at,
                    item.updated_at,
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn update_finance_operation(&self, item: &FinanceOperation) -> Result<(), String> {
        let changed = self
            .conn
            .execute(
                "UPDATE finance_operations SET
                    title=?1, occurs_on=?2, deal_value=?3, client_id=NULLIF(?4,''), property_id=NULLIF(?5,''),
                    campaign_id=NULLIF(?6,''), notes=NULLIF(?7,''), updated_at=?8
                 WHERE id=?9 AND user_id=?10",
                params![
                    item.title,
                    item.occurs_on,
                    item.deal_value,
                    item.client_id.clone().unwrap_or_default(),
                    item.property_id.clone().unwrap_or_default(),
                    item.campaign_id.clone().unwrap_or_default(),
                    item.notes.clone().unwrap_or_default(),
                    item.updated_at,
                    item.id,
                    item.user_id,
                ],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err("العملية غير موجودة".to_string());
        }
        Ok(())
    }

    fn delete_finance_operation(&self, id: &str, user_id: &str) -> Result<(), String> {
        let changed = self
            .conn
            .execute(
                "DELETE FROM finance_operations WHERE id = ?1 AND user_id = ?2",
                params![id, user_id],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err("العملية غير موجودة".to_string());
        }
        Ok(())
    }

    fn list_finance_invoices(
        &self,
        user_id: &str,
        period_from: &str,
        period_to: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<FinanceInvoice>, String> {
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(user_id.to_string())];
        let (extra, extra_params, mut idx) = date_range_clause("i.issue_date", period_from, period_to, 2);
        params_vec.extend(extra_params);
        let mut limit_sql = String::new();
        if limit > 0 {
            limit_sql = format!(" LIMIT ?{idx} OFFSET ?{}", idx + 1);
            params_vec.push(Box::new(limit));
            params_vec.push(Box::new(offset.max(0)));
        }
        let sql = format!(
            "SELECT {FIN_INV_COLS}, cl.name, pr.name, mc.name, fo.title, \
             COALESCE((SELECT SUM(p.amount) FROM finance_payments p WHERE p.user_id = i.user_id AND p.target_type = 'invoice' AND p.target_id = i.id), 0) \
             FROM finance_invoices i \
             LEFT JOIN clients cl ON cl.id = i.client_id \
             LEFT JOIN properties pr ON pr.id = i.property_id \
             LEFT JOIN marketing_campaigns mc ON mc.id = i.campaign_id \
             LEFT JOIN finance_operations fo ON fo.id = i.operation_id \
             WHERE i.user_id = ?1{extra} ORDER BY i.issue_date DESC, i.updated_at DESC{limit_sql}"
        );
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(
                rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())),
                row_to_finance_invoice,
            )
            .map_err(|e| e.to_string())?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row.map_err(|e| e.to_string())?);
        }
        Ok(items)
    }

    fn get_finance_invoice(&self, id: &str, user_id: &str) -> Result<Option<FinanceInvoice>, String> {
        let sql = format!(
            "SELECT {FIN_INV_COLS}, cl.name, pr.name, mc.name, fo.title, \
             COALESCE((SELECT SUM(p.amount) FROM finance_payments p WHERE p.user_id = i.user_id AND p.target_type = 'invoice' AND p.target_id = i.id), 0) \
             FROM finance_invoices i \
             LEFT JOIN clients cl ON cl.id = i.client_id \
             LEFT JOIN properties pr ON pr.id = i.property_id \
             LEFT JOIN marketing_campaigns mc ON mc.id = i.campaign_id \
             LEFT JOIN finance_operations fo ON fo.id = i.operation_id \
             WHERE i.id = ?1 AND i.user_id = ?2"
        );
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![id, user_id]).map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            Ok(Some(row_to_finance_invoice(row).map_err(|e| e.to_string())?))
        } else {
            Ok(None)
        }
    }

    fn create_finance_invoice(&self, item: &FinanceInvoice) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO finance_invoices (
                    id, user_id, number, title, issue_date, due_date, amount, client_id, property_id,
                    campaign_id, operation_id, notes, created_at, updated_at
                 ) VALUES (?1,?2,?3,?4,?5,NULLIF(?6,''),?7,NULLIF(?8,''),NULLIF(?9,''),NULLIF(?10,''),NULLIF(?11,''),NULLIF(?12,''),?13,?14)",
                params![
                    item.id,
                    item.user_id,
                    item.number,
                    item.title,
                    item.issue_date,
                    item.due_date.clone().unwrap_or_default(),
                    item.amount,
                    item.client_id.clone().unwrap_or_default(),
                    item.property_id.clone().unwrap_or_default(),
                    item.campaign_id.clone().unwrap_or_default(),
                    item.operation_id.clone().unwrap_or_default(),
                    item.notes.clone().unwrap_or_default(),
                    item.created_at,
                    item.updated_at,
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn update_finance_invoice(&self, item: &FinanceInvoice) -> Result<(), String> {
        let changed = self
            .conn
            .execute(
                "UPDATE finance_invoices SET
                    number=?1, title=?2, issue_date=?3, due_date=NULLIF(?4,''), amount=?5,
                    client_id=NULLIF(?6,''), property_id=NULLIF(?7,''), campaign_id=NULLIF(?8,''),
                    operation_id=NULLIF(?9,''), notes=NULLIF(?10,''), updated_at=?11
                 WHERE id=?12 AND user_id=?13",
                params![
                    item.number,
                    item.title,
                    item.issue_date,
                    item.due_date.clone().unwrap_or_default(),
                    item.amount,
                    item.client_id.clone().unwrap_or_default(),
                    item.property_id.clone().unwrap_or_default(),
                    item.campaign_id.clone().unwrap_or_default(),
                    item.operation_id.clone().unwrap_or_default(),
                    item.notes.clone().unwrap_or_default(),
                    item.updated_at,
                    item.id,
                    item.user_id,
                ],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err("الفاتورة غير موجودة".to_string());
        }
        Ok(())
    }

    fn delete_finance_invoice(&self, id: &str, user_id: &str) -> Result<(), String> {
        let changed = self
            .conn
            .execute(
                "DELETE FROM finance_invoices WHERE id = ?1 AND user_id = ?2",
                params![id, user_id],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err("الفاتورة غير موجودة".to_string());
        }
        Ok(())
    }

    fn next_finance_invoice_number(&self, user_id: &str, year: i32) -> Result<String, String> {
        let prefix = format!("F-{year}-");
        let last: Option<String> = self
            .conn
            .query_row(
                "SELECT number FROM finance_invoices WHERE user_id = ?1 AND number LIKE ?2 ORDER BY number DESC LIMIT 1",
                params![user_id, format!("{prefix}%")],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        Ok(crate::finance::next_invoice_number(year, last.as_deref()))
    }

    fn list_finance_commissions(
        &self,
        user_id: &str,
        period_from: &str,
        period_to: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<FinanceCommission>, String> {
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(user_id.to_string())];
        let (extra, extra_params, mut idx) = date_range_clause("c.occurs_on", period_from, period_to, 2);
        params_vec.extend(extra_params);
        let mut limit_sql = String::new();
        if limit > 0 {
            limit_sql = format!(" LIMIT ?{idx} OFFSET ?{}", idx + 1);
            params_vec.push(Box::new(limit));
            params_vec.push(Box::new(offset.max(0)));
        }
        let sql = format!(
            "SELECT {FIN_COM_COLS}, cl.name, pr.name, mc.name, fo.title, inv.number, \
             COALESCE((SELECT SUM(p.amount) FROM finance_payments p WHERE p.user_id = c.user_id AND p.target_type = 'commission' AND p.target_id = c.id), 0) \
             FROM finance_commissions c \
             LEFT JOIN clients cl ON cl.id = c.client_id \
             LEFT JOIN properties pr ON pr.id = c.property_id \
             LEFT JOIN marketing_campaigns mc ON mc.id = c.campaign_id \
             LEFT JOIN finance_operations fo ON fo.id = c.operation_id \
             LEFT JOIN finance_invoices inv ON inv.id = c.invoice_id \
             WHERE c.user_id = ?1{extra} ORDER BY c.occurs_on DESC, c.updated_at DESC{limit_sql}"
        );
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(
                rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())),
                row_to_finance_commission,
            )
            .map_err(|e| e.to_string())?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row.map_err(|e| e.to_string())?);
        }
        Ok(items)
    }

    fn get_finance_commission(&self, id: &str, user_id: &str) -> Result<Option<FinanceCommission>, String> {
        let sql = format!(
            "SELECT {FIN_COM_COLS}, cl.name, pr.name, mc.name, fo.title, inv.number, \
             COALESCE((SELECT SUM(p.amount) FROM finance_payments p WHERE p.user_id = c.user_id AND p.target_type = 'commission' AND p.target_id = c.id), 0) \
             FROM finance_commissions c \
             LEFT JOIN clients cl ON cl.id = c.client_id \
             LEFT JOIN properties pr ON pr.id = c.property_id \
             LEFT JOIN marketing_campaigns mc ON mc.id = c.campaign_id \
             LEFT JOIN finance_operations fo ON fo.id = c.operation_id \
             LEFT JOIN finance_invoices inv ON inv.id = c.invoice_id \
             WHERE c.id = ?1 AND c.user_id = ?2"
        );
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![id, user_id]).map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            Ok(Some(row_to_finance_commission(row).map_err(|e| e.to_string())?))
        } else {
            Ok(None)
        }
    }

    fn create_finance_commission(&self, item: &FinanceCommission) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO finance_commissions (
                    id, user_id, title, occurs_on, mode, basis_amount, rate_percent, amount_fixed,
                    office_share_pct, broker_share_pct, marketer_share_pct, partner_share_pct,
                    client_id, property_id, campaign_id, operation_id, invoice_id, notes, created_at, updated_at
                 ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,NULLIF(?13,''),NULLIF(?14,''),NULLIF(?15,''),NULLIF(?16,''),NULLIF(?17,''),NULLIF(?18,''),?19,?20)",
                params![
                    item.id,
                    item.user_id,
                    item.title,
                    item.occurs_on,
                    item.mode,
                    item.basis_amount,
                    item.rate_percent,
                    item.amount_fixed,
                    item.office_share_pct,
                    item.broker_share_pct,
                    item.marketer_share_pct,
                    item.partner_share_pct,
                    item.client_id.clone().unwrap_or_default(),
                    item.property_id.clone().unwrap_or_default(),
                    item.campaign_id.clone().unwrap_or_default(),
                    item.operation_id.clone().unwrap_or_default(),
                    item.invoice_id.clone().unwrap_or_default(),
                    item.notes.clone().unwrap_or_default(),
                    item.created_at,
                    item.updated_at,
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn update_finance_commission(&self, item: &FinanceCommission) -> Result<(), String> {
        let changed = self
            .conn
            .execute(
                "UPDATE finance_commissions SET
                    title=?1, occurs_on=?2, mode=?3, basis_amount=?4, rate_percent=?5, amount_fixed=?6,
                    office_share_pct=?7, broker_share_pct=?8, marketer_share_pct=?9, partner_share_pct=?10,
                    client_id=NULLIF(?11,''), property_id=NULLIF(?12,''), campaign_id=NULLIF(?13,''),
                    operation_id=NULLIF(?14,''), invoice_id=NULLIF(?15,''), notes=NULLIF(?16,''), updated_at=?17
                 WHERE id=?18 AND user_id=?19",
                params![
                    item.title,
                    item.occurs_on,
                    item.mode,
                    item.basis_amount,
                    item.rate_percent,
                    item.amount_fixed,
                    item.office_share_pct,
                    item.broker_share_pct,
                    item.marketer_share_pct,
                    item.partner_share_pct,
                    item.client_id.clone().unwrap_or_default(),
                    item.property_id.clone().unwrap_or_default(),
                    item.campaign_id.clone().unwrap_or_default(),
                    item.operation_id.clone().unwrap_or_default(),
                    item.invoice_id.clone().unwrap_or_default(),
                    item.notes.clone().unwrap_or_default(),
                    item.updated_at,
                    item.id,
                    item.user_id,
                ],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err("العمولة غير موجودة".to_string());
        }
        Ok(())
    }

    fn delete_finance_commission(&self, id: &str, user_id: &str) -> Result<(), String> {
        let changed = self
            .conn
            .execute(
                "DELETE FROM finance_commissions WHERE id = ?1 AND user_id = ?2",
                params![id, user_id],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err("العمولة غير موجودة".to_string());
        }
        Ok(())
    }

    fn list_finance_expenses(
        &self,
        user_id: &str,
        period_from: &str,
        period_to: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<FinanceExpense>, String> {
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(user_id.to_string())];
        let (extra, extra_params, mut idx) = date_range_clause("e.occurs_on", period_from, period_to, 2);
        params_vec.extend(extra_params);
        let mut limit_sql = String::new();
        if limit > 0 {
            limit_sql = format!(" LIMIT ?{idx} OFFSET ?{}", idx + 1);
            params_vec.push(Box::new(limit));
            params_vec.push(Box::new(offset.max(0)));
        }
        let sql = format!(
            "SELECT {FIN_EXP_COLS}, cl.name, pr.name, mc.name \
             FROM finance_expenses e \
             LEFT JOIN clients cl ON cl.id = e.client_id \
             LEFT JOIN properties pr ON pr.id = e.property_id \
             LEFT JOIN marketing_campaigns mc ON mc.id = e.campaign_id \
             WHERE e.user_id = ?1{extra} ORDER BY e.occurs_on DESC, e.updated_at DESC{limit_sql}"
        );
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(
                rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())),
                row_to_finance_expense,
            )
            .map_err(|e| e.to_string())?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row.map_err(|e| e.to_string())?);
        }
        Ok(items)
    }

    fn get_finance_expense(&self, id: &str, user_id: &str) -> Result<Option<FinanceExpense>, String> {
        let sql = format!(
            "SELECT {FIN_EXP_COLS}, cl.name, pr.name, mc.name \
             FROM finance_expenses e \
             LEFT JOIN clients cl ON cl.id = e.client_id \
             LEFT JOIN properties pr ON pr.id = e.property_id \
             LEFT JOIN marketing_campaigns mc ON mc.id = e.campaign_id \
             WHERE e.id = ?1 AND e.user_id = ?2"
        );
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![id, user_id]).map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            Ok(Some(row_to_finance_expense(row).map_err(|e| e.to_string())?))
        } else {
            Ok(None)
        }
    }

    fn create_finance_expense(&self, item: &FinanceExpense) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO finance_expenses (
                    id, user_id, title, occurs_on, amount, category, client_id, property_id, campaign_id, reference, notes, created_at, updated_at
                 ) VALUES (?1,?2,?3,?4,?5,NULLIF(?6,''),NULLIF(?7,''),NULLIF(?8,''),NULLIF(?9,''),NULLIF(?10,''),NULLIF(?11,''),?12,?13)",
                params![
                    item.id,
                    item.user_id,
                    item.title,
                    item.occurs_on,
                    item.amount,
                    item.category.clone().unwrap_or_default(),
                    item.client_id.clone().unwrap_or_default(),
                    item.property_id.clone().unwrap_or_default(),
                    item.campaign_id.clone().unwrap_or_default(),
                    item.reference.clone().unwrap_or_default(),
                    item.notes.clone().unwrap_or_default(),
                    item.created_at,
                    item.updated_at,
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn update_finance_expense(&self, item: &FinanceExpense) -> Result<(), String> {
        let changed = self
            .conn
            .execute(
                "UPDATE finance_expenses SET
                    title=?1, occurs_on=?2, amount=?3, category=NULLIF(?4,''), client_id=NULLIF(?5,''),
                    property_id=NULLIF(?6,''), campaign_id=NULLIF(?7,''), reference=NULLIF(?8,''), notes=NULLIF(?9,''), updated_at=?10
                 WHERE id=?11 AND user_id=?12",
                params![
                    item.title,
                    item.occurs_on,
                    item.amount,
                    item.category.clone().unwrap_or_default(),
                    item.client_id.clone().unwrap_or_default(),
                    item.property_id.clone().unwrap_or_default(),
                    item.campaign_id.clone().unwrap_or_default(),
                    item.reference.clone().unwrap_or_default(),
                    item.notes.clone().unwrap_or_default(),
                    item.updated_at,
                    item.id,
                    item.user_id,
                ],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err("المصروف غير موجود".to_string());
        }
        Ok(())
    }

    fn delete_finance_expense(&self, id: &str, user_id: &str) -> Result<(), String> {
        let changed = self
            .conn
            .execute(
                "DELETE FROM finance_expenses WHERE id = ?1 AND user_id = ?2",
                params![id, user_id],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err("المصروف غير موجود".to_string());
        }
        Ok(())
    }

    fn list_finance_payments(
        &self,
        user_id: &str,
        period_from: &str,
        period_to: &str,
        target_type: &str,
        target_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<FinancePayment>, String> {
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(user_id.to_string())];
        let mut idx = 2;
        let (extra_dates, extra_params, next_idx) =
            date_range_clause("p.occurs_on", period_from, period_to, idx);
        idx = next_idx;
        params_vec.extend(extra_params);
        let mut extra = extra_dates;
        if !target_type.is_empty() {
            extra.push_str(&format!(" AND p.target_type = ?{idx}"));
            params_vec.push(Box::new(target_type.to_string()));
            idx += 1;
        }
        if !target_id.is_empty() {
            extra.push_str(&format!(" AND p.target_id = ?{idx}"));
            params_vec.push(Box::new(target_id.to_string()));
            idx += 1;
        }
        let mut limit_sql = String::new();
        if limit > 0 {
            limit_sql = format!(" LIMIT ?{idx} OFFSET ?{}", idx + 1);
            params_vec.push(Box::new(limit));
            params_vec.push(Box::new(offset.max(0)));
        }
        let sql = format!(
            "SELECT {FIN_PAY_COLS}, \
             CASE p.target_type \
               WHEN 'invoice' THEN (SELECT i.number || ' — ' || i.title FROM finance_invoices i WHERE i.id = p.target_id) \
               WHEN 'commission' THEN (SELECT c.title FROM finance_commissions c WHERE c.id = p.target_id) \
               ELSE NULL END \
             FROM finance_payments p \
             WHERE p.user_id = ?1{extra} ORDER BY p.occurs_on DESC, p.updated_at DESC{limit_sql}"
        );
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(
                rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())),
                row_to_finance_payment,
            )
            .map_err(|e| e.to_string())?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row.map_err(|e| e.to_string())?);
        }
        Ok(items)
    }

    fn get_finance_payment(&self, id: &str, user_id: &str) -> Result<Option<FinancePayment>, String> {
        let sql = format!(
            "SELECT {FIN_PAY_COLS}, \
             CASE p.target_type \
               WHEN 'invoice' THEN (SELECT i.number || ' — ' || i.title FROM finance_invoices i WHERE i.id = p.target_id) \
               WHEN 'commission' THEN (SELECT c.title FROM finance_commissions c WHERE c.id = p.target_id) \
               ELSE NULL END \
             FROM finance_payments p WHERE p.id = ?1 AND p.user_id = ?2"
        );
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![id, user_id]).map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            Ok(Some(row_to_finance_payment(row).map_err(|e| e.to_string())?))
        } else {
            Ok(None)
        }
    }

    fn create_finance_payment(&self, item: &FinancePayment) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO finance_payments (
                    id, user_id, occurs_on, amount, target_type, target_id, method, reference, notes, created_at, updated_at
                 ) VALUES (?1,?2,?3,?4,?5,?6,NULLIF(?7,''),NULLIF(?8,''),NULLIF(?9,''),?10,?11)",
                params![
                    item.id,
                    item.user_id,
                    item.occurs_on,
                    item.amount,
                    item.target_type,
                    item.target_id,
                    item.method.clone().unwrap_or_default(),
                    item.reference.clone().unwrap_or_default(),
                    item.notes.clone().unwrap_or_default(),
                    item.created_at,
                    item.updated_at,
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn update_finance_payment(&self, item: &FinancePayment) -> Result<(), String> {
        let changed = self
            .conn
            .execute(
                "UPDATE finance_payments SET
                    occurs_on=?1, amount=?2, target_type=?3, target_id=?4, method=NULLIF(?5,''),
                    reference=NULLIF(?6,''), notes=NULLIF(?7,''), updated_at=?8
                 WHERE id=?9 AND user_id=?10",
                params![
                    item.occurs_on,
                    item.amount,
                    item.target_type,
                    item.target_id,
                    item.method.clone().unwrap_or_default(),
                    item.reference.clone().unwrap_or_default(),
                    item.notes.clone().unwrap_or_default(),
                    item.updated_at,
                    item.id,
                    item.user_id,
                ],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err("الدفعة غير موجودة".to_string());
        }
        Ok(())
    }

    fn delete_finance_payment(&self, id: &str, user_id: &str) -> Result<(), String> {
        let changed = self
            .conn
            .execute(
                "DELETE FROM finance_payments WHERE id = ?1 AND user_id = ?2",
                params![id, user_id],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err("الدفعة غير موجودة".to_string());
        }
        Ok(())
    }

    fn delete_finance_payments_for_target(
        &self,
        user_id: &str,
        target_type: &str,
        target_id: &str,
    ) -> Result<(), String> {
        self.conn
            .execute(
                "DELETE FROM finance_payments WHERE user_id = ?1 AND target_type = ?2 AND target_id = ?3",
                params![user_id, target_type, target_id],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ===== إجمالي العدّ الحقيقي للترقيم الخلفي =====

    fn count_finance_operations(&self, user_id: &str, period_from: &str, period_to: &str) -> Result<i64, String> {
        let (extra, extra_params, _) = date_range_clause("o.occurs_on", period_from, period_to, 2);
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(user_id.to_string())];
        params_vec.extend(extra_params);
        let sql = format!("SELECT COUNT(*) FROM finance_operations o WHERE o.user_id = ?1{extra}");
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        Ok(stmt
            .query_row(
                rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())),
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?)
    }

    fn count_finance_invoices(&self, user_id: &str, period_from: &str, period_to: &str) -> Result<i64, String> {
        let (extra, extra_params, _) = date_range_clause("i.issue_date", period_from, period_to, 2);
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(user_id.to_string())];
        params_vec.extend(extra_params);
        let sql = format!("SELECT COUNT(*) FROM finance_invoices i WHERE i.user_id = ?1{extra}");
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        Ok(stmt
            .query_row(
                rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())),
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?)
    }

    fn count_finance_commissions(&self, user_id: &str, period_from: &str, period_to: &str) -> Result<i64, String> {
        let (extra, extra_params, _) = date_range_clause("c.occurs_on", period_from, period_to, 2);
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(user_id.to_string())];
        params_vec.extend(extra_params);
        let sql = format!("SELECT COUNT(*) FROM finance_commissions c WHERE c.user_id = ?1{extra}");
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        Ok(stmt
            .query_row(
                rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())),
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?)
    }

    fn count_finance_expenses(&self, user_id: &str, period_from: &str, period_to: &str) -> Result<i64, String> {
        let (extra, extra_params, _) = date_range_clause("e.occurs_on", period_from, period_to, 2);
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(user_id.to_string())];
        params_vec.extend(extra_params);
        let sql = format!("SELECT COUNT(*) FROM finance_expenses e WHERE e.user_id = ?1{extra}");
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        Ok(stmt
            .query_row(
                rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())),
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?)
    }

    fn count_finance_payments(
        &self,
        user_id: &str,
        period_from: &str,
        period_to: &str,
        target_type: &str,
        target_id: &str,
    ) -> Result<i64, String> {
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(user_id.to_string())];
        let mut idx = 2;
        let (extra_dates, extra_params, next_idx) =
            date_range_clause("p.occurs_on", period_from, period_to, idx);
        idx = next_idx;
        params_vec.extend(extra_params);
        let mut extra = extra_dates;
        if !target_type.is_empty() {
            extra.push_str(&format!(" AND p.target_type = ?{idx}"));
            params_vec.push(Box::new(target_type.to_string()));
            idx += 1;
        }
        if !target_id.is_empty() {
            extra.push_str(&format!(" AND p.target_id = ?{idx}"));
            params_vec.push(Box::new(target_id.to_string()));
        }
        let sql = format!("SELECT COUNT(*) FROM finance_payments p WHERE p.user_id = ?1{extra}");
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        Ok(stmt
            .query_row(
                rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())),
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?)
    }

    fn count_brochures(&self, user_id: &str) -> Result<i64, String> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM brochures WHERE user_id = ?1",
                params![user_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())
    }

    // ===== Brochures =====
    fn list_brochures(&self, user_id: &str, limit: i64, offset: i64) -> Result<Vec<serde_json::Value>, String> {
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(user_id.to_string())];
        let mut limit_sql = String::new();
        if limit > 0 {
            limit_sql = " LIMIT ?2 OFFSET ?3".to_string();
            params_vec.push(Box::new(limit));
            params_vec.push(Box::new(offset.max(0)));
        }
        let mut stmt = self
            .conn
            .prepare(&format!(
                "SELECT {}, p.name as property_name FROM brochures b \
                 LEFT JOIN properties p ON p.id = b.property_id \
                 WHERE b.user_id = ?1 ORDER BY COALESCE(b.updated_at, b.created_at) DESC{limit_sql}",
                brochure_cols_prefixed()
            ))
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(
                rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())),
                |row| {
                Ok(serde_json::json!({
                    "broker_license_type":row.get::<_,Option<String>>("broker_license_type")?,"broker_license_name":row.get::<_,Option<String>>("broker_license_name")?,"broker_license_number":row.get::<_,Option<String>>("broker_license_number")?,
                    "country": row.get::<_, String>("country")?, "currency": row.get::<_, String>("currency")?,
                    "id": row.get::<_, String>(0)?,
                    "user_id": row.get::<_, String>(1)?,
                    "property_id": row.get::<_, Option<String>>(2)?,
                    "template_id": row.get::<_, Option<String>>(3)?,
                    "title": row.get::<_, Option<String>>(4)?,
                    "content": row.get::<_, Option<String>>(5)?,
                    "created_at": row.get::<_, String>(6)?,
                    "updated_at": row.get::<_, Option<String>>(7)?,
                    "property_name": row.get::<_, Option<String>>(13)?,
                }))
            })
            .map_err(|e| e.to_string())?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row.map_err(|e| e.to_string())?);
        }
        Ok(items)
    }

    fn get_brochure(&self, id: &str, user_id: &str) -> Result<Option<serde_json::Value>, String> {
        let mut stmt = self
            .conn
            .prepare(&format!(
                "SELECT {}, p.name as property_name, p.city, p.district, p.price, p.area, p.main_image \
                 FROM brochures b LEFT JOIN properties p ON p.id = b.property_id \
                 WHERE b.id = ?1 AND b.user_id = ?2",
                brochure_cols_prefixed()
            ))
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![id, user_id]).map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let content_raw: Option<String> = row.get(5).map_err(|e| e.to_string())?;
            let content = content_raw
                .and_then(|c| serde_json::from_str(&c).ok())
                .unwrap_or(serde_json::json!({}));
            Ok(Some(serde_json::json!({
                "broker_license_type":row.get::<_,Option<String>>("broker_license_type").map_err(|e|e.to_string())?,"broker_license_name":row.get::<_,Option<String>>("broker_license_name").map_err(|e|e.to_string())?,"broker_license_number":row.get::<_,Option<String>>("broker_license_number").map_err(|e|e.to_string())?,
                "country": row.get::<_, String>("country").map_err(|e|e.to_string())?, "currency": row.get::<_, String>("currency").map_err(|e|e.to_string())?,
                "id": row.get::<_, String>(0).map_err(|e| e.to_string())?,
                "user_id": row.get::<_, String>(1).map_err(|e| e.to_string())?,
                "property_id": row.get::<_, Option<String>>(2).map_err(|e| e.to_string())?,
                "template_id": row.get::<_, Option<String>>(3).map_err(|e| e.to_string())?,
                "title": row.get::<_, Option<String>>(4).map_err(|e| e.to_string())?,
                "content": content,
                "created_at": row.get::<_, String>(6).map_err(|e| e.to_string())?,
                "updated_at": row.get::<_, Option<String>>(7).map_err(|e| e.to_string())?,
                "property_name": row.get::<_, Option<String>>(13).map_err(|e| e.to_string())?,
                "city": row.get::<_, Option<String>>(14).map_err(|e| e.to_string())?,
                "district": row.get::<_, Option<String>>(15).map_err(|e| e.to_string())?,
                "price": row.get::<_, Option<f64>>(16).map_err(|e| e.to_string())?,
                "area": row.get::<_, Option<f64>>(17).map_err(|e| e.to_string())?,
                "main_image": row.get::<_, Option<String>>(18).map_err(|e| e.to_string())?,
            })))
        } else {
            Ok(None)
        }
    }

    fn create_brochure(&self, brochure: &Brochure) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO brochures (id, user_id, property_id, template_id, title, content, updated_at) \
                 VALUES (?1, ?2, NULLIF(?3,''), ?4, ?5, ?6, datetime('now'))",
                params![
                    brochure.id,
                    brochure.user_id,
                    brochure.property_id.clone().unwrap_or_default(),
                    brochure.template_id,
                    brochure.title,
                    brochure.content
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn update_brochure(
        &self,
        id: &str,
        user_id: &str,
        property_id: Option<&str>,
        template_id: &str,
        title: &str,
        content: &str,
    ) -> Result<(), String> {
        let changed = self
            .conn
            .execute(
                "UPDATE brochures SET property_id = NULLIF(?1,''), template_id = ?2, title = ?3, content = ?4, \
                 updated_at = datetime('now') WHERE id = ?5 AND user_id = ?6",
                params![
                    property_id.unwrap_or(""),
                    template_id,
                    title,
                    content,
                    id,
                    user_id
                ],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err("الملف غير موجود في مكتبة العروض".to_string());
        }
        Ok(())
    }

    fn delete_brochure(&self, id: &str, user_id: &str) -> Result<(), String> {
        self.conn
            .execute("DELETE FROM brochures WHERE id = ?1 AND user_id = ?2", params![id, user_id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ===== Settings =====
    fn get_settings(&self, user_id: &str) -> Result<Option<Settings>, String> {
        let mut stmt = self
            .conn
            .prepare(&format!("SELECT {SETTINGS_COLS} FROM settings WHERE user_id = ?1"))
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![user_id]).map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            Ok(Some(row_to_settings(row).map_err(|e| e.to_string())?))
        } else {
            Ok(None)
        }
    }

    fn create_settings(&self, user_id: &str) -> Result<(), String> {
        self.conn
            .execute("INSERT INTO settings (user_id) VALUES (?1)", params![user_id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn update_settings(&self, settings: &Settings) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE settings SET office_name = NULLIF(?1,''), office_logo = NULLIF(?2,''), office_phone = NULLIF(?3,''), office_whatsapp = NULLIF(?4,''), office_email = NULLIF(?5,''), office_address = NULLIF(?6,''), broker_name = NULLIF(?7,''), broker_phone = NULLIF(?8,''), broker_whatsapp = NULLIF(?9,''), default_brochure_template = COALESCE(NULLIF(?10,''), default_brochure_template), fal_license_number = NULLIF(?11,''), fal_license_type = NULLIF(?12,''), ui_language = NULLIF(?13,''), display_currency = NULLIF(?14,''), tracking_endpoint_url = NULLIF(?15,''), country = ?17, broker_license_name = NULLIF(?18,''), updated_at = datetime('now') WHERE user_id = ?16",
                params![settings.office_name, settings.office_logo, settings.office_phone, settings.office_whatsapp, settings.office_email, settings.office_address, settings.broker_name, settings.broker_phone, settings.broker_whatsapp, settings.default_brochure_template, settings.fal_license_number, settings.fal_license_type, settings.ui_language, settings.display_currency, settings.tracking_endpoint_url, settings.user_id, settings.country, settings.broker_license_name],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ===== Audit Logs =====
    fn create_audit_log(&self, log: &AuditLog) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO audit_logs (id, user_id, action, entity_type, entity_id, details) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![log.id, log.user_id, log.action, log.entity_type, log.entity_id, log.details],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn list_audit_logs(&self, user_id: &str, limit: i64) -> Result<Vec<AuditLog>, String> {
        let mut stmt = self
            .conn
            .prepare(&format!("SELECT {AUDIT_LOG_COLS} FROM audit_logs WHERE user_id = ?1 ORDER BY created_at DESC LIMIT ?2"))
            .map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![user_id, limit], row_to_audit_log).map_err(|e| e.to_string())?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row.map_err(|e| e.to_string())?);
        }
        Ok(items)
    }

    // ===== Dashboard =====
    fn get_dashboard_stats(&self, user_id: &str) -> Result<serde_json::Value, String> {
        let property_stats = self.conn.query_row(
            "SELECT COUNT(*) as total, \
                 SUM(CASE WHEN status = 'available' THEN 1 ELSE 0 END) as available, \
                 SUM(CASE WHEN status = 'negotiating' THEN 1 ELSE 0 END) as negotiating, \
                 SUM(CASE WHEN status = 'reserved' THEN 1 ELSE 0 END) as reserved, \
                 SUM(CASE WHEN status = 'sold' THEN 1 ELSE 0 END) as sold, \
                 SUM(CASE WHEN status = 'expired' THEN 1 ELSE 0 END) as expired \
                 FROM properties WHERE user_id = ?1",
            params![user_id],
            |r| {
                Ok(serde_json::json!({
                    "total": r.get::<_, i64>(0)?,
                    "available": r.get::<_, Option<i64>>(1)?.unwrap_or(0),
                    "negotiating": r.get::<_, Option<i64>>(2)?.unwrap_or(0),
                    "reserved": r.get::<_, Option<i64>>(3)?.unwrap_or(0),
                    "sold": r.get::<_, Option<i64>>(4)?.unwrap_or(0),
                    "expired": r.get::<_, Option<i64>>(5)?.unwrap_or(0),
                }))
            },
        ).map_err(|e| e.to_string())?;

        let client_stats = self.conn.query_row(
            "SELECT COUNT(*) as total, \
                 SUM(CASE WHEN follow_up_date IS NOT NULL AND follow_up_date != '' AND follow_up_date <= date('now') THEN 1 ELSE 0 END) as needs_followup \
                 FROM clients WHERE user_id = ?1",
            params![user_id],
            |r| {
                Ok(serde_json::json!({
                    "total": r.get::<_, i64>(0)?,
                    "needs_followup": r.get::<_, Option<i64>>(1)?.unwrap_or(0),
                }))
            },
        ).map_err(|e| e.to_string())?;

        let communication_count: i64 = self.conn
            .query_row(
                "SELECT COUNT(*) FROM communications WHERE user_id = ?1",
                params![user_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;

        let sent_offers: i64 = self.conn
            .query_row(
                "SELECT COUNT(*) FROM client_property_links cpl \
                     INNER JOIN clients c ON c.id = cpl.client_id \
                     WHERE c.user_id = ?1 AND cpl.link_type = 'sent'",
                params![user_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;

        let sold_count = property_stats.get("sold").and_then(|v| v.as_i64()).unwrap_or(0);

        let brochures_count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM brochures WHERE user_id = ?1",
                params![user_id],
                |r| r.get(0),
            )
            .unwrap_or(0);

        let market_rows: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM market_reference_data WHERE user_id = ?1",
                params![user_id],
                |r| r.get(0),
            )
            .unwrap_or(0);

        let campaigns_count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM marketing_campaigns WHERE user_id = ?1",
                params![user_id],
                |r| r.get(0),
            )
            .unwrap_or(0);

        let invoices_count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM finance_invoices WHERE user_id = ?1",
                params![user_id],
                |r| r.get(0),
            )
            .unwrap_or(0);

        let training_data_present = Database::has_training_data(self)?;

        Ok(serde_json::json!({
            "properties": property_stats,
            "clients": client_stats,
            "communications": communication_count,
            "sent_offers": sent_offers,
            "sales": sold_count,
            "brochures": brochures_count,
            "market_rows": market_rows,
            "campaigns": campaigns_count,
            "finance_invoices": invoices_count,
            "training_data_present": training_data_present,
        }))
    }

    fn get_recent_property_updates(&self, user_id: &str, limit: i64) -> Result<Vec<serde_json::Value>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT pu.*, p.name as property_name FROM property_updates pu JOIN properties p ON p.id = pu.property_id WHERE pu.user_id = ?1 ORDER BY pu.created_at DESC LIMIT ?2")
            .map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![user_id, limit], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "property_id": row.get::<_, String>(1)?,
                "user_id": row.get::<_, String>(2)?,
                "title": row.get::<_, Option<String>>(3)?,
                "description": row.get::<_, Option<String>>(4)?,
                "created_at": row.get::<_, String>(5)?,
                "property_name": row.get::<_, String>(6)?,
            }))
        }).map_err(|e| e.to_string())?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row.map_err(|e| e.to_string())?);
        }
        Ok(items)
    }

    // ===== Export =====
    fn export_all(&self, user_id: &str) -> Result<serde_json::Value, String> {
        let properties = self.list_properties(user_id, "", "", "", "", "", 100000, 0)?.0;
        let clients = self.list_clients(user_id, "", "", "", "", false, 100000, 0)?.0;
        let market_data = self.list_market_data(user_id, "", "", "", 100000, 0)?.0;
        let brochures = self.list_brochures(user_id, 0, 0)?;
        let campaigns = self.list_campaigns(user_id, "", "", "", "", 100000, 0)?.0;
        let appointments = self.list_appointments(user_id, "", "")?;
        let finance_operations = self.list_finance_operations(user_id, "", "", 0, 0)?;
        let finance_invoices = self.list_finance_invoices(user_id, "", "", 0, 0)?;
        let finance_commissions = self.list_finance_commissions(user_id, "", "", 0, 0)?;
        let finance_expenses = self.list_finance_expenses(user_id, "", "", 0, 0)?;
        let finance_payments = self.list_finance_payments(user_id, "", "", "", "", 0, 0)?;
        let settings = self.get_settings(user_id)?;

        Ok(serde_json::json!({
            "version": "1.0",
            "exported_at": chrono::Utc::now().to_rfc3339(),
            "user_id": user_id,
            "data": {
                "properties": properties,
                "clients": clients,
                "market_reference_data": market_data,
                "brochures": brochures,
                "marketing_campaigns": campaigns,
                "office_appointments": appointments,
                "finance_operations": finance_operations,
                "finance_invoices": finance_invoices,
                "finance_commissions": finance_commissions,
                "finance_expenses": finance_expenses,
                "finance_payments": finance_payments,
                "settings": settings,
            }
        }))
    }

    // ===== Atomic multi-step writes (P11) =====

    fn create_property_atomic(
        &self,
        property: &Property,
        user_id: &str,
        audit: &AuditLog,
    ) -> Result<(), String> {
        self.run_atomic(|| {
            self.create_property(property)?;
            super::tx::checkpoint_after_first()?;
            self.sync_owner_owned_link(user_id, &property.id, None, property.owner_client_id.as_deref())?;
            self.create_audit_log(audit)?;
            Ok(())
        })
    }

    fn update_property_atomic(
        &self,
        property: &Property,
        previous_owner: Option<&str>,
        user_id: &str,
        audit: &AuditLog,
    ) -> Result<(), String> {
        self.run_atomic(|| {
            self.update_property(property)?;
            super::tx::checkpoint_after_first()?;
            self.sync_owner_owned_link(
                user_id,
                &property.id,
                previous_owner,
                property.owner_client_id.as_deref(),
            )?;
            self.create_audit_log(audit)?;
            Ok(())
        })
    }

    fn delete_property_atomic(&self, id: &str, audit: &AuditLog) -> Result<(), String> {
        self.run_atomic(|| {
            self.delete_property(id)?;
            super::tx::checkpoint_after_first()?;
            self.create_audit_log(audit)?;
            Ok(())
        })
    }

    fn change_property_status_atomic(
        &self,
        id: &str,
        status: &str,
        update: &PropertyUpdate,
    ) -> Result<(), String> {
        self.run_atomic(|| {
            self.update_property_status(id, status)?;
            super::tx::checkpoint_after_first()?;
            self.create_property_update(update)?;
            Ok(())
        })
    }

    fn add_property_image_atomic(
        &self,
        image: &PropertyImage,
        set_as_main_url: Option<&str>,
    ) -> Result<(), String> {
        self.run_atomic(|| {
            self.create_property_image(image)?;
            super::tx::checkpoint_after_first()?;
            if let Some(url) = set_as_main_url {
                self.set_property_main_image(&image.property_id, url)?;
            }
            Ok(())
        })
    }

    fn delete_property_image_atomic(
        &self,
        image_id: &str,
        property_id: &str,
        was_main: bool,
        next_main_url: Option<&str>,
    ) -> Result<(), String> {
        self.run_atomic(|| {
            self.delete_property_image(image_id)?;
            super::tx::checkpoint_after_first()?;
            if was_main {
                if let Some(url) = next_main_url {
                    self.set_property_main_image(property_id, url)?;
                } else {
                    self.clear_property_main_image(property_id)?;
                }
            }
            Ok(())
        })
    }

    fn duplicate_property_atomic(
        &self,
        property: &Property,
        images: &[PropertyImage],
        main_image: Option<&str>,
    ) -> Result<(), String> {
        self.run_atomic(|| {
            self.create_property(property)?;
            super::tx::checkpoint_after_first()?;
            for img in images {
                self.create_property_image(img)?;
            }
            if let Some(url) = main_image.filter(|s| !s.trim().is_empty()) {
                self.set_property_main_image(&property.id, url)?;
            }
            Ok(())
        })
    }

    fn create_client_atomic(
        &self,
        client: &Client,
        roles: &[String],
        audit: &AuditLog,
    ) -> Result<(), String> {
        self.run_atomic(|| {
            self.create_client(client)?;
            super::tx::checkpoint_after_first()?;
            if !roles.is_empty() {
                self.set_client_roles(&client.id, roles)?;
            }
            self.create_audit_log(audit)?;
            Ok(())
        })
    }

    fn update_client_atomic(
        &self,
        client: &Client,
        roles: Option<&[String]>,
        audit: &AuditLog,
    ) -> Result<(), String> {
        self.run_atomic(|| {
            self.update_client(client)?;
            super::tx::checkpoint_after_first()?;
            if let Some(roles) = roles {
                self.set_client_roles(&client.id, roles)?;
            }
            self.create_audit_log(audit)?;
            Ok(())
        })
    }

    fn delete_client_atomic(&self, id: &str, audit: &AuditLog) -> Result<(), String> {
        self.run_atomic(|| {
            self.delete_client(id)?;
            super::tx::checkpoint_after_first()?;
            self.create_audit_log(audit)?;
            Ok(())
        })
    }

    fn add_communication_atomic(&self, comm: &Communication) -> Result<(), String> {
        self.run_atomic(|| {
            self.create_communication(comm)?;
            super::tx::checkpoint_after_first()?;
            self.update_client_last_contact(&comm.client_id)?;
            Ok(())
        })
    }

    fn delete_finance_invoice_atomic(&self, user_id: &str, id: &str) -> Result<(), String> {
        self.run_atomic(|| {
            self.delete_finance_payments_for_target(user_id, "invoice", id)?;
            super::tx::checkpoint_after_first()?;
            self.delete_finance_invoice(id, user_id)?;
            Ok(())
        })
    }

    fn delete_finance_commission_atomic(&self, user_id: &str, id: &str) -> Result<(), String> {
        self.run_atomic(|| {
            self.delete_finance_payments_for_target(user_id, "commission", id)?;
            super::tx::checkpoint_after_first()?;
            self.delete_finance_commission(id, user_id)?;
            Ok(())
        })
    }
}

impl SqliteDatabase {
    fn run_atomic<F>(&self, f: F) -> Result<(), String>
    where
        F: FnOnce() -> Result<(), String>,
    {
        self.conn
            .execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| format!("تعذر بدء المعاملة: {e}"))?;
        match (|| {
            f()?;
            super::tx::checkpoint_before_commit()?;
            Ok(())
        })() {
            Ok(()) => {
                self.conn
                    .execute_batch("COMMIT")
                    .map_err(|e| format!("تعذر تثبيت المعاملة: {e}"))?;
                Ok(())
            }
            Err(e) => {
                let _ = self.conn.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }

    /// يزامن رابط owned (نفس منطق الخدمة) داخل الاتصال الحالي / المعاملة.
    fn sync_owner_owned_link(
        &self,
        user_id: &str,
        property_id: &str,
        previous_owner: Option<&str>,
        owner_client_id: Option<&str>,
    ) -> Result<(), String> {
        if previous_owner == owner_client_id {
            if let Some(oid) = owner_client_id {
                if self.get_client_property_link(oid, property_id)?.is_none() {
                    self.create_client_property_link(&ClientPropertyLink {
                        id: uuid::Uuid::new_v4().to_string(),
                        client_id: oid.to_string(),
                        property_id: property_id.to_string(),
                        link_type: Some("owned".to_string()),
                        created_at: chrono::Utc::now().to_rfc3339(),
                    })?;
                }
            }
            return Ok(());
        }
        if let Some(old_id) = previous_owner {
            if let Some(link) = self.get_client_property_link(old_id, property_id)? {
                if link.link_type.as_deref() == Some("owned") {
                    let _ = self.delete_client_property_link(old_id, property_id, user_id)?;
                }
            }
        }
        if let Some(new_id) = owner_client_id {
            if let Some(link) = self.get_client_property_link(new_id, property_id)? {
                self.update_client_property_link_type(&link.id, "owned")?;
            } else {
                self.create_client_property_link(&ClientPropertyLink {
                    id: uuid::Uuid::new_v4().to_string(),
                    client_id: new_id.to_string(),
                    property_id: property_id.to_string(),
                    link_type: Some("owned".to_string()),
                    created_at: chrono::Utc::now().to_rfc3339(),
                })?;
            }
        }
        Ok(())
    }
}
