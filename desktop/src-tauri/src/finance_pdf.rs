//! فواتير وتقارير PDF بهوية المكتب من الإعدادات — دون أرقام ثابتة.

use crate::db::Settings;
use crate::document_locale::{language, text as tr};
use crate::share_pack;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

pub struct GeneratedPdf {
    pub pdf_path: String,
}

/// يولّد ملف PDF فقط. فتح الملف إجراء خارجي منفصل يقرره مستدعي واجهة التطبيق.
pub fn generate_pdf(
    app_data_dir: &Path,
    kind: &str,
    stem: &str,
    html: &str,
) -> Result<GeneratedPdf, String> {
    let dir = app_data_dir.join("finance-docs");
    fs::create_dir_all(&dir).map_err(|e| format!("تعذر إنشاء مجلد المستندات المالية: {e}"))?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let safe: String = stem
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .take(40)
        .collect();
    let base = format!("{kind}-{safe}-{stamp}");
    let html_path = dir.join(format!("{base}.html"));
    let pdf_path = dir.join(format!("{base}.pdf"));
    fs::write(&html_path, html).map_err(|e| format!("تعذر حفظ ملف HTML: {e}"))?;
    let render_result = share_pack::print_html_to_pdf(&html_path, &pdf_path);
    // قد يبقي Chromium مقبض الملف مفتوحًا لجزء قصير من الثانية بعد خروجه.
    for _ in 0..20 {
        match fs::remove_file(&html_path) {
            Ok(()) => break,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => break,
            Err(_) => std::thread::sleep(Duration::from_millis(50)),
        }
    }
    render_result?;
    if !pdf_path.is_file() {
        return Err("تعذر إنشاء ملف PDF.".to_string());
    }
    let pdf_str = pdf_path.to_string_lossy().to_string();
    Ok(GeneratedPdf { pdf_path: pdf_str })
}

pub fn open_pdf_file(path: &Path) -> Result<(), String> {
    if !path.is_file() {
        return Err("ملف PDF غير موجود.".to_string());
    }
    #[cfg(target_os = "windows")]
    {
        // CREATE_NO_WINDOW (0x08000000): يمنع ظهور نافذة cmd السوداء الظرفية عند start.
        use std::os::windows::process::CommandExt;
        Command::new("cmd")
            .args(["/C", "start", "", &path.to_string_lossy()])
            .creation_flags(0x0800_0000)
            .spawn()
            .map_err(|e| format!("تعذر فتح ملف PDF: {e}"))?;
        return Ok(());
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
        Err("فتح PDF غير متاح على هذا النظام.".into())
    }
}

pub fn office_display_name(settings: &Option<Settings>) -> String {
    settings
        .as_ref()
        .and_then(|s| s.office_name.as_ref())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_default()
}

