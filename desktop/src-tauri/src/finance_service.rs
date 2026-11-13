//! خدمات المالية والعمولات — ربط المدخلات بالحسابات والحفظ.

use crate::campaigns::{json_nonneg_f64, json_opt_str, optional_iso_date, parse_iso_date};
use crate::db::*;
use crate::finance::{
    commission_amount, is_known_expense_category, is_known_mode, is_known_payment_method,
    is_known_payment_target, net_cash, payment_status, remaining, round_money, share_amount,
    validate_share_percents, LEDGER_KINDS, PaymentStatus,
};
use chrono::Datelike;
use serde_json::{json, Value};
use uuid::Uuid;

pub struct FinanceService<'a> {
    pub db: &'a dyn Database,
}

impl<'a> FinanceService<'a> {
    pub fn new(db: &'a dyn Database) -> Self {
        Self { db }
    }

    fn today() -> String {
        chrono::Local::now().format("%Y-%m-%d").to_string()
    }

    fn require_title(data: &Value, existing: Option<&str>, field: &str) -> Result<String, String> {
        let title = json_opt_str(data, "title")
            .or_else(|| existing.map(|s| s.to_string()))
            .unwrap_or_default();
        let title = title.trim().to_string();
        if title.chars().count() < 2 {
            return Err(format!("{field} مطلوب (حرفان على الأقل)."));
        }
        if title.chars().count() > 200 {
            return Err(format!("{field} أطول من الحد المسموح."));
        }
        Ok(title)
    }

    fn require_date(data: &Value, key: &str, existing: Option<&str>) -> Result<String, String> {
        let raw = json_opt_str(data, key)
            .or_else(|| existing.map(|s| s.to_string()))
            .unwrap_or_default();
        parse_iso_date(&raw)?;
        Ok(raw)
    }

    fn optional_link(
        &self,
        user_id: &str,
        data: &Value,
        key: &str,
        existing: Option<String>,
    ) -> Result<Option<String>, String> {
        let value = if data.get(key).is_some() {
            json_opt_str(data, key)
        } else {
            existing
        };
        match key {
            "client_id" => {
                if let Some(ref id) = value {
                    self.db
                        .get_client(id, user_id)?
                        .ok_or_else(|| "العميل المرتبط غير موجود.".to_string())?;
                }
            }
            "property_id" => {
                if let Some(ref id) = value {
                    self.db
                        .get_property(id, user_id)?
                        .ok_or_else(|| "العقار المرتبط غير موجود.".to_string())?;
                }
            }
            "campaign_id" => {
                if let Some(ref id) = value {
                    self.db
                        .get_campaign(id, user_id)?
                        .ok_or_else(|| "الحملة المرتبطة غير موجودة.".to_string())?;
                }
            }
            "operation_id" => {
                if let Some(ref id) = value {
                    self.db
                        .get_finance_operation(id, user_id)?
                        .ok_or_else(|| "العملية المرتبطة غير موجودة.".to_string())?;
                }
            }
            "invoice_id" => {
                if let Some(ref id) = value {
                    self.db
                        .get_finance_invoice(id, user_id)?
                        .ok_or_else(|| "الفاتورة المرتبطة غير موجودة.".to_string())?;
                }
            }
            _ => {}
        }
        Ok(value)
    }

    fn notes(data: &Value, existing: Option<String>) -> Option<String> {
        if data.get("notes").is_some() {
            json_opt_str(data, "notes")
        } else {
            existing
        }
    }

    fn validate_period(from: &str, to: &str) -> Result<(), String> {
        if !from.is_empty() {
            parse_iso_date(from)?;
        }
        if !to.is_empty() {
            parse_iso_date(to)?;
        }
        if !from.is_empty() && !to.is_empty() && to < from {
            return Err("نهاية الفترة يجب أن تكون بعد بدايتها.".to_string());
        }
        Ok(())
    }

    fn validate_status_filter(status: &str) -> Result<(), String> {
        if status.is_empty() || PaymentStatus::parse(status).is_some() {
            Ok(())
        } else {
            Err("حالة غير معروفة.".to_string())
        }
    }

    fn validate_kind_filter(kind: &str) -> Result<(), String> {
        if kind.is_empty() || LEDGER_KINDS.contains(&kind) {
            Ok(())
        } else {
            Err("نوع السجل غير معروف.".to_string())
        }
    }

