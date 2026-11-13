//! Country catalog shared with React. Record context is captured by SQLite at insertion.
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use crate::db::Settings;

pub fn catalog() -> Value {
    serde_json::from_str(include_str!("../../src/domain/countryProfiles.json")).expect("validated country catalog")
}
pub fn profile(code: &str) -> Result<Value, String> {
    catalog()["countries"].as_array().unwrap().iter().find(|p| p["code"] == code)
        .cloned().ok_or_else(|| "الدولة غير مدعومة".into())
}
pub fn currency(code: &str) -> Result<String, String> {
    Ok(profile(code)?["currency"].as_str().unwrap().into())
}
pub fn service(code: &str, license_type: &str) -> Result<Value, String> {
    let p = profile(code)?;
    if let Some(l) = p["licenses"].as_array().unwrap().iter().find(|l| l["id"] == license_type) {
        if let Some(url) = l.get("serviceUrl") {
            return Ok(json!({"label":"الخدمات الرسمية للوساطة العقارية", "url":url}));
        }
    }
    Ok(p["service"].clone())
}
pub fn license_label(code: &str, kind: &str, manual: &str) -> String {
    profile(code).ok().and_then(|p| p["licenses"].as_array()?.iter()
        .find(|l| l["id"] == kind).and_then(|l| l["name"].as_str()).map(str::to_owned))
        .unwrap_or_else(|| manual.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecordContext {
    pub country: String,
    pub currency: String,
    pub broker_license_type: Option<String>,
    pub broker_license_name: Option<String>,
    pub broker_license_number: Option<String>,
}
impl RecordContext {
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self { country: row.get("country")?, currency: row.get("currency")?,
            broker_license_type: row.get("broker_license_type")?, broker_license_name: row.get("broker_license_name")?,
            broker_license_number: row.get("broker_license_number")? })
    }
    pub fn from_settings(settings: &Option<Settings>) -> Result<Self, String> {
        let country = settings.as_ref().map(|s| s.country.as_str()).unwrap_or("SA");
        Ok(Self { country: country.into(), currency: currency(country)?,
            broker_license_type: settings.as_ref().and_then(|s| s.fal_license_type.clone()),
            broker_license_name: settings.as_ref().and_then(|s| s.broker_license_name.clone()),
            broker_license_number: settings.as_ref().and_then(|s| s.fal_license_number.clone()) })
    }
    pub fn print_settings(&self, settings: &Option<Settings>) -> Option<Settings> {
        settings.clone().map(|mut s| { s.country = self.country.clone(); s.display_currency = Some(self.currency.clone());
            s.fal_license_type = self.broker_license_type.clone(); s.fal_license_number = self.broker_license_number.clone();
            s.broker_license_name = self.broker_license_name.clone(); s })
    }
}
pub fn record_context(value: &Value) -> Result<RecordContext, String> {
    let ctx: RecordContext = serde_json::from_value(value.clone()).map_err(|_| "الدولة والعملة غير محددتين لهذا السجل")?;
    if currency(&ctx.country)? != ctx.currency { return Err("عملة السجل لا تطابق دولته".into()); }
    Ok(ctx)
}

pub const CONTEXT_COLS: &str = "country, currency, broker_license_type, broker_license_name, broker_license_number";
pub const RECORD_TABLES: &[&str] = &["properties", "clients", "market_reference_data", "marketing_campaigns", "brochures",
    "finance_operations", "finance_invoices", "finance_commissions", "finance_expenses", "finance_payments", "contracts"];


fn monetary_links(table: &str) -> Vec<(&'static str, &'static str)> {
    match table {
            "brochures" | "marketing_campaigns" => vec![("properties", "property_id")],
            "finance_operations" | "finance_expenses" => vec![("properties", "property_id"), ("marketing_campaigns", "campaign_id")],
            "finance_invoices" => vec![("finance_operations", "operation_id"), ("properties", "property_id"), ("marketing_campaigns", "campaign_id")],
            "finance_commissions" => vec![("finance_invoices", "invoice_id"), ("finance_operations", "operation_id"), ("properties", "property_id"), ("marketing_campaigns", "campaign_id")],
            "contracts" => vec![("finance_invoices", "invoice_id"), ("finance_commissions", "commission_id"), ("properties", "property_id"), ("marketing_campaigns", "campaign_id")],
            _ => vec![],
        }
}

pub fn new_context(conn: &Connection, table: &str, user_id: &str, data: &Value, settings: &Option<Settings>) -> Result<RecordContext, String> {
    let mut inherited: Option<RecordContext> = None;
    for (parent, key) in monetary_links(table) {
        if let Some(id) = data.get(key).and_then(Value::as_str).filter(|v| !v.is_empty()) {
            let context = conn.query_row(&format!("SELECT {CONTEXT_COLS} FROM {parent} WHERE id=?1 AND user_id=?2"), rusqlite::params![id,user_id], RecordContext::from_row).map_err(|_| "السجل المرتبط غير موجود")?;
            if inherited.as_ref().is_some_and(|c| c.country != context.country) { return Err("السجلات المرتبطة تنتمي إلى دول مختلفة".into()); }
            if inherited.is_none() { inherited = Some(context); }
        }
    }
    match inherited { Some(c) => Ok(c), None => RecordContext::from_settings(settings) }
}