fn esc(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn json_text(value: &Value, key: &str) -> String {
    match value.get(key) {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => {
            if s.trim().is_empty() {
                String::new()
            } else {
                s.clone()
            }
        }
        Some(other) => other.to_string().trim_matches('"').to_string(),
    }
}

fn json_money(value: &Value, key: &str) -> String {
    match value.get(key).and_then(|v| v.as_f64()) {
        Some(n) => format!("{n:.2}"),
        None => String::new(),
    }
}

/// تعريب عرض القيم المخزنة إنجليزيًا في قاعدة البيانات — طبقة العرض فقط دون تعديل المخزّن.
fn display_ar(key: &str, value: &str) -> String {
    match key {
        // طرق الدفع
        "transfer" | "bank transfer" => "تحويل بنكي".to_string(),
        "cash" => "نقد".to_string(),
        "network" | "card" => "شبكة".to_string(),
        "check" | "cheque" => "شيك".to_string(),
        "other" => "أخرى".to_string(),
        // تصنيفات المصروفات
        "marketing" if key == "category" => "تسويق".to_string(),
        "travel" => "انتقالات".to_string(),
        "fees" => "رسوم".to_string(),
        "maintenance" => "صيانة".to_string(),
        _ => value.to_string(),
    }
}

fn detail_row(label: &str, value: String) -> String {
    if value.trim().is_empty() {
        String::new()
    } else {
        format!("<tr><th>{}</th><td>{}</td></tr>", esc(label), esc(&value))
    }
}

fn money_row(label: &str, value: String, currency: &str) -> String {
    if value.trim().is_empty() {
        String::new()
    } else {
        format!("<div>{}: <strong>{}</strong> {}</div>", esc(label), esc(&value), esc(currency))
    }
}

fn logo_html(settings: &Option<Settings>) -> String {
    let raw = settings
        .as_ref()
        .and_then(|s| s.office_logo.as_ref())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    match raw {
        Some(src) if src.starts_with("data:image") => {
            format!(r#"<img class="logo" src="{}" alt="" />"#, esc(&src))
        }
        _ => String::new(),
    }
}

fn office_meta_html(settings: &Option<Settings>) -> String {
    let s = settings.as_ref();
    let mut lines: Vec<String> = Vec::new();
    if let Some(v) = s.and_then(|x| x.office_phone.as_ref()).filter(|v| !v.trim().is_empty()) {
        lines.push(format!("{}: {}", tr("doc.phone", settings), esc(v)));
    }
    if let Some(v) = s.and_then(|x| x.office_whatsapp.as_ref()).filter(|v| !v.trim().is_empty()) {
        lines.push(format!("{}: {}", tr("doc.whatsapp", settings), esc(v)));
    }
    if let Some(v) = s.and_then(|x| x.office_email.as_ref()).filter(|v| !v.trim().is_empty()) {
        lines.push(format!("{}: {}", tr("doc.email", settings), esc(v)));
    }
    if let Some(v) = s.and_then(|x| x.office_address.as_ref()).filter(|v| !v.trim().is_empty()) {
        lines.push(esc(v));
    }
    let fal_num = s.and_then(|x| x.fal_license_number.as_ref()).filter(|v| !v.trim().is_empty());
    let fal_typ_display = s.map(|x| crate::country::license_label(&x.country,
        x.fal_license_type.as_deref().unwrap_or(""), x.broker_license_name.as_deref().unwrap_or(""))).filter(|v| !v.is_empty());
    match (fal_num, fal_typ_display) {
        (Some(n), Some(t)) => lines.push(format!("{}: {} — {}", tr("doc.license", settings), esc(n), esc(&t))),
        (Some(n), None) => lines.push(format!("{}: {}", tr("doc.license", settings), esc(n))),
        (None, Some(t)) => lines.push(format!("{}: {}", tr("doc.license", settings), esc(&t))),
        _ => {}
    }
    if let Some(v) = s.and_then(|x| x.broker_name.as_ref()).filter(|v| !v.trim().is_empty()) {
        lines.push(format!("{}: {}", tr("doc.broker", settings), esc(v)));
    }
    if lines.is_empty() {
        return String::new();
    }
    format!("<div class=\"meta\">{}</div>", lines.join("<br/>"))
}

pub(crate) fn shell(title: &str, settings: &Option<Settings>, body: &str) -> String {
    let office = esc(&office_display_name(settings));
    // حقل فارغ = إخفاء؛ لا قيمة افتراضية ولا نص تجريبي في مستندات العميل.
    let office_heading = if office.is_empty() {
        String::new()
    } else {
        format!("<h1>{office}</h1>")
    };
    format!(
        r#"<!DOCTYPE html>
<html lang="{lang}" dir="{dir}">
<head>
<meta charset="utf-8"/>
<title>{title}</title>
<style>
  @page {{ size: A4; margin: 18mm; }}
  body {{ font-family: "Segoe UI", Tahoma, Arial, sans-serif; color: #1d2740; background: #f7f1e8; margin: 0; }}
  .sheet {{ background: #fffdf8; border: 1px solid #d7cbb8; padding: 24px 28px; }}
  .head {{ display: flex; justify-content: space-between; gap: 16px; border-bottom: 3px solid #23365f; padding-bottom: 14px; margin-bottom: 18px; }}
  .brand {{ color: #23365f; }}
  .brand h1 {{ margin: 0 0 4px; font-size: 22px; }}
  .logo {{ max-height: 64px; max-width: 140px; object-fit: contain; }}
  .meta {{ font-size: 12px; color: #4b5568; line-height: 1.6; }}
  h2 {{ margin: 0 0 12px; font-size: 18px; color: #23365f; }}
  table {{ width: 100%; border-collapse: collapse; margin-top: 10px; font-size: 13px; }}
  th, td {{ border-bottom: 1px solid #eadfce; padding: 8px 6px; text-align: start; }}
  th {{ background: #23365f; color: #f7f1e8; font-weight: 600; }}
  .totals {{ margin-top: 16px; padding: 12px 14px; background: #f7f1e8; border: 1px solid #eadfce; }}
  .muted {{ color: #6b7280; font-size: 12px; }}
  .gold {{ color: #b45309; }}
</style>
</head>
<body>
  <div class="sheet">
    <div class="head">
      <div class="brand">
        {heading}
        {meta}
      </div>
      {logo}
    </div>
    {body}
  </div>
</body>
</html>"#,
        lang = language(settings),
        dir = if language(settings) == "en" { "ltr" } else { "rtl" },
        title = esc(title),
        heading = office_heading,
        meta = office_meta_html(settings),
        logo = logo_html(settings),
        body = body
    )
}

pub fn invoice_html(invoice: &Value, payments: &[Value], settings: &Option<Settings>) -> String {
    let currency = json_text(invoice, "currency");
    let number = json_text(invoice, "number");
    let title = json_text(invoice, "title");
    let rows: String = payments
        .iter()
        .map(|p| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                esc(&json_text(p, "occurs_on")),
                esc(&payment_method(&json_text(p, "method"), settings)),
                esc(&json_text(p, "reference")),
                esc(&format!("{} {}", json_money(p, "amount"), json_text(p, "currency"))),
            )
        })
        .collect();
    let payments_table = if payments.is_empty() {
        format!("<p class=\"muted\">{}</p>", esc(tr("doc.noPayments", settings)))
    } else {
        format!(
            "<table><thead><tr><th>{}</th><th>{}</th><th>{}</th><th>{}</th></tr></thead><tbody>{rows}</tbody></table>",
            esc(tr("doc.date", settings)), esc(tr("doc.method", settings)),
            esc(tr("doc.reference", settings)), esc(tr("doc.amount", settings))
        )
    };
    let details = [
        detail_row(tr("doc.date", settings), json_text(invoice, "issue_date")),
        detail_row(tr("doc.due", settings), json_text(invoice, "due_date")),
        detail_row(tr("doc.client", settings), json_text(invoice, "client_name")),
        detail_row(tr("doc.property", settings), json_text(invoice, "property_name")),
        detail_row(tr("doc.campaign", settings), json_text(invoice, "campaign_name")),
        detail_row(tr("doc.status", settings), invoice_status(invoice, settings)),
        detail_row(tr("doc.notes", settings), json_text(invoice, "notes")),
    ].join("");
    let totals = [
        money_row(tr("doc.amount", settings), json_money(invoice, "amount"), &currency),
        money_row(tr("doc.paid", settings), json_money(invoice, "paid"), &currency),
        money_row(tr("doc.remaining", settings), json_money(invoice, "remaining"), &currency),
    ].join("");
    let body = format!(
        r#"<h2>{invoice_label} {number}</h2>
{title_html}
{details_table}
{totals_html}
<h2>{payments_label}</h2>
{payments_table}"#,
        invoice_label = esc(tr("doc.invoice", settings)),
        payments_label = esc(tr("doc.payments", settings)),
        number = esc(&number),
        title_html = if title.is_empty() { String::new() } else { format!("<p>{}</p>", esc(&title)) },
        details_table = if details.is_empty() { String::new() } else { format!("<table>{details}</table>") },
        totals_html = if totals.is_empty() { String::new() } else { format!("<div class=\"totals\">{totals}</div>") },
        payments_table = payments_table,
    );
    shell(&format!("{} {number}", tr("doc.invoice", settings)), settings, &body)
}

pub fn record_html(kind: &str, record: &Value, settings: &Option<Settings>) -> Result<String, String> {
    let (title, fields): (&str, Vec<(&str, &str, bool)>) = match kind {
        "operation" => (tr("doc.operation", settings), vec![(tr("doc.title", settings), "title", false), (tr("doc.date", settings), "occurs_on", false), (tr("doc.dealValue", settings), "deal_value", true), (tr("doc.client", settings), "client_name", false), (tr("doc.property", settings), "property_name", false), (tr("doc.campaign", settings), "campaign_name", false), (tr("doc.notes", settings), "notes", false)]),
        "commission" => (tr("doc.commission", settings), vec![(tr("doc.title", settings), "title", false), (tr("doc.date", settings), "occurs_on", false), (tr("doc.amount", settings), "computed_amount", true), (tr("doc.basis", settings), "basis_amount", true), (tr("doc.rate", settings), "rate_percent", false), (tr("doc.client", settings), "client_name", false), (tr("doc.property", settings), "property_name", false), (tr("doc.notes", settings), "notes", false)]),
        "expense" => (tr("doc.expense", settings), vec![(tr("doc.title", settings), "title", false), (tr("doc.date", settings), "occurs_on", false), (tr("doc.amount", settings), "amount", true), (tr("doc.category", settings), "category", false), (tr("doc.reference", settings), "reference", false), (tr("doc.property", settings), "property_name", false), (tr("doc.campaign", settings), "campaign_name", false), (tr("doc.notes", settings), "notes", false)]),
        "payment" => (tr("doc.payment", settings), vec![(tr("doc.date", settings), "occurs_on", false), (tr("doc.amount", settings), "amount", true), (tr("doc.linked", settings), "target_title", false), (tr("doc.method", settings), "method", false), (tr("doc.reference", settings), "reference", false), (tr("doc.notes", settings), "notes", false)]),
        "receivable" => (tr("doc.receivable", settings), vec![(tr("doc.statement", settings), "title", false), (tr("doc.kind", settings), "kind_label", false), (tr("doc.status", settings), "status_label", false), (tr("doc.amount", settings), "amount", true), (tr("doc.remaining", settings), "remaining", true), (tr("doc.client", settings), "client_name", false), (tr("doc.property", settings), "property_name", false)]),
        _ => return Err("نوع مستند الطباعة غير معروف.".to_string()),
    };
    let rows = fields.into_iter().map(|(label, key, money)| {
        let raw = if money { let n = json_money(record, key); if n.is_empty() { n } else { format!("{} {}", n, json_text(record, "currency")) } } else { json_text(record, key) };
        let value = match key {
            "method" => payment_method(&raw, settings),
            "status_label" => invoice_status(record, settings),
            "category" => match raw.as_str() {
                "تسويق" | "marketing" => tr("doc.value.marketing",settings).into(),
                "انتقالات" | "travel" => tr("doc.value.travel",settings).into(),
                "رسوم" | "fees" => tr("doc.value.fees",settings).into(),
                "صيانة" | "maintenance" => tr("doc.value.maintenance",settings).into(),
                "أخرى" | "other" => tr("doc.value.other",settings).into(),
                _ => raw,
            },
            _ => raw,
        };
        detail_row(label, value)
    }).collect::<String>();
    let body = format!("<h2>{}</h2>{}", esc(title), if rows.is_empty() { String::new() } else { format!("<table>{rows}</table>") });
    Ok(shell(title, settings, &body))
}

pub fn report_html(summary: &Value, settings: &Option<Settings>, period_from: &str, period_to: &str) -> String {
    let period = match (period_from.trim(), period_to.trim()) {
        ("", "") => tr("doc.allPeriod", settings).to_string(),
        (a, "") => format!("{} {a}", tr("doc.from", settings)),
        ("", b) => format!("{} {b}", tr("doc.until", settings)),
        (a, b) => format!("{} {a} {} {b}", tr("doc.from", settings), tr("doc.to", settings)),
    };
    let body = format!(
        r#"<h2>{label_report}</h2>
<p class="muted">{period}</p>
<div class="totals">
  <div>{label_invoiceCollected}: {inv_paid} {currency} {label_ofTotal} {inv_total} {currency}</div>
  <div>{label_commissionCollected}: {com_paid} {currency} {label_ofTotal} {com_total} {currency}</div>
  <div>{label_expenses}: {exp} {currency}</div>
  <div>{label_receivables}: {recv} {currency}</div>
  <div class="gold">{label_net}: <strong>{net}</strong> {currency}</div>
</div>
<p class="muted">{label_netNote}</p>"#,
        label_report = tr("doc.report", settings),
        label_invoiceCollected = tr("doc.invoiceCollected", settings),
        label_commissionCollected = tr("doc.commissionCollected", settings),
        label_ofTotal = tr("doc.ofTotal", settings),
        label_expenses = tr("doc.expenses", settings),
        label_receivables = tr("doc.receivables", settings),
        label_net = tr("doc.net", settings),
        label_netNote = tr("doc.netNote", settings),
        currency = esc(&json_text(summary, "currency")),
        period = esc(&period),
        inv_paid = esc(&json_money(summary, "invoice_collected")),
        inv_total = esc(&json_money(summary, "invoice_total")),
        com_paid = esc(&json_money(summary, "commission_collected")),
        com_total = esc(&json_money(summary, "commission_total")),
        exp = esc(&json_money(summary, "expense_total")),
        recv = esc(&json_money(summary, "receivables")),
        net = esc(&json_money(summary, "net_cash")),
    );
    shell(tr("doc.report", settings), settings, &body)
}

pub fn pdf_parent_ok(app_data_dir: &Path, file: &str) -> bool {
    PathBuf::from(file)
        .canonicalize()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .and_then(|parent| {
            app_data_dir
                .join("finance-docs")
                .canonicalize()
                .ok()
                .map(|allowed| parent == allowed)
        })
        .unwrap_or(false)
}

fn invoice_status(invoice: &Value, settings: &Option<Settings>) -> String {
    let status = json_text(invoice, "status");
    match status.as_str() {
        "due" | "partial" | "paid" | "overdue" => tr(&format!("doc.value.{status}"), settings).to_string(),
        _ => json_text(invoice, "status_label"),
    }
}

fn payment_method(value: &str, settings: &Option<Settings>) -> String {
    let key = match value {
        "تحويل" | "تحويل بنكي" | "transfer" | "bank transfer" => "transfer",
        "نقد" | "cash" => "cash",
        "شبكة" | "network" | "card" => "card",
        "شيك" | "check" | "cheque" => "cheque",
        "أخرى" | "other" => "other",
        _ => return value.to_string(),
    };
    tr(&format!("doc.value.{key}"), settings).to_string()
}