    fn json_opt_f64(data: &Value, key: &str) -> Result<Option<f64>, String> {
        match data.get(key) {
            None => Ok(None),
            Some(Value::Null) => Ok(None),
            Some(Value::String(s)) if s.trim().is_empty() => Ok(None),
            Some(Value::Number(n)) => {
                let f = n
                    .as_f64()
                    .ok_or_else(|| format!("قيمة «{key}» غير صالحة."))?;
                if !f.is_finite() {
                    return Err(format!("قيمة «{key}» غير صالحة."));
                }
                Ok(Some(f))
            }
            Some(Value::String(s)) => {
                let f: f64 = s
                    .trim()
                    .parse()
                    .map_err(|_| format!("قيمة «{key}» يجب أن تكون رقمًا."))?;
                if !f.is_finite() {
                    return Err(format!("قيمة «{key}» غير صالحة."));
                }
                Ok(Some(f))
            }
            _ => Err(format!("قيمة «{key}» غير صالحة.")),
        }
    }

    fn positive_amount(data: &Value, key: &str, existing: Option<f64>, allow_zero: bool) -> Result<f64, String> {
        let value = json_nonneg_f64(data, key, existing.unwrap_or(0.0))?;
        if !allow_zero && value <= 0.0 {
            return Err(format!("قيمة «{key}» يجب أن تكون أكبر من صفر."));
        }
        Ok(round_money(value))
    }

    fn enrich_invoice(&self, item: FinanceInvoice) -> Value {
        let today = Self::today();
        let status = payment_status(item.amount, item.paid, item.due_date.as_deref(), &today);
        let mut value = serde_json::to_value(&item).unwrap_or_else(|_| json!({}));
        if let Some(obj) = value.as_object_mut() {
            obj.insert("remaining".into(), json!(remaining(item.amount, item.paid)));
            obj.insert("status".into(), json!(status.as_str()));
            obj.insert("status_label".into(), json!(status.label_ar()));
        }
        value
    }

    fn enrich_commission(&self, item: FinanceCommission) -> Result<Value, String> {
        let total = commission_amount(
            &item.mode,
            item.basis_amount,
            item.rate_percent,
            item.amount_fixed,
        )?;
        let today = Self::today();
        let status = payment_status(total, item.paid, None, &today);
        let mut value = serde_json::to_value(&item).unwrap_or_else(|_| json!({}));
        if let Some(obj) = value.as_object_mut() {
            obj.insert("computed_amount".into(), json!(total));
            obj.insert("office_amount".into(), json!(share_amount(total, item.office_share_pct)));
            obj.insert("broker_amount".into(), json!(share_amount(total, item.broker_share_pct)));
            obj.insert("marketer_amount".into(), json!(share_amount(total, item.marketer_share_pct)));
            obj.insert("partner_amount".into(), json!(share_amount(total, item.partner_share_pct)));
            obj.insert("remaining".into(), json!(remaining(total, item.paid)));
            obj.insert("status".into(), json!(status.as_str()));
            obj.insert("status_label".into(), json!(status.label_ar()));
        }
        Ok(value)
    }

    fn parse_operation(
        &self,
        user_id: &str,
        data: &Value,
        existing: Option<&FinanceOperation>,
    ) -> Result<FinanceOperation, String> {
        let now = chrono::Utc::now().to_rfc3339();
        Ok(FinanceOperation {
            context: crate::country::RecordContext::default(),
            id: existing
                .map(|i| i.id.clone())
                .unwrap_or_else(|| Uuid::new_v4().to_string()),
            user_id: user_id.to_string(),
            title: Self::require_title(data, existing.map(|i| i.title.as_str()), "عنوان العملية")?,
            occurs_on: Self::require_date(data, "occurs_on", existing.map(|i| i.occurs_on.as_str()))?,
            deal_value: json_nonneg_f64(data, "deal_value", existing.map(|i| i.deal_value).unwrap_or(0.0))?,
            client_id: self.optional_link(user_id, data, "client_id", existing.and_then(|i| i.client_id.clone()))?,
            property_id: self.optional_link(user_id, data, "property_id", existing.and_then(|i| i.property_id.clone()))?,
            campaign_id: self.optional_link(user_id, data, "campaign_id", existing.and_then(|i| i.campaign_id.clone()))?,
            notes: Self::notes(data, existing.and_then(|i| i.notes.clone())),
            created_at: existing
                .map(|i| i.created_at.clone())
                .unwrap_or_else(|| now.clone()),
            updated_at: now,
            client_name: None,
            property_name: None,
            campaign_name: None,
        })
    }