pub fn migrate(conn: &Connection) -> Result<(), String> {
    let countries = catalog()["countries"].as_array().unwrap().clone();
    let cases = countries.iter().map(|p| format!("WHEN '{}' THEN '{}'", p["code"].as_str().unwrap(), p["currency"].as_str().unwrap())).collect::<Vec<_>>().join(" ");
    let reverse = countries.iter().map(|p| format!("WHEN '{}' THEN '{}'", p["currency"].as_str().unwrap(), p["code"].as_str().unwrap())).collect::<Vec<_>>().join(" ");
    conn.execute_batch(&format!("ALTER TABLE settings ADD COLUMN country TEXT NOT NULL DEFAULT 'SA';
        ALTER TABLE settings ADD COLUMN broker_license_name TEXT;
        UPDATE settings SET country = CASE display_currency {reverse} ELSE 'SA' END;
        UPDATE settings SET display_currency = CASE country {cases} END;
        CREATE TRIGGER settings_country_guard BEFORE UPDATE ON settings
        WHEN (CASE NEW.country {cases} ELSE '' END) != COALESCE(NEW.display_currency,'')
        BEGIN SELECT RAISE(ABORT, 'عملة الإعدادات يجب أن تتبع الدولة'); END;" )).map_err(|e| e.to_string())?;
    for table in RECORD_TABLES {
        // Monetary descendants inherit the original record's context, as requested by the owner.
        // A client/contact is not a monetary source: it can participate in several countries.
        let links = monetary_links(table);
        let source_query = |column: &str| {
            let mut selects: Vec<String> = links.iter().map(|(parent, key)| format!("SELECT {column} AS value FROM {parent} WHERE id=NEW.{key} AND user_id=NEW.user_id")).collect();
            if *table == "finance_payments" {
                selects.push(format!("SELECT {column} AS value FROM finance_invoices WHERE NEW.target_type='invoice' AND id=NEW.target_id AND user_id=NEW.user_id"));
                selects.push(format!("SELECT {column} AS value FROM finance_commissions WHERE NEW.target_type='commission' AND id=NEW.target_id AND user_id=NEW.user_id"));
            }
            if selects.is_empty() { "SELECT NULL AS value WHERE 0".into() } else { selects.join(" UNION ALL ") }
        };
        let source_country = source_query("country");
        let selected_country = format!("COALESCE((SELECT value FROM ({source_country}) LIMIT 1),(SELECT country FROM settings WHERE user_id=NEW.user_id),'SA')");
        let snapshot = |column: &str, setting: &str| format!("CASE WHEN EXISTS({source_country}) THEN (SELECT value FROM ({}) LIMIT 1) ELSE (SELECT {setting} FROM settings WHERE user_id=NEW.user_id) END", source_query(column));
        let snapshot_type = snapshot("broker_license_type", "fal_license_type");
        let snapshot_name = snapshot("broker_license_name", "broker_license_name");
        let snapshot_number = snapshot("broker_license_number", "fal_license_number");
        // Existing records are SA/SAR by the owner's explicit migration decision.
        conn.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN country TEXT NOT NULL DEFAULT '';
            ALTER TABLE {table} ADD COLUMN currency TEXT NOT NULL DEFAULT '';
            ALTER TABLE {table} ADD COLUMN broker_license_type TEXT;
            ALTER TABLE {table} ADD COLUMN broker_license_name TEXT;
            ALTER TABLE {table} ADD COLUMN broker_license_number TEXT;
            UPDATE {table} SET country='SA', currency='SAR',
                broker_license_type=(SELECT fal_license_type FROM settings WHERE user_id={table}.user_id),
                broker_license_number=(SELECT fal_license_number FROM settings WHERE user_id={table}.user_id);
            CREATE TRIGGER {table}_country_capture AFTER INSERT ON {table} WHEN NEW.country=''
            BEGIN UPDATE {table} SET country={selected_country},
                currency=CASE {selected_country} {cases} END,
                broker_license_type={snapshot_type},
                broker_license_name={snapshot_name},
                broker_license_number={snapshot_number} WHERE id=NEW.id; END;
            CREATE TRIGGER {table}_linked_country_insert BEFORE INSERT ON {table}
            WHEN (SELECT COUNT(DISTINCT value) FROM ({source_country}))>1 OR (NEW.country!='' AND EXISTS(SELECT 1 FROM ({source_country}) WHERE value!=NEW.country))
            BEGIN SELECT RAISE(ABORT,'السجلات المرتبطة تنتمي إلى دول مختلفة؛ لا يمكن دمج عملاتها'); END;
            CREATE TRIGGER {table}_linked_country_update BEFORE UPDATE ON {table}
            WHEN NEW.country!='' AND EXISTS(SELECT 1 FROM ({source_country}) WHERE value!=NEW.country)
            BEGIN SELECT RAISE(ABORT,'لا يمكن ربط السجل المحفوظ بسجل بعملة مختلفة'); END;
            CREATE TRIGGER {table}_country_immutable BEFORE UPDATE ON {table}
            WHEN OLD.country!='' AND (NEW.country!=OLD.country OR NEW.currency!=OLD.currency)
            BEGIN SELECT RAISE(ABORT,'لا يمكن تغيير دولة أو عملة سجل محفوظ'); END;
            CREATE TRIGGER {table}_country_valid BEFORE UPDATE ON {table}
            WHEN NEW.country!='' AND (CASE NEW.country {cases} ELSE '' END)!=NEW.currency
            BEGIN SELECT RAISE(ABORT,'عملة السجل لا تطابق دولته'); END;
            CREATE TRIGGER {table}_country_insert_valid BEFORE INSERT ON {table}
            WHEN NEW.country!='' AND (CASE NEW.country {cases} ELSE 'INVALID' END)!=NEW.currency
            BEGIN SELECT RAISE(ABORT,'عملة السجل لا تطابق دولته'); END;" )).map_err(|e|e.to_string())?;
    }
    Ok(())
}
