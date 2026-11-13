//! مؤشرات الحملات التسويقية — المصدر المعتمد للحسابات.
//! المقام صفر يعني أن الناتج غير قابل للحساب (None)، وليس صفرًا ناجحًا.

use serde_json::{json, Value};

pub const CAMPAIGN_PLATFORMS: &[&str] = &[
    "snapchat",
    "instagram",
    "tiktok",
    "twitter",
    "facebook",
    "youtube",
    "google",
    "haraj",
    "aqar",
    "wasalt",
    "other",
];

#[derive(Debug, Clone, PartialEq)]
pub struct CampaignMetrics {
    pub inquiry_rate: Option<f64>,
    pub viewing_conversion: Option<f64>,
    pub deal_conversion: Option<f64>,
    pub cost_per_inquiry: Option<f64>,
    pub cost_per_viewing: Option<f64>,
    pub cost_per_deal: Option<f64>,
}

pub fn ratio_i64(numerator: i64, denominator: i64) -> Option<f64> {
    if denominator <= 0 {
        None
    } else {
        Some(numerator as f64 / denominator as f64)
    }
}

pub fn ratio_cost(cost: f64, denominator: i64) -> Option<f64> {
    if denominator <= 0 {
        None
    } else {
        Some(cost / denominator as f64)
    }
}

/// معدل الاستفسار = الاستفسارات ÷ المشاهدات
/// معدل التحويل إلى معاينة = المعاينات ÷ الاستفسارات
/// معدل التحويل إلى صفقة = الصفقات ÷ المعاينات
/// تكلفة الاستفسار / المعاينة / الصفقة = التكلفة ÷ العدد المقابل
pub fn compute_campaign_metrics(
    views: i64,
    inquiries: i64,
    viewings: i64,
    deals: i64,
    cost: f64,
) -> CampaignMetrics {
    CampaignMetrics {
        inquiry_rate: ratio_i64(inquiries, views),
        viewing_conversion: ratio_i64(viewings, inquiries),
        deal_conversion: ratio_i64(deals, viewings),
        cost_per_inquiry: ratio_cost(cost, inquiries),
        cost_per_viewing: ratio_cost(cost, viewings),
        cost_per_deal: ratio_cost(cost, deals),
    }
}

pub fn metrics_to_json(m: &CampaignMetrics) -> Value {
    json!({
        "inquiry_rate": m.inquiry_rate,
        "viewing_conversion": m.viewing_conversion,
        "deal_conversion": m.deal_conversion,
        "cost_per_inquiry": m.cost_per_inquiry,
        "cost_per_viewing": m.cost_per_viewing,
        "cost_per_deal": m.cost_per_deal,
    })
}

pub fn validate_hhmm(value: Option<&str>) -> Result<Option<String>, String> {
    match value.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(raw) => {
            let parts: Vec<&str> = raw.split(':').collect();
            if parts.len() != 2 {
                return Err("وقت الموعد يجب أن يكون بصيغة HH:MM.".to_string());
            }
            let hour: i32 = parts[0]
                .parse()
                .map_err(|_| "ساعة الموعد غير صالحة.".to_string())?;
            let minute: i32 = parts[1]
                .parse()
                .map_err(|_| "دقيقة الموعد غير صالحة.".to_string())?;
            if !(0..=23).contains(&hour) || !(0..=59).contains(&minute) || parts[0].len() > 2 || parts[1].len() != 2 {
                return Err("وقت الموعد يجب أن يكون بصيغة HH:MM.".to_string());
            }
            Ok(Some(format!("{hour:02}:{minute:02}")))
        }
    }
}

pub fn is_known_platform(platform: &str) -> bool {
    CAMPAIGN_PLATFORMS.contains(&platform)
}

pub fn parse_iso_date(value: &str) -> Result<chrono::NaiveDate, String> {
    chrono::NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
        .map_err(|_| "صيغة التاريخ غير صحيحة. استخدم YYYY-MM-DD.".to_string())
}

pub fn optional_iso_date(value: Option<&str>) -> Result<Option<String>, String> {
    match value.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(s) => {
            parse_iso_date(s)?;
            Ok(Some(s.to_string()))
        }
    }
}