    fn parse_invoice(
        &self,
        user_id: &str,
        data: &Value,
        existing: Option<&FinanceInvoice>,
    ) -> Result<FinanceInvoice, String> {
        let now = chrono::Utc::now().to_rfc3339();
        let year = chrono::Local::now().year();
        let number = if let Some(n) = json_opt_str(data, "number") {
            n
        } else if let Some(ex) = existing {
            ex.number.clone()
        } else {
            self.db.next_finance_invoice_number(user_id, year)?
        };
        let due_date = if data.get("due_date").is_some() {
            optional_iso_date(data.get("due_date").and_then(|v| v.as_str()))?
        } else {
            existing.and_then(|i| i.due_date.clone())
        };
        Ok(FinanceInvoice {
            context: crate::country::RecordContext::default(),
            id: existing
                .map(|i| i.id.clone())
                .unwrap_or_else(|| Uuid::new_v4().to_string()),
            user_id: user_id.to_string(),
            number,
            title: Self::require_title(data, existing.map(|i| i.title.as_str()), "عنوان الفاتورة")?,
            issue_date: Self::require_date(data, "issue_date", existing.map(|i| i.issue_date.as_str()))?,
            due_date,
            amount: Self::positive_amount(data, "amount", existing.map(|i| i.amount), false)?,
            client_id: self.optional_link(user_id, data, "client_id", existing.and_then(|i| i.client_id.clone()))?,
            property_id: self.optional_link(user_id, data, "property_id", existing.and_then(|i| i.property_id.clone()))?,
            campaign_id: self.optional_link(user_id, data, "campaign_id", existing.and_then(|i| i.campaign_id.clone()))?,
            operation_id: self.optional_link(user_id, data, "operation_id", existing.and_then(|i| i.operation_id.clone()))?,
            notes: Self::notes(data, existing.and_then(|i| i.notes.clone())),
            created_at: existing
                .map(|i| i.created_at.clone())
                .unwrap_or_else(|| now.clone()),
            updated_at: now,
            client_name: None,
            property_name: None,
            campaign_name: None,
            operation_title: None,
            paid: 0.0,
        })
    }

    fn parse_commission(
        &self,
        user_id: &str,
        data: &Value,
        existing: Option<&FinanceCommission>,
    ) -> Result<FinanceCommission, String> {
        let now = chrono::Utc::now().to_rfc3339();
        let mode = json_opt_str(data, "mode")
            .or_else(|| existing.map(|i| i.mode.clone()))
            .unwrap_or_default();
        if !is_known_mode(&mode) {
            return Err("اختر نوع العمولة: نسبة أو مبلغ ثابت.".to_string());
        }
        let basis = json_nonneg_f64(data, "basis_amount", existing.map(|i| i.basis_amount).unwrap_or(0.0))?;
        let rate = if data.get("rate_percent").is_some() {
            Self::json_opt_f64(data, "rate_percent")?
        } else {
            existing.and_then(|i| i.rate_percent)
        };
        let fixed = if data.get("amount_fixed").is_some() {
            Self::json_opt_f64(data, "amount_fixed")?
        } else {
            existing.and_then(|i| i.amount_fixed)
        };
        let _ = commission_amount(&mode, basis, rate, fixed)?;
        let office = json_nonneg_f64(
            data,
            "office_share_pct",
            existing.map(|i| i.office_share_pct).unwrap_or(100.0),
        )?;
        let broker = json_nonneg_f64(
            data,
            "broker_share_pct",
            existing.map(|i| i.broker_share_pct).unwrap_or(0.0),
        )?;
        let marketer = json_nonneg_f64(
            data,
            "marketer_share_pct",
            existing.map(|i| i.marketer_share_pct).unwrap_or(0.0),
        )?;
        let partner = json_nonneg_f64(
            data,
            "partner_share_pct",
            existing.map(|i| i.partner_share_pct).unwrap_or(0.0),
        )?;
        validate_share_percents(office, broker, marketer, partner)?;
        Ok(FinanceCommission {
            context: crate::country::RecordContext::default(),
            id: existing
                .map(|i| i.id.clone())
                .unwrap_or_else(|| Uuid::new_v4().to_string()),
            user_id: user_id.to_string(),
            title: Self::require_title(data, existing.map(|i| i.title.as_str()), "عنوان العمولة")?,
            occurs_on: Self::require_date(data, "occurs_on", existing.map(|i| i.occurs_on.as_str()))?,
            mode,
            basis_amount: basis,
            rate_percent: rate,
            amount_fixed: fixed,
            office_share_pct: office,
            broker_share_pct: broker,
            marketer_share_pct: marketer,
            partner_share_pct: partner,
            client_id: self.optional_link(user_id, data, "client_id", existing.and_then(|i| i.client_id.clone()))?,
            property_id: self.optional_link(user_id, data, "property_id", existing.and_then(|i| i.property_id.clone()))?,
            campaign_id: self.optional_link(user_id, data, "campaign_id", existing.and_then(|i| i.campaign_id.clone()))?,
            operation_id: self.optional_link(user_id, data, "operation_id", existing.and_then(|i| i.operation_id.clone()))?,
            invoice_id: self.optional_link(user_id, data, "invoice_id", existing.and_then(|i| i.invoice_id.clone()))?,
            notes: Self::notes(data, existing.and_then(|i| i.notes.clone())),
            created_at: existing
                .map(|i| i.created_at.clone())
                .unwrap_or_else(|| now.clone()),
            updated_at: now,
            client_name: None,
            property_name: None,
            campaign_name: None,
            operation_title: None,
            invoice_number: None,
            paid: 0.0,
        })
    }

