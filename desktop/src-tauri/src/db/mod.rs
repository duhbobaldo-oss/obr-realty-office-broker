// طبقة قاعدة البيانات - واجهة مجردة تسمح باستبدال SQLite بـ PostgreSQL مستقبلًا
// دون إعادة كتابة منطق التطبيق.

use serde::{Deserialize, Serialize};
use std::sync::Mutex;

pub mod backup;
pub mod connection;
pub mod demo;
pub mod training_seed;
pub mod files;
pub mod images_migration;
pub mod migrations;
pub mod sqlite;
pub mod tx;

pub use connection::{configure_connection, open_configured};

pub use sqlite::SqliteDatabase;

// ===== أنواع البيانات المشتركة =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub full_name: Option<String>,
    pub phone: Option<String>,
    pub office_name: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    pub id: String,
    pub user_id: String,
    pub license_key: String,
    pub status: String,
    pub issued_at: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub token: String,
    pub device_name: Option<String>,
    pub created_at: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Property {
    #[serde(flatten, default)]
    pub context: crate::country::RecordContext,
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub city: Option<String>,
    pub district: Option<String>,
    pub location: Option<String>,
    pub property_type: Option<String>,
    pub area: Option<f64>,
    pub price: Option<f64>,
    pub price_per_meter: Option<f64>,
    pub ad_number: Option<String>,
    pub ad_link: Option<String>,
    pub owner_name: Option<String>,
    pub owner_phone: Option<String>,
    pub owner_contact: Option<String>,
    pub owner_notes: Option<String>,
    pub marketing_platform: Option<String>,
    pub marketing_link: Option<String>,
    pub published_date: Option<String>,
    pub main_image: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    // ===== P2.2: مصدر العقار وتفاصيله الفنية =====
    /// جهة اتصال حقيقية في جدول clients (المالك/المصدر). NULL = غير مرتبط.
    pub owner_client_id: Option<String>,
    pub source_type: Option<String>,
    pub purpose: Option<String>,
    pub usage_type: Option<String>,
    pub frontage: Option<String>,
    pub street_width: Option<f64>,
    pub streets_count: Option<i64>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub description: Option<String>,
    pub listing_expiry_date: Option<String>,
    pub commission_agreement: Option<String>,
    pub fal_ad_license: Option<String>,
    // ===== بيانات داخلية حساسة: لا تُدرج في مخرجات المشاركة =====
    pub internal_price: Option<f64>,
    pub internal_notes: Option<String>,
    /// سجل تدريبي من دفعة obr-training-v1 (يظهر بوسم واضح في الواجهة)
    #[serde(default)]
    pub is_training: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyImage {
    pub id: String,
    pub property_id: String,
    pub image_url: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyUpdate {
    pub id: String,
    pub property_id: String,
    pub user_id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Client {
    #[serde(flatten, default)]
    pub context: crate::country::RecordContext,
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub phone: Option<String>,
    pub whatsapp: Option<String>,
    /// دور أساسي موروث من ما قبل P2.1. مصدر الحقيقة للأدوار هو جدول client_roles.
    pub relationship_type: Option<String>,
    pub city: Option<String>,
    pub interests: Option<String>,
    pub required_property_type: Option<String>,
    pub budget: Option<String>,
    pub notes: Option<String>,
    pub last_contact: Option<String>,
    pub follow_up_date: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    // ===== P2.1: حقول جهة الاتصال الموسعة =====
    pub email: Option<String>,
    pub phone_secondary: Option<String>,
    pub company: Option<String>,
    pub source: Option<String>,
    pub status: Option<String>,
    pub category: Option<String>,
    /// سجل تدريبي من دفعة obr-training-v1 (يظهر بوسم واضح في الواجهة)
    #[serde(default)]
    pub is_training: bool,
}

impl Property {
    /// عقار بالحقول الاختيارية فارغة. يُستخدم مع بنية التحديث `..` عند النسخ.
    pub fn blank(id: String, user_id: String, name: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            context: crate::country::RecordContext::default(),
            id,
            user_id,
            name,
            city: None,
            district: None,
            location: None,
            property_type: None,
            area: None,
            price: None,
            price_per_meter: None,
            ad_number: None,
            ad_link: None,
            owner_name: None,
            owner_phone: None,
            owner_contact: None,
            owner_notes: None,
            marketing_platform: None,
            marketing_link: None,
            published_date: None,
            main_image: None,
            status: "available".to_string(),
            created_at: now.clone(),
            updated_at: now,
            owner_client_id: None,
            source_type: None,
            purpose: None,
            usage_type: None,
            frontage: None,
            street_width: None,
            streets_count: None,
            latitude: None,
            longitude: None,
            description: None,
            listing_expiry_date: None,
            commission_agreement: None,
            fal_ad_license: None,
            internal_price: None,
            internal_notes: None,
            is_training: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Communication {
    pub id: String,
    pub user_id: String,
    pub client_id: String,
    pub r#type: Option<String>,
    pub subject: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientPropertyLink {
    pub id: String,
    pub client_id: String,
    pub property_id: String,
    pub link_type: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketReferenceData {
    #[serde(flatten, default)]
    pub context: crate::country::RecordContext,
    pub id: String,
    pub user_id: String,
    pub city: Option<String>,
    pub district: Option<String>,
    pub property_type: Option<String>,
    pub usage_type: Option<String>,
    pub area: Option<f64>,
    pub sale_price: Option<f64>,
    pub price_per_meter_sale: Option<f64>,
    pub rent_value: Option<f64>,
    pub rent_per_meter: Option<f64>,
    pub requests_count: Option<i64>,
    pub offers_count: Option<i64>,
    pub entry_date: Option<String>,
    pub data_source: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketingCampaign {
    #[serde(flatten, default)]
    pub context: crate::country::RecordContext,
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub property_id: Option<String>,
    pub platform: String,
    pub ad_link: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub views: i64,
    pub inquiries: i64,
    pub viewings: i64,
    pub deals: i64,
    pub cost: f64,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub property_name: Option<String>,
    #[serde(default)]
    pub original_ad_url: Option<String>,
    #[serde(default)]
    pub tracking_code: Option<String>,
    #[serde(default)]
    pub tracking_url: Option<String>,
    #[serde(default)]
    pub tracked_clicks: i64,
    #[serde(default)]
    pub unique_clicks: i64,
    #[serde(default)]
    pub last_click_at: Option<String>,
    #[serde(default)]
    pub tracking_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfficeAppointment {
    pub id: String,
    pub user_id: String,
    pub title: String,
    pub occurs_on: String,
    pub starts_at: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FinanceOperation {
    #[serde(flatten, default)]
    pub context: crate::country::RecordContext,
    pub id: String,
    pub user_id: String,
    pub title: String,
    pub occurs_on: String,
    pub deal_value: f64,
    pub client_id: Option<String>,
    pub property_id: Option<String>,
    pub campaign_id: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub client_name: Option<String>,
    #[serde(default)]
    pub property_name: Option<String>,
    #[serde(default)]
    pub campaign_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FinanceInvoice {
    #[serde(flatten, default)]
    pub context: crate::country::RecordContext,
    pub id: String,
    pub user_id: String,
    pub number: String,
    pub title: String,
    pub issue_date: String,
    pub due_date: Option<String>,
    pub amount: f64,
    pub client_id: Option<String>,
    pub property_id: Option<String>,
    pub campaign_id: Option<String>,
    pub operation_id: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub client_name: Option<String>,
    #[serde(default)]
    pub property_name: Option<String>,
    #[serde(default)]
    pub campaign_name: Option<String>,
    #[serde(default)]
    pub operation_title: Option<String>,
    #[serde(default)]
    pub paid: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FinanceCommission {
    #[serde(flatten, default)]
    pub context: crate::country::RecordContext,
    pub id: String,
    pub user_id: String,
    pub title: String,
    pub occurs_on: String,
    pub mode: String,
    pub basis_amount: f64,
    pub rate_percent: Option<f64>,
    pub amount_fixed: Option<f64>,
    pub office_share_pct: f64,
    pub broker_share_pct: f64,
    pub marketer_share_pct: f64,
    pub partner_share_pct: f64,
    pub client_id: Option<String>,
    pub property_id: Option<String>,
    pub campaign_id: Option<String>,
    pub operation_id: Option<String>,
    pub invoice_id: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub client_name: Option<String>,
    #[serde(default)]
    pub property_name: Option<String>,
    #[serde(default)]
    pub campaign_name: Option<String>,
    #[serde(default)]
    pub operation_title: Option<String>,
    #[serde(default)]
    pub invoice_number: Option<String>,
    #[serde(default)]
    pub paid: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FinanceExpense {
    #[serde(flatten, default)]
    pub context: crate::country::RecordContext,
    pub id: String,
    pub user_id: String,
    pub title: String,
    pub occurs_on: String,
    pub amount: f64,
    pub category: Option<String>,
    pub client_id: Option<String>,
    pub property_id: Option<String>,
    pub campaign_id: Option<String>,
    pub reference: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub client_name: Option<String>,
    #[serde(default)]
    pub property_name: Option<String>,
    #[serde(default)]
    pub campaign_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FinancePayment {
    #[serde(flatten, default)]
    pub context: crate::country::RecordContext,
    pub id: String,
    pub user_id: String,
    pub occurs_on: String,
    pub amount: f64,
    pub target_type: String,
    pub target_id: String,
    pub method: Option<String>,
    pub reference: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub target_title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Brochure {
    #[serde(flatten, default)]
    pub context: crate::country::RecordContext,
    pub id: String,
    pub user_id: String,
    pub property_id: Option<String>,
    pub template_id: Option<String>,
    pub title: Option<String>,
    pub content: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub country: String,
    pub broker_license_name: Option<String>,
    pub user_id: String,
    pub office_name: Option<String>,
    pub office_logo: Option<String>,
    pub office_phone: Option<String>,
    pub office_whatsapp: Option<String>,
    pub office_email: Option<String>,
    pub office_address: Option<String>,
    pub broker_name: Option<String>,
    pub broker_phone: Option<String>,
    pub broker_whatsapp: Option<String>,
    pub default_brochure_template: Option<String>,
    pub fal_license_number: Option<String>,
    pub fal_license_type: Option<String>,
    pub updated_at: String,
    pub ui_language: Option<String>,
    pub display_currency: Option<String>,
    #[serde(default)]
    pub tracking_endpoint_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: String,
    pub user_id: Option<String>,
    pub action: String,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub details: Option<String>,
    pub created_at: String,
}

// ===== واجهة قاعدة البيانات المجردة =====
// أي تنفيذ (SQLite, PostgreSQL, ...) يجب أن يطبق هذه الواجهة.
// منطق التطبيق (Services) يعتمد فقط على هذه الواجهة.

pub trait Database: Send {
    // ===== Users =====
    fn create_user(&self, user: &User) -> Result<(), String>;
    fn get_user_by_username_or_email(&self, username: &str, email: &str) -> Result<Option<User>, String>;
    fn get_user_by_id(&self, id: &str) -> Result<Option<User>, String>;
    fn get_first_user(&self) -> Result<Option<User>, String>;
    fn count_users(&self) -> Result<i64, String>;

    // ===== Licenses =====
    fn create_license(&self, license: &License) -> Result<(), String>;
    fn update_license_id(&self, new_id: &str, old_id: &str) -> Result<(), String>;
    fn get_license_by_user_id(&self, user_id: &str) -> Result<Option<License>, String>;
    fn get_license_by_key(&self, license_key: &str) -> Result<Option<License>, String>;

    // ===== Sessions =====
    fn create_session(&self, session: &Session) -> Result<(), String>;
    fn get_session_by_token(&self, token: &str) -> Result<Option<Session>, String>;
    fn delete_session_by_token(&self, token: &str) -> Result<(), String>;

    // ===== Training data =====
    /// هل تحتوي القاعدة على دفعة بيانات تدريبية (seed_batches غير فارغ)؟
    fn has_training_data(&self) -> Result<bool, String>;

    // ===== Properties =====
    #[allow(clippy::too_many_arguments)]
    fn list_properties(&self, user_id: &str, search: &str, status: &str, city: &str, property_type: &str, purpose: &str, limit: i64, offset: i64) -> Result<(Vec<Property>, i64), String>;
    fn get_property(&self, id: &str, user_id: &str) -> Result<Option<Property>, String>;
    fn create_property(&self, property: &Property) -> Result<(), String>;
    fn update_property(&self, property: &Property) -> Result<(), String>;
    fn delete_property(&self, id: &str) -> Result<(), String>;
    fn update_property_status(&self, id: &str, status: &str) -> Result<(), String>;

    // ===== Property Images =====
    fn list_property_images(&self, property_id: &str) -> Result<Vec<PropertyImage>, String>;
    fn get_property_image(&self, id: &str) -> Result<Option<PropertyImage>, String>;
    fn count_property_images(&self, property_id: &str) -> Result<i64, String>;
    fn count_images_for_ids(&self, ids: &[String]) -> Result<std::collections::HashMap<String, i64>, String>;
    fn create_property_image(&self, image: &PropertyImage) -> Result<(), String>;
    fn delete_property_image(&self, id: &str) -> Result<(), String>;
    fn set_property_main_image(&self, property_id: &str, image_url: &str) -> Result<(), String>;
    fn clear_property_main_image(&self, property_id: &str) -> Result<(), String>;

    // ===== Property Updates =====
    fn list_property_updates(&self, property_id: &str) -> Result<Vec<PropertyUpdate>, String>;
    fn create_property_update(&self, update: &PropertyUpdate) -> Result<(), String>;

    // ===== Clients =====
    #[allow(clippy::too_many_arguments)]
    fn list_clients(&self, user_id: &str, search: &str, role: &str, city: &str, status: &str, needs_followup: bool, limit: i64, offset: i64) -> Result<(Vec<Client>, i64), String>;
    fn get_client(&self, id: &str, user_id: &str) -> Result<Option<Client>, String>;
    fn create_client(&self, client: &Client) -> Result<(), String>;
    fn update_client(&self, client: &Client) -> Result<(), String>;
    fn delete_client(&self, id: &str) -> Result<(), String>;
    fn update_client_last_contact(&self, id: &str) -> Result<(), String>;

    // ===== Client Roles (أدوار متعددة لنفس جهة الاتصال) =====
    fn list_client_roles(&self, client_id: &str) -> Result<Vec<String>, String>;
    /// يستبدل مجموعة الأدوار بالكامل داخل معاملة واحدة.
    fn set_client_roles(&self, client_id: &str, roles: &[String]) -> Result<(), String>;
    /// أدوار مجموعة عملاء في استعلام واحد: (client_id, role).
    fn list_roles_for_clients(&self, client_ids: &[String]) -> Result<Vec<(String, String)>, String>;
    /// العملاء المتاحون للربط كمالك/مصدر عقار.
    fn list_client_options(&self, user_id: &str, search: &str, limit: i64) -> Result<Vec<serde_json::Value>, String>;

    // ===== Communications =====
    fn list_communications(&self, client_id: &str) -> Result<Vec<Communication>, String>;
    fn create_communication(&self, comm: &Communication) -> Result<(), String>;

    // ===== Client-Property Links =====
    fn get_client_property_link(&self, client_id: &str, property_id: &str) -> Result<Option<ClientPropertyLink>, String>;
    fn create_client_property_link(&self, link: &ClientPropertyLink) -> Result<(), String>;
    fn update_client_property_link_type(&self, id: &str, link_type: &str) -> Result<(), String>;
    fn delete_client_property_link(&self, client_id: &str, property_id: &str, user_id: &str) -> Result<usize, String>;
    fn list_linked_properties_for_client(&self, client_id: &str) -> Result<Vec<serde_json::Value>, String>;
    fn list_linked_clients_for_property(&self, property_id: &str) -> Result<Vec<serde_json::Value>, String>;

    // ===== Market Reference Data =====
    fn list_market_data(&self, user_id: &str, city: &str, district: &str, property_type: &str, limit: i64, offset: i64) -> Result<(Vec<MarketReferenceData>, i64), String>;
    fn get_market_stats(&self, user_id: &str, city: &str, district: &str, property_type: &str) -> Result<serde_json::Value, String>;
    fn get_market_trends(&self, user_id: &str, city: &str, district: &str, property_type: &str) -> Result<Vec<serde_json::Value>, String>;

    /// DISTINCT cities available in market reference data — dynamic filter source (SQLite).
    fn market_distinct_cities(&self, user_id: &str) -> Result<Vec<String>, String>;

    /// DISTINCT districts of one city — dependent filter source (SQLite).
    fn market_districts_of_city(&self, user_id: &str, city: &str) -> Result<Vec<String>, String>;

    /// Aggregated geo cards for the market board: mode 'city' groups by city,
    /// mode 'district' drills into districts of the selected city. Latest groups first.
    fn market_geo_summary(&self, user_id: &str, city: &str, district: &str, mode: &str, limit: i64) -> Result<Vec<serde_json::Value>, String>;
    fn get_market_data(&self, id: &str, user_id: &str) -> Result<Option<MarketReferenceData>, String>;
    fn create_market_data(&self, data: &MarketReferenceData) -> Result<(), String>;
    fn create_market_data_bulk(&self, user_id: &str, items: &[serde_json::Value]) -> Result<(i64, Vec<String>), String>;
    fn update_market_data(&self, data: &MarketReferenceData) -> Result<(), String>;
    fn delete_market_data(&self, id: &str, user_id: &str) -> Result<(), String>;
    fn delete_many_market_data(&self, ids: &[String], user_id: &str) -> Result<i64, String>;
    fn delete_market_data_filtered(&self, user_id: &str, city: &str, district: &str, property_type: &str) -> Result<i64, String>;
    fn purge_audit_market_data(&self, user_id: &str) -> Result<i64, String>;

    // ===== Marketing Campaigns =====
    #[allow(clippy::too_many_arguments)]
    fn list_campaigns(
        &self,
        user_id: &str,
        platform: &str,
        property_id: &str,
        period_from: &str,
        period_to: &str,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<MarketingCampaign>, i64), String>;
    fn get_campaign(&self, id: &str, user_id: &str) -> Result<Option<MarketingCampaign>, String>;
    fn create_campaign(&self, campaign: &MarketingCampaign) -> Result<(), String>;
    fn update_campaign(&self, campaign: &MarketingCampaign) -> Result<(), String>;
    fn delete_campaign(&self, id: &str, user_id: &str) -> Result<(), String>;
    fn get_campaign_totals(
        &self,
        user_id: &str,
        platform: &str,
        property_id: &str,
        period_from: &str,
        period_to: &str,
    ) -> Result<serde_json::Value, String>;

    // ===== Office Appointments =====
    fn list_appointments(
        &self,
        user_id: &str,
        from: &str,
        to: &str,
    ) -> Result<Vec<OfficeAppointment>, String>;
    fn get_appointment(&self, id: &str, user_id: &str) -> Result<Option<OfficeAppointment>, String>;
    fn create_appointment(&self, appointment: &OfficeAppointment) -> Result<(), String>;
    fn update_appointment(&self, appointment: &OfficeAppointment) -> Result<(), String>;
    fn delete_appointment(&self, id: &str, user_id: &str) -> Result<(), String>;

    // ===== Broker finance =====
    // limit/offset: قيمة limit <= 0 تعني بدون تحديد (سلوك قديم كامل)
    fn list_finance_operations(
        &self,
        user_id: &str,
        period_from: &str,
        period_to: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<FinanceOperation>, String>;
    fn get_finance_operation(&self, id: &str, user_id: &str) -> Result<Option<FinanceOperation>, String>;
    fn create_finance_operation(&self, item: &FinanceOperation) -> Result<(), String>;
    fn update_finance_operation(&self, item: &FinanceOperation) -> Result<(), String>;
    fn delete_finance_operation(&self, id: &str, user_id: &str) -> Result<(), String>;

    fn list_finance_invoices(
        &self,
        user_id: &str,
        period_from: &str,
        period_to: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<FinanceInvoice>, String>;
    fn get_finance_invoice(&self, id: &str, user_id: &str) -> Result<Option<FinanceInvoice>, String>;
    fn create_finance_invoice(&self, item: &FinanceInvoice) -> Result<(), String>;
    fn update_finance_invoice(&self, item: &FinanceInvoice) -> Result<(), String>;
    fn delete_finance_invoice(&self, id: &str, user_id: &str) -> Result<(), String>;
    fn next_finance_invoice_number(&self, user_id: &str, year: i32) -> Result<String, String>;

    fn list_finance_commissions(
        &self,
        user_id: &str,
        period_from: &str,
        period_to: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<FinanceCommission>, String>;
    fn get_finance_commission(&self, id: &str, user_id: &str) -> Result<Option<FinanceCommission>, String>;
    fn create_finance_commission(&self, item: &FinanceCommission) -> Result<(), String>;
    fn update_finance_commission(&self, item: &FinanceCommission) -> Result<(), String>;
    fn delete_finance_commission(&self, id: &str, user_id: &str) -> Result<(), String>;

    fn list_finance_expenses(
        &self,
        user_id: &str,
        period_from: &str,
        period_to: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<FinanceExpense>, String>;
    fn get_finance_expense(&self, id: &str, user_id: &str) -> Result<Option<FinanceExpense>, String>;
    fn create_finance_expense(&self, item: &FinanceExpense) -> Result<(), String>;
    fn update_finance_expense(&self, item: &FinanceExpense) -> Result<(), String>;
    fn delete_finance_expense(&self, id: &str, user_id: &str) -> Result<(), String>;

    fn list_finance_payments(
        &self,
        user_id: &str,
        period_from: &str,
        period_to: &str,
        target_type: &str,
        target_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<FinancePayment>, String>;
    fn get_finance_payment(&self, id: &str, user_id: &str) -> Result<Option<FinancePayment>, String>;
    fn create_finance_payment(&self, item: &FinancePayment) -> Result<(), String>;
    fn update_finance_payment(&self, item: &FinancePayment) -> Result<(), String>;
    fn delete_finance_payment(&self, id: &str, user_id: &str) -> Result<(), String>;
    fn delete_finance_payments_for_target(
        &self,
        user_id: &str,
        target_type: &str,
        target_id: &str,
    ) -> Result<(), String>;

    // عدّاد إجمالي حقيقي للترقيم الخلفي (مستقل عن حد الصفحة)
    fn count_finance_operations(&self, user_id: &str, period_from: &str, period_to: &str) -> Result<i64, String>;
    fn count_finance_invoices(&self, user_id: &str, period_from: &str, period_to: &str) -> Result<i64, String>;
    fn count_finance_commissions(&self, user_id: &str, period_from: &str, period_to: &str) -> Result<i64, String>;
    fn count_finance_expenses(&self, user_id: &str, period_from: &str, period_to: &str) -> Result<i64, String>;
    fn count_finance_payments(
        &self,
        user_id: &str,
        period_from: &str,
        period_to: &str,
        target_type: &str,
        target_id: &str,
    ) -> Result<i64, String>;
    fn count_brochures(&self, user_id: &str) -> Result<i64, String>;

    // ===== Brochures =====
    fn list_brochures(&self, user_id: &str, limit: i64, offset: i64) -> Result<Vec<serde_json::Value>, String>;
    fn get_brochure(&self, id: &str, user_id: &str) -> Result<Option<serde_json::Value>, String>;
    fn create_brochure(&self, brochure: &Brochure) -> Result<(), String>;
    fn update_brochure(
        &self,
        id: &str,
        user_id: &str,
        property_id: Option<&str>,
        template_id: &str,
        title: &str,
        content: &str,
    ) -> Result<(), String>;
    fn delete_brochure(&self, id: &str, user_id: &str) -> Result<(), String>;

    // ===== Settings =====
    fn get_settings(&self, user_id: &str) -> Result<Option<Settings>, String>;
    fn create_settings(&self, user_id: &str) -> Result<(), String>;
    fn update_settings(&self, settings: &Settings) -> Result<(), String>;

    // ===== Audit Logs =====
    fn create_audit_log(&self, log: &AuditLog) -> Result<(), String>;
    fn list_audit_logs(&self, user_id: &str, limit: i64) -> Result<Vec<AuditLog>, String>;

    // ===== Dashboard =====
    fn get_dashboard_stats(&self, user_id: &str) -> Result<serde_json::Value, String>;
    fn get_recent_property_updates(&self, user_id: &str, limit: i64) -> Result<Vec<serde_json::Value>, String>;

    // ===== Export =====
    fn export_all(&self, user_id: &str) -> Result<serde_json::Value, String>;

    // ===== Atomic multi-step writes (P11) =====
    /// إنشاء عقار + مزامنة رابط المالك + audit في معاملة واحدة.
    fn create_property_atomic(
        &self,
        property: &Property,
        user_id: &str,
        audit: &AuditLog,
    ) -> Result<(), String>;
    /// تعديل عقار + مزامنة رابط المالك + audit في معاملة واحدة.
    fn update_property_atomic(
        &self,
        property: &Property,
        previous_owner: Option<&str>,
        user_id: &str,
        audit: &AuditLog,
    ) -> Result<(), String>;
    /// حذف عقار + audit في معاملة واحدة.
    fn delete_property_atomic(&self, id: &str, audit: &AuditLog) -> Result<(), String>;
    /// تغيير حالة + سجل تحديث في معاملة واحدة.
    fn change_property_status_atomic(
        &self,
        id: &str,
        status: &str,
        update: &PropertyUpdate,
    ) -> Result<(), String>;
    /// إنشاء صورة + تعيين رئيسية اختياريًا في معاملة واحدة.
    fn add_property_image_atomic(
        &self,
        image: &PropertyImage,
        set_as_main_url: Option<&str>,
    ) -> Result<(), String>;
    /// حذف صورة + تحديث/مسح الصورة الرئيسية في معاملة واحدة.
    fn delete_property_image_atomic(
        &self,
        image_id: &str,
        property_id: &str,
        was_main: bool,
        next_main_url: Option<&str>,
    ) -> Result<(), String>;
    /// نسخ عقار + صوره + الرئيسية في معاملة واحدة.
    fn duplicate_property_atomic(
        &self,
        property: &Property,
        images: &[PropertyImage],
        main_image: Option<&str>,
    ) -> Result<(), String>;
    /// إنشاء عميل + أدوار + audit في معاملة واحدة.
    fn create_client_atomic(
        &self,
        client: &Client,
        roles: &[String],
        audit: &AuditLog,
    ) -> Result<(), String>;
    /// تعديل عميل + أدوار اختيارية + audit في معاملة واحدة.
    fn update_client_atomic(
        &self,
        client: &Client,
        roles: Option<&[String]>,
        audit: &AuditLog,
    ) -> Result<(), String>;
    /// حذف عميل + audit في معاملة واحدة.
    fn delete_client_atomic(&self, id: &str, audit: &AuditLog) -> Result<(), String>;
    /// تواصل + تحديث last_contact في معاملة واحدة.
    fn add_communication_atomic(&self, comm: &Communication) -> Result<(), String>;
    /// حذف مدفوعات الهدف ثم الفاتورة في معاملة واحدة.
    fn delete_finance_invoice_atomic(&self, user_id: &str, id: &str) -> Result<(), String>;
    /// حذف مدفوعات الهدف ثم العمولة في معاملة واحدة.
    fn delete_finance_commission_atomic(&self, user_id: &str, id: &str) -> Result<(), String>;
}

// ===== حالة التطبيق المشتركة =====
// تحمل قاعدة البيانات الحالية (SQLite الآن، PostgreSQL لاحقًا)

pub struct AppState {
    pub db: Mutex<Box<dyn Database>>,
}

impl AppState {
    pub fn new(db: Box<dyn Database>) -> Self {
        Self {
            db: Mutex::new(db),
        }
    }
}