pub fn validate_ad_link(value: Option<&str>) -> Result<Option<String>, String> {
    match value.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(link) => {
            let lower = link.to_ascii_lowercase();
            if lower.starts_with("javascript:")
                || lower.starts_with("data:")
                || lower.starts_with("file:")
                || lower.contains('\n')
                || lower.contains('\r')
            {
                return Err("رابط الإعلان غير مسموح.".to_string());
            }
            if !(lower.starts_with("https://") || lower.starts_with("http://")) {
                return Err("رابط الإعلان يجب أن يبدأ بـ http:// أو https://.".to_string());
            }
            Ok(Some(link.to_string()))
        }
    }
}

pub fn json_nonneg_i64(data: &Value, key: &str, default: i64) -> Result<i64, String> {
    match data.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(Value::Number(n)) => {
            if let Some(i) = n.as_i64() {
                if i < 0 {
                    return Err(format!("قيمة «{key}» لا يجوز أن تكون سالبة."));
                }
                Ok(i)
            } else if let Some(f) = n.as_f64() {
                if f < 0.0 || (f.fract() - 0.0).abs() > f64::EPSILON {
                    return Err(format!("قيمة «{key}» يجب أن تكون عددًا صحيحًا غير سالب."));
                }
                Ok(f as i64)
            } else {
                Err(format!("قيمة «{key}» غير صالحة."))
            }
        }
        Some(Value::String(s)) => {
            let t = s.trim();
            if t.is_empty() {
                return Ok(default);
            }
            let i: i64 = t
                .parse()
                .map_err(|_| format!("قيمة «{key}» يجب أن تكون عددًا صحيحًا غير سالب."))?;
            if i < 0 {
                return Err(format!("قيمة «{key}» لا يجوز أن تكون سالبة."));
            }
            Ok(i)
        }
        _ => Err(format!("قيمة «{key}» غير صالحة.")),
    }
}

pub fn json_nonneg_f64(data: &Value, key: &str, default: f64) -> Result<f64, String> {
    match data.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(Value::Number(n)) => {
            let f = n
                .as_f64()
                .ok_or_else(|| format!("قيمة «{key}» غير صالحة."))?;
            if f < 0.0 || !f.is_finite() {
                return Err(format!("قيمة «{key}» لا يجوز أن تكون سالبة."));
            }
            Ok(f)
        }
        Some(Value::String(s)) => {
            let t = s.trim();
            if t.is_empty() {
                return Ok(default);
            }
            let f: f64 = t
                .parse()
                .map_err(|_| format!("قيمة «{key}» يجب أن تكون رقمًا غير سالب."))?;
            if f < 0.0 || !f.is_finite() {
                return Err(format!("قيمة «{key}» لا يجوز أن تكون سالبة."));
            }
            Ok(f)
        }
        _ => Err(format!("قيمة «{key}» غير صالحة.")),
    }
}

pub fn json_opt_str(data: &Value, key: &str) -> Option<String> {
    data.get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formulas_match_specified_ratios() {
        let m = compute_campaign_metrics(100, 10, 4, 1, 1000.0);
        assert_eq!(m.inquiry_rate, Some(0.1));
        assert_eq!(m.viewing_conversion, Some(0.4));
        assert_eq!(m.deal_conversion, Some(0.25));
        assert_eq!(m.cost_per_inquiry, Some(100.0));
        assert_eq!(m.cost_per_viewing, Some(250.0));
        assert_eq!(m.cost_per_deal, Some(1000.0));
    }

    #[test]
    fn zero_denominators_are_not_computable() {
        let m = compute_campaign_metrics(0, 0, 0, 0, 500.0);
        assert_eq!(m.inquiry_rate, None);
        assert_eq!(m.viewing_conversion, None);
        assert_eq!(m.deal_conversion, None);
        assert_eq!(m.cost_per_inquiry, None);
        assert_eq!(m.cost_per_viewing, None);
        assert_eq!(m.cost_per_deal, None);
    }
}