    fn parse_expense(
        &self,
        user_id: &str,
        data: &Value,
        existing: Option<&FinanceExpense>,
    ) -> Result<FinanceExpense, String> {
        let now = chrono::Utc::now().to_rfc3339();
        let category = if data.get("category").is_some() {
            json_opt_str(data, "category")
        } else {
            existing.and_then(|i| i.category.clone())
        };
        if let Some(ref cat) = category {
            if !is_known_expense_category(cat) {
                return Err("تصنيف المصروف غير معروف.".to_string());
            }
        }
        Ok(FinanceExpense {
            context: crate::country::RecordContext::default(),
            id: existing
                .map(|i| i.id.clone())
                .unwrap_or_else(|| Uuid::new_v4().to_string()),
            user_id: user_id.to_string(),
            title: Self::require_title(data, existing.map(|i| i.title.as_str()), "عنوان المصروف")?,
            occurs_on: Self::require_date(data, "occurs_on", existing.map(|i| i.occurs_on.as_str()))?,
            amount: Self::positive_amount(data, "amount", existing.map(|i| i.amount), false)?,
            category,
            client_id: self.optional_link(user_id, data, "client_id", existing.and_then(|i| i.client_id.clone()))?,
            property_id: self.optional_link(user_id, data, "property_id", existing.and_then(|i| i.property_id.clone()))?,
            campaign_id: self.optional_link(user_id, data, "campaign_id", existing.and_then(|i| i.campaign_id.clone()))?,
            reference: if data.get("reference").is_some() {
                json_opt_str(data, "reference")
            } else {
                existing.and_then(|i| i.reference.clone())
            },
            notes: Self::notes(data, existing.and_then(|i| i.notes.clone())),
            created_at: existing
                .map(|i| i.created_at.clone())
                .unwrap_or_else(|| now.clone()),
            updated_at: now,
            client_name: None,
            property_name: None,
            campaign_name: None,
        })
    }

    fn parse_payment(
        &self,
        user_id: &str,
        data: &Value,
        existing: Option<&FinancePayment>,
    ) -> Result<FinancePayment, String> {
        let now = chrono::Utc::now().to_rfc3339();
        let target_type = json_opt_str(data, "target_type")
            .or_else(|| existing.map(|i| i.target_type.clone()))
            .unwrap_or_default();
        if !is_known_payment_target(&target_type) {
            return Err("الدفعة يجب أن ترتبط بفاتورة أو عمولة.".to_string());
        }
        let target_id = json_opt_str(data, "target_id")
            .or_else(|| existing.map(|i| i.target_id.clone()))
            .unwrap_or_default();
        if target_id.is_empty() {
            return Err("اختر الفاتورة أو العمولة المرتبطة بالدفعة.".to_string());
        }
        let context = existing.map(|r| r.context.clone());
        let target_country = match target_type.as_str() {
            "invoice" => self.db.get_finance_invoice(&target_id, user_id)?.ok_or("الفاتورة المرتبطة غير موجودة")?.context.country,
            "commission" => self.db.get_finance_commission(&target_id, user_id)?.ok_or("العمولة المرتبطة غير موجودة")?.context.country,
            _ => return Err("نوع السجل المرتبط غير صالح".into()),
        };
        if context.is_some_and(|c| target_country != c.country) { return Err("لا يمكن نقل دفعة محفوظة إلى سجل بعملة مختلفة".into()); }
        let method = if data.get("method").is_some() {
            json_opt_str(data, "method")
        } else {
            existing.and_then(|i| i.method.clone())
        };
        if let Some(ref m) = method {
            if !is_known_payment_method(m) {
                return Err("طريقة الدفع غير معروفة.".to_string());
            }
        }
        Ok(FinancePayment {
            context: crate::country::RecordContext::default(),
            id: existing
                .map(|i| i.id.clone())
                .unwrap_or_else(|| Uuid::new_v4().to_string()),
            user_id: user_id.to_string(),
            occurs_on: Self::require_date(data, "occurs_on", existing.map(|i| i.occurs_on.as_str()))?,
            amount: Self::positive_amount(data, "amount", existing.map(|i| i.amount), false)?,
            target_type,
            target_id,
            method,
            reference: if data.get("reference").is_some() {
                json_opt_str(data, "reference")
            } else {
                existing.and_then(|i| i.reference.clone())
            },
            notes: Self::notes(data, existing.and_then(|i| i.notes.clone())),
            created_at: existing
                .map(|i| i.created_at.clone())
                .unwrap_or_else(|| now.clone()),
            updated_at: now,
            target_title: None,
        })
    }

