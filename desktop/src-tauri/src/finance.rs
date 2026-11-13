//! حسابات المالية والعمولات للوسيط العقاري.
//! المقام أو المدخل غير الصالح يعني أن الناتج غير قابل للحساب، وليس رقمًا مخترعًا.

use serde::{Deserialize, Serialize};

pub const MONEY_EPS: f64 = 0.005;
pub const SHARE_EPS: f64 = 0.01;

pub const COMMISSION_MODES: &[&str] = &["percent", "fixed"];
pub const EXPENSE_CATEGORIES: &[&str] = &["تسويق", "انتقالات", "رسوم", "صيانة", "أخرى"];
pub const PAYMENT_METHODS: &[&str] = &["تحويل", "نقد", "شبكة", "شيك", "أخرى"];
pub const PAYMENT_TARGETS: &[&str] = &["invoice", "commission"];
pub const LEDGER_KINDS: &[&str] = &["invoice", "commission", "expense", "payment", "operation"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentStatus {
    Due,
    Partial,
    Paid,
    Overdue,
}

impl PaymentStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            PaymentStatus::Due => "due",
            PaymentStatus::Partial => "partial",
            PaymentStatus::Paid => "paid",
            PaymentStatus::Overdue => "overdue",
        }
    }

    pub fn label_ar(self) -> &'static str {
        match self {
            PaymentStatus::Due => "مستحق",
            PaymentStatus::Partial => "مدفوع جزئيًا",
            PaymentStatus::Paid => "مدفوع",
            PaymentStatus::Overdue => "متأخر",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "due" => Some(Self::Due),
            "partial" => Some(Self::Partial),
            "paid" => Some(Self::Paid),
            "overdue" => Some(Self::Overdue),
            _ => None,
        }
    }
}

pub fn round_money(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

pub fn remaining(amount: f64, paid: f64) -> f64 {
    round_money((amount - paid).max(0.0))
}

/// الحالة: مدفوع إن وُفّي المبلغ، وإلا متأخر إن تجاوز تاريخ الاستحقاق، وإلا جزئي إن وُجدت دفعة، وإلا مستحق.
pub fn payment_status(amount: f64, paid: f64, due_date: Option<&str>, today: &str) -> PaymentStatus {
    if paid + MONEY_EPS >= amount {
        return PaymentStatus::Paid;
    }
    if let Some(due) = due_date.map(str::trim).filter(|s| !s.is_empty()) {
        if due < today {
            return PaymentStatus::Overdue;
        }
    }
    if paid > MONEY_EPS {
        PaymentStatus::Partial
    } else {
        PaymentStatus::Due
    }
}

pub fn commission_amount(
    mode: &str,
    basis_amount: f64,
    rate_percent: Option<f64>,
    amount_fixed: Option<f64>,
) -> Result<f64, String> {
    match mode.trim() {
        "percent" => {
            let rate = rate_percent.ok_or_else(|| "نسبة العمولة مطلوبة.".to_string())?;
            if !rate.is_finite() || rate < 0.0 || rate > 100.0 {
                return Err("نسبة العمولة يجب أن تكون بين 0 و 100.".to_string());
            }
            if !basis_amount.is_finite() || basis_amount < 0.0 {
                return Err("مبلغ أساس العمولة غير صالح.".to_string());
            }
            Ok(round_money(basis_amount * rate / 100.0))
        }
        "fixed" => {
            let amount = amount_fixed.ok_or_else(|| "مبلغ العمولة مطلوب.".to_string())?;
            if !amount.is_finite() || amount < 0.0 {
                return Err("مبلغ العمولة غير صالح.".to_string());
            }
            Ok(round_money(amount))
        }
        _ => Err("نوع العمولة يجب أن يكون نسبة أو مبلغًا ثابتًا.".to_string()),
    }
}

pub fn validate_share_percents(
    office: f64,
    broker: f64,
    marketer: f64,
    partner: f64,
) -> Result<(), String> {
    for (name, value) in [
        ("حصة المكتب", office),
        ("حصة الوسيط", broker),
        ("حصة المسوق", marketer),
        ("حصة المتعاون", partner),
    ] {
        if !value.is_finite() || value < 0.0 || value > 100.0 {
            return Err(format!("{name} يجب أن تكون بين 0 و 100."));
        }
    }
    let sum = office + broker + marketer + partner;
    if (sum - 100.0).abs() > SHARE_EPS {
        return Err("يجب أن يبلغ مجموع الحصص 100%.".to_string());
    }
    Ok(())
}

pub fn share_amount(total: f64, percent: f64) -> f64 {
    round_money(total * percent / 100.0)
}

pub fn net_cash(collections: f64, expenses: f64) -> f64 {
    round_money(collections - expenses)
}

pub fn is_known_mode(mode: &str) -> bool {
    COMMISSION_MODES.contains(&mode)
}

pub fn is_known_expense_category(value: &str) -> bool {
    EXPENSE_CATEGORIES.contains(&value)
}

pub fn is_known_payment_method(value: &str) -> bool {
    PAYMENT_METHODS.contains(&value)
}

pub fn is_known_payment_target(value: &str) -> bool {
    PAYMENT_TARGETS.contains(&value)
}

pub fn next_invoice_number(year: i32, last: Option<&str>) -> String {
    let prefix = format!("F-{year}-");
    let next = last
        .and_then(|n| n.strip_prefix(&prefix))
        .and_then(|rest| rest.parse::<u32>().ok())
        .unwrap_or(0)
        .saturating_add(1);
    format!("{prefix}{next:04}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_commission_and_shares() {
        let total = commission_amount("percent", 1_000_000.0, Some(2.5), None).unwrap();
        assert_eq!(total, 25_000.0);
        validate_share_percents(50.0, 30.0, 10.0, 10.0).unwrap();
        assert_eq!(share_amount(total, 50.0), 12_500.0);
        assert_eq!(share_amount(total, 30.0), 7_500.0);
        assert_eq!(share_amount(total, 10.0), 2_500.0);
    }

    #[test]
    fn fixed_commission_and_invalid_shares() {
        assert_eq!(
            commission_amount("fixed", 0.0, None, Some(8_000.0)).unwrap(),
            8_000.0
        );
        assert!(validate_share_percents(40.0, 40.0, 10.0, 5.0).is_err());
        assert!(commission_amount("percent", 100.0, None, None).is_err());
        assert!(commission_amount("percent", 100.0, Some(-1.0), None).is_err());
    }

    #[test]
    fn status_and_net() {
        assert_eq!(
            payment_status(10_000.0, 10_000.0, Some("2026-01-01"), "2026-09-14"),
            PaymentStatus::Paid
        );
        assert_eq!(
            payment_status(10_000.0, 4_000.0, Some("2026-09-20"), "2026-09-14"),
            PaymentStatus::Partial
        );
        assert_eq!(
            payment_status(10_000.0, 0.0, Some("2026-09-20"), "2026-09-14"),
            PaymentStatus::Due
        );
        assert_eq!(
            payment_status(10_000.0, 1_000.0, Some("2026-09-01"), "2026-09-14"),
            PaymentStatus::Overdue
        );
        assert_eq!(net_cash(15_000.0, 3_500.0), 11_500.0);
        assert_eq!(remaining(10_000.0, 4_000.0), 6_000.0);
    }

    #[test]
    fn invoice_number_increments() {
        assert_eq!(next_invoice_number(2026, None), "F-2026-0001");
        assert_eq!(next_invoice_number(2026, Some("F-2026-0007")), "F-2026-0008");
    }
}