    pub fn list_operations(&self, user_id: &str, from: &str, to: &str, limit: i64, offset: i64) -> Result<Value, String> {
        Self::validate_period(from, to)?;
        let items = self.db.list_finance_operations(user_id, from, to, limit, offset)?;
        let total = if limit > 0 {
            self.db.count_finance_operations(user_id, from, to)?
        } else {
            items.len() as i64
        };
        Ok(json!({ "items": items, "total": total }))
    }

    pub fn get_operation(&self, id: &str, user_id: &str) -> Result<Value, String> {
        let item = self
            .db
            .get_finance_operation(id, user_id)?
            .ok_or_else(|| "العملية غير موجودة".to_string())?;
        serde_json::to_value(item).map_err(|e| e.to_string())
    }

    pub fn create_operation(&self, user_id: &str, data: &Value) -> Result<Value, String> {
        let item = self.parse_operation(user_id, data, None)?;
        self.db.create_finance_operation(&item)?;
        self.get_operation(&item.id, user_id)
    }

    pub fn update_operation(&self, id: &str, user_id: &str, data: &Value) -> Result<Value, String> {
        let existing = self
            .db
            .get_finance_operation(id, user_id)?
            .ok_or_else(|| "العملية غير موجودة".to_string())?;
        let item = self.parse_operation(user_id, data, Some(&existing))?;
        self.db.update_finance_operation(&item)?;
        self.get_operation(id, user_id)
    }

    pub fn delete_operation(&self, id: &str, user_id: &str) -> Result<Value, String> {
        self.db.delete_finance_operation(id, user_id)?;
        Ok(json!({ "message": "تم حذف العملية" }))
    }

    pub fn list_invoices(&self, user_id: &str, from: &str, to: &str, status: &str, limit: i64, offset: i64) -> Result<Value, String> {
        Self::validate_period(from, to)?;
        Self::validate_status_filter(status)?;
        // الحالة تُشتق بعد الجلب (من المبالغ المدفوعة) — لذا مع فلتر حالة نجلب كامل السجلات
        // للحفاظ على تطابق total مع العناصر؛ الحد يُطبق فقط دون فلتر حالة.
        let eff_limit = if status.is_empty() { limit } else { 0 };
        let items: Vec<Value> = self
            .db
            .list_finance_invoices(user_id, from, to, eff_limit, offset)?
            .into_iter()
            .map(|i| self.enrich_invoice(i))
            .filter(|v| status.is_empty() || v.get("status").and_then(|s| s.as_str()) == Some(status))
            .collect();
        let total = if eff_limit > 0 {
            self.db.count_finance_invoices(user_id, from, to)?
        } else {
            items.len() as i64
        };
        Ok(json!({ "items": items, "total": total }))
    }

    pub fn get_invoice(&self, id: &str, user_id: &str) -> Result<Value, String> {
        let item = self
            .db
            .get_finance_invoice(id, user_id)?
            .ok_or_else(|| "الفاتورة غير موجودة".to_string())?;
        Ok(self.enrich_invoice(item))
    }

    pub fn create_invoice(&self, user_id: &str, data: &Value) -> Result<Value, String> {
        let item = self.parse_invoice(user_id, data, None)?;
        self.db.create_finance_invoice(&item)?;
        self.get_invoice(&item.id, user_id)
    }

    pub fn update_invoice(&self, id: &str, user_id: &str, data: &Value) -> Result<Value, String> {
        let existing = self
            .db
            .get_finance_invoice(id, user_id)?
            .ok_or_else(|| "الفاتورة غير موجودة".to_string())?;
        let item = self.parse_invoice(user_id, data, Some(&existing))?;
        self.db.update_finance_invoice(&item)?;
        self.get_invoice(id, user_id)
    }

    pub fn delete_invoice(&self, id: &str, user_id: &str) -> Result<Value, String> {
        self.db.delete_finance_invoice_atomic(user_id, id)?;
        Ok(json!({ "message": "تم حذف الفاتورة" }))
    }

    pub fn list_commissions(&self, user_id: &str, from: &str, to: &str, status: &str, limit: i64, offset: i64) -> Result<Value, String> {
        Self::validate_period(from, to)?;
        Self::validate_status_filter(status)?;
        // الحالة تُشتق بعد الجلب — نفس قاعدة الفواتير
        let eff_limit = if status.is_empty() { limit } else { 0 };
        let mut items = Vec::new();
        for item in self.db.list_finance_commissions(user_id, from, to, eff_limit, offset)? {
            let value = self.enrich_commission(item)?;
            if status.is_empty() || value.get("status").and_then(|s| s.as_str()) == Some(status) {
                items.push(value);
            }
        }
        let total = if eff_limit > 0 {
            self.db.count_finance_commissions(user_id, from, to)?
        } else {
            items.len() as i64
        };
        Ok(json!({ "items": items, "total": total }))
    }

    pub fn get_commission(&self, id: &str, user_id: &str) -> Result<Value, String> {
        let item = self
            .db
            .get_finance_commission(id, user_id)?
            .ok_or_else(|| "العمولة غير موجودة".to_string())?;
        self.enrich_commission(item)
    }

    pub fn create_commission(&self, user_id: &str, data: &Value) -> Result<Value, String> {
        let item = self.parse_commission(user_id, data, None)?;
        self.db.create_finance_commission(&item)?;
        self.get_commission(&item.id, user_id)
    }

    pub fn update_commission(&self, id: &str, user_id: &str, data: &Value) -> Result<Value, String> {
        let existing = self
            .db
            .get_finance_commission(id, user_id)?
            .ok_or_else(|| "العمولة غير موجودة".to_string())?;
        let item = self.parse_commission(user_id, data, Some(&existing))?;
        self.db.update_finance_commission(&item)?;
        self.get_commission(id, user_id)
    }

    pub fn delete_commission(&self, id: &str, user_id: &str) -> Result<Value, String> {
        self.db.delete_finance_commission_atomic(user_id, id)?;
        Ok(json!({ "message": "تم حذف العمولة" }))
    }

    pub fn list_expenses(&self, user_id: &str, from: &str, to: &str, limit: i64, offset: i64) -> Result<Value, String> {
        Self::validate_period(from, to)?;
        let items = self.db.list_finance_expenses(user_id, from, to, limit, offset)?;
        let total = if limit > 0 {
            self.db.count_finance_expenses(user_id, from, to)?
        } else {
            items.len() as i64
        };
        Ok(json!({ "items": items, "total": total }))
    }

    pub fn get_expense(&self, id: &str, user_id: &str) -> Result<Value, String> {
        let item = self
            .db
            .get_finance_expense(id, user_id)?
            .ok_or_else(|| "المصروف غير موجود".to_string())?;
        serde_json::to_value(item).map_err(|e| e.to_string())
    }

    pub fn create_expense(&self, user_id: &str, data: &Value) -> Result<Value, String> {
        let item = self.parse_expense(user_id, data, None)?;
        self.db.create_finance_expense(&item)?;
        self.get_expense(&item.id, user_id)
    }

    pub fn update_expense(&self, id: &str, user_id: &str, data: &Value) -> Result<Value, String> {
        let existing = self
            .db
            .get_finance_expense(id, user_id)?
            .ok_or_else(|| "المصروف غير موجود".to_string())?;
        let item = self.parse_expense(user_id, data, Some(&existing))?;
        self.db.update_finance_expense(&item)?;
        self.get_expense(id, user_id)
    }

    pub fn delete_expense(&self, id: &str, user_id: &str) -> Result<Value, String> {
        self.db.delete_finance_expense(id, user_id)?;
        Ok(json!({ "message": "تم حذف المصروف" }))
    }

    pub fn list_payments(
        &self,
        user_id: &str,
        from: &str,
        to: &str,
        target_type: &str,
        target_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Value, String> {
        Self::validate_period(from, to)?;
        let items = self
            .db
            .list_finance_payments(user_id, from, to, target_type, target_id, limit, offset)?;
        let total = if limit > 0 {
            self.db.count_finance_payments(user_id, from, to, target_type, target_id)?
        } else {
            items.len() as i64
        };
        Ok(json!({ "items": items, "total": total }))
    }

    pub fn get_payment(&self, id: &str, user_id: &str) -> Result<Value, String> {
        let item = self
            .db
            .get_finance_payment(id, user_id)?
            .ok_or_else(|| "الدفعة غير موجودة".to_string())?;
        serde_json::to_value(item).map_err(|e| e.to_string())
    }

    pub fn create_payment(&self, user_id: &str, data: &Value) -> Result<Value, String> {
        let item = self.parse_payment(user_id, data, None)?;
        self.db.create_finance_payment(&item)?;
        self.get_payment(&item.id, user_id)
    }

    pub fn update_payment(&self, id: &str, user_id: &str, data: &Value) -> Result<Value, String> {
        let existing = self
            .db
            .get_finance_payment(id, user_id)?
            .ok_or_else(|| "الدفعة غير موجودة".to_string())?;
        let item = self.parse_payment(user_id, data, Some(&existing))?;
        self.db.update_finance_payment(&item)?;
        self.get_payment(id, user_id)
    }

    pub fn delete_payment(&self, id: &str, user_id: &str) -> Result<Value, String> {
        self.db.delete_finance_payment(id, user_id)?;
        Ok(json!({ "message": "تم حذف الدفعة" }))
    }

    pub fn receivables(&self, user_id: &str, from: &str, to: &str, status: &str) -> Result<Value, String> {
        let invoices = self.list_invoices(user_id, from, to, status, 0, 0)?;
        let commissions = self.list_commissions(user_id, from, to, status, 0, 0)?;
        let inv_items = invoices
            .get("items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|v| v.get("status").and_then(|s| s.as_str()) != Some("paid"))
            .map(|mut v| {
                if let Some(obj) = v.as_object_mut() {
                    obj.insert("kind".into(), json!("invoice"));
                    obj.insert("kind_label".into(), json!("فاتورة"));
                }
                v
            });
        let com_items = commissions
            .get("items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|v| v.get("status").and_then(|s| s.as_str()) != Some("paid"))
            .map(|mut v| {
                if let Some(obj) = v.as_object_mut() {
                    obj.insert("kind".into(), json!("commission"));
                    obj.insert("kind_label".into(), json!("عمولة"));
                    if obj.get("amount").is_none() {
                        obj.insert("amount".into(), obj.get("computed_amount").cloned().unwrap_or(json!(0)));
                    }
                }
                v
            });
        let items: Vec<Value> = inv_items.chain(com_items).collect();
        let total = items.len();
        Ok(json!({ "items": items, "total": total }))
    }

    pub fn summary(&self, user_id: &str, from: &str, to: &str) -> Result<Value, String> {
        Self::validate_period(from, to)?;
        let context = crate::country::RecordContext::from_settings(&self.db.get_settings(user_id)?)?;
        let invoices = self.db.list_finance_invoices(user_id, from, to, 0, 0)?.into_iter().filter(|r| r.context.country == context.country).collect::<Vec<_>>();
        let commissions = self.db.list_finance_commissions(user_id, from, to, 0, 0)?.into_iter().filter(|r| r.context.country == context.country).collect::<Vec<_>>();
        let expenses = self.db.list_finance_expenses(user_id, from, to, 0, 0)?.into_iter().filter(|r| r.context.country == context.country).collect::<Vec<_>>();
        let payments = self.db.list_finance_payments(user_id, from, to, "", "", 0, 0)?.into_iter().filter(|r| r.context.country == context.country).collect::<Vec<_>>();

        let invoice_total = round_money(invoices.iter().map(|i| i.amount).sum());
        let invoice_collected = round_money(invoices.iter().map(|i| i.paid).sum());
        let mut commission_total = 0.0;
        let mut commission_collected = 0.0;
        for item in &commissions {
            let total = commission_amount(&item.mode, item.basis_amount, item.rate_percent, item.amount_fixed)?;
            commission_total += total;
            commission_collected += item.paid;
        }
        commission_total = round_money(commission_total);
        commission_collected = round_money(commission_collected);
        let expense_total = round_money(expenses.iter().map(|e| e.amount).sum());
        let payment_total = round_money(payments.iter().map(|p| p.amount).sum());
        let collections = round_money(invoice_collected + commission_collected);
        let receivables = remaining(invoice_total, invoice_collected) + remaining(commission_total, commission_collected);
        Ok(json!({
            "country": context.country, "currency": context.currency,
            "invoice_count": invoices.len(),
            "commission_count": commissions.len(),
            "expense_count": expenses.len(),
            "payment_count": payments.len(),
            "invoice_total": invoice_total,
            "invoice_collected": invoice_collected,
            "commission_total": commission_total,
            "commission_collected": commission_collected,
            "expense_total": expense_total,
            "payment_total": payment_total,
            "collections": collections,
            "receivables": round_money(receivables),
            "net_cash": net_cash(collections, expense_total),
            "training_data_present": self.db.has_training_data()?,
        }))
    }

    pub fn ledger(
        &self,
        user_id: &str,
        from: &str,
        to: &str,
        kind: &str,
        status: &str,
    ) -> Result<Value, String> {
        Self::validate_period(from, to)?;
        Self::validate_kind_filter(kind)?;
        Self::validate_status_filter(status)?;
        let mut items: Vec<Value> = Vec::new();
        if kind.is_empty() || kind == "invoice" {
            for item in self.db.list_finance_invoices(user_id, from, to, 0, 0)? {
                let v = self.enrich_invoice(item.clone());
                if status.is_empty() || v.get("status").and_then(|s| s.as_str()) == Some(status) {
                    items.push(json!({
                        "id": item.id, "country": item.context.country, "currency": item.context.currency,
                        "kind": "invoice",
                        "kind_label": "فاتورة",
                        "occurs_on": item.issue_date,
                        "title": format!("{} — {}", item.number, item.title),
                        "amount": item.amount,
                        "status": v.get("status"),
                        "status_label": v.get("status_label"),
                    }));
                }
            }
        }
        if kind.is_empty() || kind == "commission" {
            for item in self.db.list_finance_commissions(user_id, from, to, 0, 0)? {
                let v = self.enrich_commission(item.clone())?;
                if status.is_empty() || v.get("status").and_then(|s| s.as_str()) == Some(status) {
                    items.push(json!({
                        "id": item.id, "country": item.context.country, "currency": item.context.currency,
                        "kind": "commission",
                        "kind_label": "عمولة",
                        "occurs_on": item.occurs_on,
                        "title": item.title,
                        "amount": v.get("computed_amount"),
                        "status": v.get("status"),
                        "status_label": v.get("status_label"),
                    }));
                }
            }
        }
        if (kind.is_empty() || kind == "expense") && status.is_empty() {
            for item in self.db.list_finance_expenses(user_id, from, to, 0, 0)? {
                items.push(json!({
                    "id": item.id, "country": item.context.country, "currency": item.context.currency,
                    "kind": "expense",
                    "kind_label": "مصروف",
                    "occurs_on": item.occurs_on,
                    "title": item.title,
                    "amount": item.amount,
                    "status": "paid",
                    "status_label": "مسجّل",
                }));
            }
        }
        if (kind.is_empty() || kind == "payment") && status.is_empty() {
            for item in self.db.list_finance_payments(user_id, from, to, "", "", 0, 0)? {
                items.push(json!({
                    "id": item.id, "country": item.context.country, "currency": item.context.currency,
                    "kind": "payment",
                    "kind_label": "دفعة",
                    "occurs_on": item.occurs_on,
                    "title": item.target_title.clone().unwrap_or_else(|| "دفعة".to_string()),
                    "amount": item.amount,
                    "status": "paid",
                    "status_label": "مسجّل",
                }));
            }
        }
        if (kind.is_empty() || kind == "operation") && status.is_empty() {
            for item in self.db.list_finance_operations(user_id, from, to, 0, 0)? {
                items.push(json!({
                    "id": item.id, "country": item.context.country, "currency": item.context.currency,
                    "kind": "operation",
                    "kind_label": "عملية",
                    "occurs_on": item.occurs_on,
                    "title": item.title,
                    "amount": item.deal_value,
                    "status": "",
                    "status_label": "—",
                }));
            }
        }
        items.sort_by(|a, b| {
            let da = a.get("occurs_on").and_then(|v| v.as_str()).unwrap_or("");
            let db = b.get("occurs_on").and_then(|v| v.as_str()).unwrap_or("");
            db.cmp(da)
        });
        let total = items.len();
        Ok(json!({ "items": items, "total": total }))
    }

    pub fn invoice_pdf_payload(&self, id: &str, user_id: &str) -> Result<(Value, Vec<Value>, Option<Settings>), String> {
        let invoice = self.get_invoice(id, user_id)?;
        let payments = self
            .db
            .list_finance_payments(user_id, "", "", "invoice", id, 0, 0)?;
        let pay_json: Vec<Value> = payments
            .into_iter()
            .map(|p| serde_json::to_value(p).unwrap_or(json!({})))
            .collect();
        let settings = self.db.get_settings(user_id)?;
        let settings = crate::country::record_context(&invoice)?.print_settings(&settings);
        Ok((invoice, pay_json, settings))
    }

    pub fn record_pdf_payload(&self, kind: &str, id: &str, user_id: &str) -> Result<(Value, Option<Settings>), String> {
        let mut record = match kind {
            "operation" => self.get_operation(id, user_id)?,
            "commission" => self.get_commission(id, user_id)?,
            "expense" => self.get_expense(id, user_id)?,
            "payment" => self.get_payment(id, user_id)?,
            "receivable_invoice" => self.get_invoice(id, user_id)?,
            "receivable_commission" => self.get_commission(id, user_id)?,
            _ => return Err("نوع سجل الطباعة غير معروف.".to_string()),
        };
        if kind.starts_with("receivable_") {
            if let Some(obj) = record.as_object_mut() {
                let source = kind.trim_start_matches("receivable_");
                obj.insert("kind_label".into(), json!(if source == "invoice" { "فاتورة" } else { "عمولة" }));
                if obj.get("amount").is_none() {
                    obj.insert("amount".into(), obj.get("computed_amount").cloned().unwrap_or(Value::Null));
                }
            }
        }
        let settings = crate::country::record_context(&record)?.print_settings(&self.db.get_settings(user_id)?);
        Ok((record, settings))
    }

    pub fn report_pdf_payload(
        &self,
        user_id: &str,
        from: &str,
        to: &str,
    ) -> Result<(Value, Option<Settings>), String> {
        let summary = self.summary(user_id, from, to)?;
        let settings = self.db.get_settings(user_id)?;
        Ok((summary, settings))
    }
}
